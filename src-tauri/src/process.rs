use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::path::PathBuf;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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

    std::thread::sleep(std::time::Duration::from_millis(300));
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn get_antigravity_exe_path() -> Option<PathBuf> {
    if let Some(local_app_data) = dirs::data_local_dir() {
        let p = local_app_data.join("Programs").join("antigravity").join("Antigravity.exe");
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
    let _ = Command::new("pkill").args(["-f", "language_server"]).output();
    std::thread::sleep(std::time::Duration::from_millis(300));
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

pub fn restart_antigravity() -> Result<(), String> {
    kill_antigravity()?;
    start_antigravity()
}
