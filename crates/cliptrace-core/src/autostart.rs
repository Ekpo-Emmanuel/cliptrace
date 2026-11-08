use std::path::Path;

use thiserror::Error;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{
    HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
};
use windows::core::{PCWSTR, w};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE_NAME: PCWSTR = w!("Cliptrace");

#[derive(Debug, Error)]
pub enum AutostartError {
    #[error("registry error {0}")]
    Registry(u32),
    #[error("app path is not valid unicode")]
    Path,
}

pub fn set_autostart(enabled: bool, exe: &Path) -> Result<(), AutostartError> {
    let mut key = HKEY::default();
    // Safety: RUN_KEY is a static UTF-16 string; key out-pointer is valid.
    let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, 0, KEY_SET_VALUE, &mut key) };
    if status.is_err() {
        return Err(AutostartError::Registry(status.0));
    }
    let result = if enabled {
        let wide = path_to_wide(exe)?;
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2)
        };
        // Safety: key opened above; value is NUL-terminated UTF-16 bytes.
        let status = unsafe { RegSetValueExW(key, VALUE_NAME, 0, REG_SZ, Some(bytes)) };
        if status.is_err() {
            Err(AutostartError::Registry(status.0))
        } else {
            Ok(())
        }
    } else {
        // Safety: key opened above.
        let status = unsafe { RegDeleteValueW(key, VALUE_NAME) };
        if status.is_err() && status != ERROR_FILE_NOT_FOUND {
            Err(AutostartError::Registry(status.0))
        } else {
            Ok(())
        }
    };
    result
}

fn path_to_wide(path: &Path) -> Result<Vec<u16>, AutostartError> {
    let s = path.to_str().ok_or(AutostartError::Path)?;
    let quoted = format!("\"{s}\"");
    let mut wide: Vec<u16> = quoted.encode_utf16().collect();
    wide.push(0);
    Ok(wide)
}
