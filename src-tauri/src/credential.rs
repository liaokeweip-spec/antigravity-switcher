pub const DEFAULT_TARGET: &str = "gemini:antigravity";

#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::ptr::null_mut;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::FILETIME;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::Credentials::{
    CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

#[cfg(target_os = "windows")]
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

#[cfg(target_os = "windows")]
pub fn read_credential(target: &str) -> Result<String, String> {
    unsafe {
        let wide_target = to_wide(target);
        let mut cred_ptr: *mut CREDENTIALW = null_mut();

        let ok = CredReadW(wide_target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut cred_ptr);
        if ok != 0 && !cred_ptr.is_null() {
            let blob_size = (*cred_ptr).CredentialBlobSize as usize;
            let blob_slice = std::slice::from_raw_parts((*cred_ptr).CredentialBlob, blob_size);
            let utf8_str = String::from_utf8_lossy(blob_slice).to_string();
            CredFree(cred_ptr as _);
            Ok(utf8_str)
        } else {
            Err(format!("无法读取 Windows 凭据 '{}' 或条目不存在", target))
        }
    }
}

#[cfg(target_os = "windows")]
pub fn write_credential(target: &str, user_name: &str, secret_json: &str) -> Result<(), String> {
    unsafe {
        let mut wide_target = to_wide(target);
        let mut wide_user = to_wide(user_name);
        let mut wide_comment = to_wide("Managed by Antigravity Switcher");
        let mut blob_bytes = secret_json.as_bytes().to_vec();

        let cred = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: wide_target.as_mut_ptr(),
            Comment: wide_comment.as_mut_ptr(),
            LastWritten: FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            },
            CredentialBlobSize: blob_bytes.len() as u32,
            CredentialBlob: blob_bytes.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: null_mut(),
            TargetAlias: null_mut(),
            UserName: wide_user.as_mut_ptr(),
        };

        let ok = CredWriteW(&cred, 0);
        if ok != 0 {
            Ok(())
        } else {
            Err(format!("写入 Windows 凭据 '{}' 失败", target))
        }
    }
}

// macOS Keychain 实现：通过 macOS 原生 security CLI 操作钥匙串 (Service: gemini, Account: antigravity)
#[cfg(target_os = "macos")]
pub fn read_credential(_target: &str) -> Result<String, String> {
    let output = std::process::Command::new("security")
        .args(["find-generic-password", "-a", "antigravity", "-s", "gemini", "-w"])
        .output()
        .map_err(|e| format!("调用 macOS security 失败: {}", e))?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !text.is_empty() {
            return Ok(text);
        }
    }
    Err("macOS 钥匙串中未找到 gemini:antigravity 凭据".into())
}

#[cfg(target_os = "macos")]
pub fn write_credential(_target: &str, _user_name: &str, secret_json: &str) -> Result<(), String> {
    let output = std::process::Command::new("security")
        .args([
            "add-generic-password",
            "-U",
            "-a",
            "antigravity",
            "-s",
            "gemini",
            "-w",
            secret_json,
        ])
        .output()
        .map_err(|e| format!("调用 macOS security 失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "写入 macOS 钥匙串失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
