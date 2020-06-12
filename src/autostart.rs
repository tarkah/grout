use std::env;
use std::fs;
use std::mem;
use std::ptr;

use anyhow::format_err;

use winapi::shared::minwindef::HKEY;
use winapi::um::winnt::{KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ};
use winapi::um::winreg::{RegCreateKeyExW, RegDeleteKeyValueW, RegSetValueExW, HKEY_CURRENT_USER};

use crate::{str_to_wide, Result};

pub unsafe fn toggle_autostart_registry_key(enabled: bool) -> Result<()> {
    let mut app_path =
        dirs::config_dir().ok_or_else(|| format_err!("Failed to get config directory"))?;
    app_path.push("grout");
    app_path.push("grout.exe");

    let current_path = env::current_exe()?;
    if current_path != app_path && enabled {
        fs::copy(current_path, &app_path)?;
    }

    let app_path = str_to_wide!(app_path.to_str().unwrap_or_default());
    let mut key_name = str_to_wide!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    let mut value_name = str_to_wide!("grout");

    let mut key: HKEY = mem::zeroed();

    if enabled {
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_name.as_mut_ptr(),
            0,
            ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            ptr::null_mut(),
            &mut key,
            ptr::null_mut(),
        ) == 0
        {
            RegSetValueExW(
                key,
                value_name.as_mut_ptr(),
                0,
                REG_SZ,
                app_path.as_ptr() as _,
                app_path.len() as u32 * 2,
            );
        }
    } else {
        RegDeleteKeyValueW(
            HKEY_CURRENT_USER,
            key_name.as_mut_ptr(),
            value_name.as_mut_ptr(),
        );
    }

    Ok(())
}
