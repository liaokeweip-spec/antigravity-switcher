use crate::credential::{read_credential, write_credential, DEFAULT_TARGET};
use crate::crypto::{protect, unprotect};
use crate::models::{AccountItem, AccountVault, AppStateDto, SwitchResultDto};
use crate::process::{
    check_hot_reload_guard, force_kill_language_server_only, get_consecutive_hot_reloads,
    is_antigravity_running, kill_antigravity, record_hot_reload_triggered, start_antigravity,
    HotReloadCheck, MAX_SAFE_HOT_RELOADS,
};
use chrono::Local;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

fn get_vault_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let gemini_dir = home.join(".gemini");
    let _ = fs::create_dir_all(&gemini_dir);
    gemini_dir.join("antigravity_accounts.vault")
}

fn get_user_scope_path() -> Option<PathBuf> {
    dirs::data_dir().map(|appdata| appdata.join("Antigravity").join("credential-user-scope"))
}

fn get_google_accounts_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".gemini").join("google_accounts.json"))
}

pub fn load_vault() -> AccountVault {
    let path = get_vault_path();
    if !path.exists() {
        return AccountVault::default();
    }
    if let Ok(encrypted) = fs::read(&path) {
        if let Ok(decrypted) = unprotect(&encrypted) {
            if let Ok(vault) = serde_json::from_slice::<AccountVault>(&decrypted) {
                return vault;
            }
        }
    }
    AccountVault::default()
}

pub fn save_vault(vault: &AccountVault) -> Result<(), String> {
    let path = get_vault_path();
    let json_bytes = serde_json::to_vec(vault).map_err(|e| e.to_string())?;
    let encrypted = protect(&json_bytes)?;
    fs::write(&path, encrypted).map_err(|e| format!("保存账号数据库失败: {}", e))
}

pub fn extract_email_from_cred(secret_json: &str) -> Option<String> {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(secret_json) {
        if let Some(email) = val.get("id_token_email").and_then(|v| v.as_str()) {
            if !email.is_empty() {
                return Some(email.to_string());
            }
        }
        if let Some(email) = val.pointer("/token/email").and_then(|v| v.as_str()) {
            if !email.is_empty() {
                return Some(email.to_string());
            }
        }
    }
    None
}

pub fn get_current_user_scope() -> Option<String> {
    if let Some(scope_path) = get_user_scope_path() {
        if scope_path.exists() {
            if let Ok(content) = fs::read_to_string(&scope_path) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

pub fn get_app_state() -> AppStateDto {
    let current_cred = read_credential(DEFAULT_TARGET).ok();
    let current_email = current_cred.as_deref().and_then(extract_email_from_cred);
    let current_scope = get_current_user_scope();
    let is_running = is_antigravity_running();
    let consecutive_hot_reloads = get_consecutive_hot_reloads();

    let vault = load_vault();
    let accounts: Vec<AccountItem> = vault
        .accounts
        .into_iter()
        .map(|mut acc| {
            let match_email = current_email.as_ref().map_or(false, |e| e == &acc.email);
            let match_scope = current_scope.as_ref().map_or(false, |s| s == &acc.user_scope);
            acc.is_active = match_email || match_scope;
            acc
        })
        .collect();

    AppStateDto {
        current_email,
        current_user_scope: current_scope,
        is_antigravity_running: is_running,
        accounts,
        consecutive_hot_reloads,
        max_safe_hot_reloads: MAX_SAFE_HOT_RELOADS,
    }
}

pub fn snapshot_current_account(name: String) -> Result<(), String> {
    let cred_json = read_credential(DEFAULT_TARGET)
        .map_err(|_| "当前系统凭据管理器中未找到有效的 Antigravity 登录凭据，请先在桌面端登录".to_string())?;

    let email = extract_email_from_cred(&cred_json)
        .unwrap_or_else(|| "unknown@gmail.com".to_string());

    let user_scope = get_current_user_scope().unwrap_or_default();
    let now = Local::now().format("%Y-%m-%d %H:%M").to_string();

    let mut vault = load_vault();

    if let Some(existing) = vault.accounts.iter_mut().find(|a| a.email == email) {
        existing.name = name;
        existing.credential_blob = cred_json;
        existing.user_scope = user_scope;
        existing.created_at = now;
    } else {
        vault.accounts.push(AccountItem {
            id: Uuid::new_v4().to_string(),
            name,
            email,
            user_scope,
            credential_blob: cred_json,
            is_active: true,
            created_at: now,
        });
    }

    save_vault(&vault)
}

/// 将目标账号凭据与辅助文件统一写入操作系统（提取为单一原子路径）。
fn apply_account_credentials(target_account: &AccountItem) -> Result<(), String> {
    write_credential(DEFAULT_TARGET, "antigravity", &target_account.credential_blob)?;

    if let Some(scope_path) = get_user_scope_path() {
        if !target_account.user_scope.is_empty() {
            let _ = fs::write(&scope_path, &target_account.user_scope);
        }
    }

    if let Some(accounts_path) = get_google_accounts_path() {
        if accounts_path.exists() {
            if let Ok(content) = fs::read_to_string(&accounts_path) {
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = val.as_object_mut() {
                        obj.insert(
                            "active".to_string(),
                            serde_json::Value::String(target_account.email.clone()),
                        );
                        let _ = fs::write(
                            &accounts_path,
                            serde_json::to_string_pretty(&val).unwrap_or_default(),
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn switch_account(account_id: String, hot_reload: bool) -> Result<SwitchResultDto, String> {
    let vault = load_vault();
    let target_account = vault
        .accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| "未找到目标账号".to_string())?;

    if !is_antigravity_running() {
        apply_account_credentials(target_account)?;
        start_antigravity()?;
        return Ok(SwitchResultDto {
            mode: "cold_start".into(),
            message: "✅ 凭据已切换并启动 Antigravity 桌面端。".into(),
        });
    }

    if !hot_reload {
        // 先杀旧进程并等锁释放，再写新凭据，避免旧进程退出回写覆盖
        kill_antigravity()?;
        apply_account_credentials(target_account)?;
        start_antigravity()?;
        return Ok(SwitchResultDto {
            mode: "cold_restart".into(),
            message: "✅ 已完成凭据替换并冷重启 Antigravity 桌面端。".into(),
        });
    }

    match check_hot_reload_guard() {
        HotReloadCheck::InCooldown(remain_ms) => {
            let secs = (remain_ms as f64 / 1000.0).ceil() as u64;
            Err(format!(
                "语言服务器正在官方 2 秒重连冷却中，请 {} 秒后再切号（避免触发状态错乱）",
                secs.max(1)
            ))
        }
        HotReloadCheck::CircuitBreakerReached => {
            // 若 65 秒间隔内连续热切已达安全上限，自动升级为冷重启重置看门狗计数器，保证不断连
            kill_antigravity()?;
            apply_account_credentials(target_account)?;
            start_antigravity()?;
            Ok(SwitchResultDto {
                mode: "fallback_cold_restart".into(),
                message: "🛡️ 为避免触发官方 60 秒连续崩溃熔断保护，已自动升级为安全冷重启并重置看门狗计数！".into(),
            })
        }
        HotReloadCheck::Allowed => {
            // Write-before-Kill：凭据落盘后再终止后台 LS，Electron 看门狗约 2 秒后自动拉起新 LS
            apply_account_credentials(target_account)?;
            force_kill_language_server_only()?;
            record_hot_reload_triggered();
            Ok(SwitchResultDto {
                mode: "hot_reload".into(),
                message: "⚡ 无感热切换成功！桌面端将在约 2 秒冷却后自动重连新账号，窗口无需重启。".into(),
            })
        }
    }
}

pub fn delete_account(account_id: String) -> Result<(), String> {
    let mut vault = load_vault();
    vault.accounts.retain(|a| a.id != account_id);
    save_vault(&vault)
}
