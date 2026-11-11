use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::path::PathBuf;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 对齐 Antigravity v2.31.0 `languageServer.js` 的看门狗参数并预留安全余量：
/// 官方在 60s 内连续崩溃 >= 3 次会触发熔断放弃拉起（Giving up），每次冷却 2000ms。
pub const LS_COOLDOWN_GUARD: Duration = Duration::from_millis(2500);
pub const LS_WINDOW_RESET: Duration = Duration::from_secs(65);
pub const MAX_SAFE_HOT_RELOADS: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotReloadCheck {
    Allowed,
    InCooldown(u64),
    CircuitBreakerReached,
}

#[derive(Debug, Default)]
pub struct WatchdogTracker {
    last_hot_reload: Option<Instant>,
    consecutive_count: u32,
}

impl WatchdogTracker {
    /// 评估当前时刻是否允许安全触发热重载；使用时间戳判断是否已越过官方 60s 复位窗口。
    pub fn evaluate(&mut self, now: Instant) -> HotReloadCheck {
        if let Some(last) = self.last_hot_reload {
            let elapsed = now.saturating_duration_since(last);
            if elapsed < LS_COOLDOWN_GUARD {
                let remain = (LS_COOLDOWN_GUARD - elapsed).as_millis() as u64;
                return HotReloadCheck::InCooldown(remain.max(1));
            }
            if elapsed > LS_WINDOW_RESET {
                self.consecutive_count = 0;
            }
        }

        if self.consecutive_count >= MAX_SAFE_HOT_RELOADS {
            HotReloadCheck::CircuitBreakerReached
        } else {
            HotReloadCheck::Allowed
        }
    }

    pub fn record_success(&mut self, now: Instant) {
        self.last_hot_reload = Some(now);
        self.consecutive_count = self.consecutive_count.saturating_add(1);
    }

    pub fn reset(&mut self) {
        self.last_hot_reload = None;
        self.consecutive_count = 0;
    }

    pub fn active_consecutive_count(&self, now: Instant) -> u32 {
        match self.last_hot_reload {
            Some(last) if now.saturating_duration_since(last) <= LS_WINDOW_RESET => {
                self.consecutive_count
            }
            _ => 0,
        }
    }
}

static WATCHDOG_TRACKER: OnceLock<Mutex<WatchdogTracker>> = OnceLock::new();

fn tracker() -> &'static Mutex<WatchdogTracker> {
    WATCHDOG_TRACKER.get_or_init(|| Mutex::new(WatchdogTracker::default()))
}

pub fn check_hot_reload_guard() -> HotReloadCheck {
    if !is_antigravity_running() {
        reset_watchdog_tracker();
        return HotReloadCheck::Allowed;
    }
    let mut guard = tracker().lock().unwrap_or_else(|e| e.into_inner());
    guard.evaluate(Instant::now())
}

pub fn record_hot_reload_triggered() {
    let mut guard = tracker().lock().unwrap_or_else(|e| e.into_inner());
    guard.record_success(Instant::now());
}

pub fn reset_watchdog_tracker() {
    let mut guard = tracker().lock().unwrap_or_else(|e| e.into_inner());
    guard.reset();
}

pub fn get_consecutive_hot_reloads() -> u32 {
    if !is_antigravity_running() {
        return 0;
    }
    let guard = tracker().lock().unwrap_or_else(|e| e.into_inner());
    guard.active_consecutive_count(Instant::now())
}

/// 轮询等待旧进程彻底退出，避免新版单实例锁（requestSingleInstanceLock）在冷重启时发生抢锁闪退。
fn wait_until_stopped(max_wait: Duration) {
    let start = Instant::now();
    while start.elapsed() < max_wait {
        if !is_antigravity_running() {
            // 额外等待 OS 回收命名管道与单实例文件锁
            std::thread::sleep(Duration::from_millis(250));
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(target_os = "windows")]
pub fn is_antigravity_running() -> bool {
    let output = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq Antigravity.exe", "/NH"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        text.contains("Antigravity.exe")
    } else {
        false
    }
}

#[cfg(target_os = "windows")]
pub fn kill_antigravity() -> Result<(), String> {
    let _ = Command::new("taskkill")
        .args(["/F", "/IM", "Antigravity.exe", "/T"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let _ = Command::new("taskkill")
        .args(["/F", "/IM", "language_server.exe", "/T"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    wait_until_stopped(Duration::from_secs(5));
    reset_watchdog_tracker();
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn force_kill_language_server_only() -> Result<(), String> {
    // 仅精准终止后台核心语言服务器，Electron 窗口保持存活并通过 Watchdog 自动重拉起新实例
    let _ = Command::new("taskkill")
        .args(["/F", "/IM", "language_server.exe", "/T"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    std::thread::sleep(Duration::from_millis(150));
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn get_antigravity_exe_path() -> Option<PathBuf> {
    if let Some(local_app_data) = dirs::data_local_dir() {
        let p = local_app_data
            .join("Programs")
            .join("antigravity")
            .join("Antigravity.exe");
        if p.exists() {
            return Some(p);
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub fn start_antigravity() -> Result<(), String> {
    if let Some(exe_path) = get_antigravity_exe_path() {
        Command::new(exe_path)
            .spawn()
            .map_err(|e| format!("启动 Antigravity 失败: {}", e))?;
        Ok(())
    } else {
        Err("未能定位到 Antigravity.exe 的安装路径".into())
    }
}

// ------------------- macOS 平台实现 -------------------

#[cfg(target_os = "macos")]
pub fn is_antigravity_running() -> bool {
    let output = Command::new("pgrep")
        .args(["-x", "Antigravity"])
        .output();

    if let Ok(out) = output {
        out.status.success()
    } else {
        false
    }
}

#[cfg(target_os = "macos")]
pub fn kill_antigravity() -> Result<(), String> {
    let _ = Command::new("killall").arg("Antigravity").output();
    let _ = force_kill_language_server_only();
    wait_until_stopped(Duration::from_secs(5));
    reset_watchdog_tracker();
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn force_kill_language_server_only() -> Result<(), String> {
    // 精确匹配位于资源目录 `/bin/language_server` 的可执行文件路径，避免 `pkill -f language_server` 误伤无关终端命令
    let _ = Command::new("pkill")
        .args(["-f", "/bin/language_server(\\s|$)"])
        .output();
    std::thread::sleep(Duration::from_millis(150));
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn start_antigravity() -> Result<(), String> {
    Command::new("open")
        .args(["-a", "Antigravity"])
        .spawn()
        .map_err(|e| format!("启动 macOS 版 Antigravity 失败: {}", e))?;
    Ok(())
}

/// 受看门狗保护的手动热重载入口
pub fn hot_reload_language_server() -> Result<(), String> {
    if !is_antigravity_running() {
        return Err("Antigravity 桌面端当前未在运行，无需热重载".into());
    }

    match check_hot_reload_guard() {
        HotReloadCheck::InCooldown(remain_ms) => {
            let secs = (remain_ms as f64 / 1000.0).ceil() as u64;
            Err(format!(
                "语言服务器正在官方 2 秒重连冷却中，请 {} 秒后再试",
                secs.max(1)
            ))
        }
        HotReloadCheck::CircuitBreakerReached => Err(
            "60 秒内已连续热重载 2 次，继续热重载可能触发官方客户端看门狗熔断；请等待约 1 分钟或点击底部「冷重启」重置"
                .into(),
        ),
        HotReloadCheck::Allowed => {
            force_kill_language_server_only()?;
            record_hot_reload_triggered();
            Ok(())
        }
    }
}

pub fn restart_antigravity() -> Result<(), String> {
    kill_antigravity()?;
    start_antigravity()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_tracker_respects_cooldown_limit_and_reset_window() {
        let mut t = WatchdogTracker::default();
        let t0 = Instant::now();

        // 第 1 次允许
        assert_eq!(t.evaluate(t0), HotReloadCheck::Allowed);
        t.record_success(t0);
        assert_eq!(t.active_consecutive_count(t0), 1);

        // 冷却期内拦截（1秒 < 2.5秒）
        let t1 = t0 + Duration::from_secs(1);
        assert!(matches!(t.evaluate(t1), HotReloadCheck::InCooldown(_)));

        // 冷却期后允许第 2 次
        let t2 = t0 + Duration::from_secs(3);
        assert_eq!(t.evaluate(t2), HotReloadCheck::Allowed);
        t.record_success(t2);
        assert_eq!(t.active_consecutive_count(t2), 2);

        // 第 3 次若间隔不到 65 秒，触发防熔断保护
        let t3 = t2 + Duration::from_secs(10);
        assert_eq!(t.evaluate(t3), HotReloadCheck::CircuitBreakerReached);

        // 相邻间隔超过 65 秒（官方 60s 复位窗口已过），自动重置为允许
        let t4 = t2 + Duration::from_secs(66);
        assert_eq!(t.active_consecutive_count(t4), 0);
        assert_eq!(t.evaluate(t4), HotReloadCheck::Allowed);

        // 主动冷重启复位
        t.record_success(t4);
        assert_eq!(t.active_consecutive_count(t4), 1);
        t.reset();
        assert_eq!(t.active_consecutive_count(t4), 0);
    }
}
