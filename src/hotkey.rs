use std::mem;
use std::ptr;
use std::thread;

use winapi::um::winuser::{
    DispatchMessageW, GetKeyboardLayout, GetMessageW, RegisterHotKey, TranslateMessage,
    VkKeyScanExW, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, WM_HOTKEY,
};

use crate::common::report_and_exit;
use crate::config;
use crate::Message;
use crate::CHANNEL;

pub fn spawn_hotkey_thread() {
    let mut hotkey: Vec<String> = config::load_config()
        .hotkey
        .split('+')
        .map(|s| s.trim().to_string())
        .collect();

    if !(2..5).contains(&hotkey.len()) {
        unsafe {
            report_and_exit("Invalid hotkey: Combination must be between 2 to 4 keys long.");
        }
    }

    let virtual_key_char = hotkey.pop().unwrap().chars().next().unwrap();

    thread::spawn(move || unsafe {
        let sender = &CHANNEL.0.clone();

        let result = RegisterHotKey(
            ptr::null_mut(),
            0,
            compile_modifiers(&hotkey) | MOD_NOREPEAT as u32,
            get_vkcode(virtual_key_char),
        );

        if result == 0 {
            report_and_exit("Failed to assign hot key. Either program is already running or hotkey is already assigned in another program.");
        }

        let mut msg = mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);

            if msg.message == WM_HOTKEY {
                let _ = sender.send(Message::HotkeyPressed);
            }
        }
    });
}

fn compile_modifiers(activators: &[String]) -> u32 {
    let mut code: u32 = 0;
    for key in activators {
        match key.as_str() {
            "ALT" => code |= MOD_ALT as u32,
            "CTRL" => code |= MOD_CONTROL as u32,
            "SHIFT" => code |= MOD_SHIFT as u32,
            "WIN" => code |= MOD_WIN as u32,

            _ => unsafe {
                report_and_exit("Invalid hotkey: Unidentified modifier in hotkey combination. Valid modifiers are CTRL, ALT, SHIFT, WIN.")
            },
        }
    }
    code
}

unsafe fn get_vkcode(key_char: char) -> u32 {
    let keyboard_layout = GetKeyboardLayout(0);
    let vk_code = VkKeyScanExW(key_char as u16, keyboard_layout).to_be_bytes();

    vk_code[1] as u32
}
