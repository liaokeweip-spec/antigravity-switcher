#[cfg(target_os = "windows")]
use std::ptr::null_mut;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::LocalFree;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
};

#[cfg(target_os = "windows")]
pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let mut in_data = data.to_vec();
        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: in_data.len() as u32,
            pbData: in_data.as_mut_ptr(),
        };
        let mut out_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: null_mut(),
        };

        let ok = CryptProtectData(
            &in_blob,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            0,
            &mut out_blob,
        );

        if ok != 0 {
            let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
            let result = slice.to_vec();
            LocalFree(out_blob.pbData as _);
            Ok(result)
        } else {
            Err("Windows DPAPI CryptProtectData failed".into())
        }
    }
}

#[cfg(target_os = "windows")]
pub fn unprotect(encrypted_data: &[u8]) -> Result<Vec<u8>, String> {
    unsafe {
        let mut in_data = encrypted_data.to_vec();
        let in_blob = CRYPT_INTEGER_BLOB {
            cbData: in_data.len() as u32,
            pbData: in_data.as_mut_ptr(),
        };
        let mut out_blob = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: null_mut(),
        };

        let ok = CryptUnprotectData(
            &in_blob,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            0,
            &mut out_blob,
        );

        if ok != 0 {
            let slice = std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize);
            let result = slice.to_vec();
            LocalFree(out_blob.pbData as _);
            Ok(result)
        } else {
            Err("Windows DPAPI CryptUnprotectData failed".into())
        }
    }
}

// macOS 平台实现（macOS 用户目录受 FileVault 保护与系统权限隔离）
#[cfg(not(target_os = "windows"))]
pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    Ok(data.to_vec())
}

#[cfg(not(target_os = "windows"))]
pub fn unprotect(encrypted_data: &[u8]) -> Result<Vec<u8>, String> {
    Ok(encrypted_data.to_vec())
}
