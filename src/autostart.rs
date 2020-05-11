use std::env;
use std::fs;
use std::mem;
use std::ptr;

use winapi::shared::minwindef::HKEY;
use winapi::um::winnt::{KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ};
use winapi::um::winreg::{RegCreateKeyExW, RegDeleteKeyValueW, RegSetValueExW, HKEY_CURRENT_USER};

pub unsafe fn toggle_autostart_registry_key(enabled: bool) {
    if let Some(mut app_path) = dirs::config_dir() {
        app_path.push("grout");
        app_path.push("grout.exe");

        if let Ok(current_path) = env::current_exe() {
            if current_path != app_path && enabled {
                let _ = fs::copy(current_path, &app_path);
            }

            let app_path = app_path.to_str().unwrap_or_default();
            let mut app_path = app_path.encode_utf16().collect::<Vec<_>>();
            app_path.push(0);

            let key_name = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
            let mut key_name = key_name.encode_utf16().collect::<Vec<_>>();
            key_name.push(0);

            let value_name = "grout";
            let mut value_name = value_name.encode_utf16().collect::<Vec<_>>();
            value_name.push(0);

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
                };
            } else {
                RegDeleteKeyValueW(
                    HKEY_CURRENT_USER,
                    key_name.as_mut_ptr(),
                    value_name.as_mut_ptr(),
                );
            }
        }
    }
}
