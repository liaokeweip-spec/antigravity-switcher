use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountItem {
    pub id: String,
    pub name: String,
    pub email: String,
    pub user_scope: String,
    pub credential_blob: String,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountVault {
    pub accounts: Vec<AccountItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateDto {
    pub current_email: Option<String>,
    pub current_user_scope: Option<String>,
    pub is_antigravity_running: bool,
    pub accounts: Vec<AccountItem>,
    pub consecutive_hot_reloads: u32,
    pub max_safe_hot_reloads: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchResultDto {
    /// "hot_reload" | "fallback_cold_restart" | "cold_restart" | "cold_start"
    pub mode: String,
    pub message: String,
}
