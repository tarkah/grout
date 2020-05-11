use std::mem;
use std::ptr;
use std::thread;

use winapi::um::winuser::{
    DispatchMessageW, GetKeyboardLayout, GetMessageW, RegisterHotKey, TranslateMessage,
    VkKeyScanExW, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN, WM_HOTKEY,
};

use crate::common::report_and_exit;
use crate::Message;
use crate::CHANNEL;

#[derive(PartialEq, Clone, Copy)]
pub enum HotkeyType {
    Main,
    QuickResize,
}

pub fn spawn_hotkey_thread(hotkey_str: &str, hotkey_type: HotkeyType) {
    let mut hotkey: Vec<String> = hotkey_str
        .split('+')
        .map(|s| s.trim().to_string())
        .collect();

    if !(2..6).contains(&hotkey.len()) {
        unsafe {
            report_and_exit(&format!(
                "Invalid hotkey <{}>: Combination must be between 2 to 5 keys long.",
                hotkey_str
            ));
        }
    }

    let virtual_key_char = hotkey.pop().unwrap().chars().next().unwrap();

    let hotkey_str = hotkey_str.to_owned();
    thread::spawn(move || unsafe {
        let sender = &CHANNEL.0.clone();

        let result = RegisterHotKey(
            ptr::null_mut(),
            0,
            compile_modifiers(&hotkey, &hotkey_str) | MOD_NOREPEAT as u32,
            get_vkcode(virtual_key_char),
        );

        if result == 0 {
            report_and_exit(&format!("Failed to assign hot key <{}>. Either program is already running or hotkey is already assigned in another program.", hotkey_str));
        }

        let mut msg = mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);

            if msg.message == WM_HOTKEY {
                let _ = sender.send(Message::HotkeyPressed(hotkey_type));
            }
        }
    });
}

fn compile_modifiers(activators: &[String], hotkey_str: &str) -> u32 {
    let mut code: u32 = 0;
    for key in activators {
        match key.as_str() {
            "ALT" => code |= MOD_ALT as u32,
            "CTRL" => code |= MOD_CONTROL as u32,
            "SHIFT" => code |= MOD_SHIFT as u32,
            "WIN" => code |= MOD_WIN as u32,

            _ => unsafe {
                report_and_exit(&format!("Invalid hotkey <{}>: Unidentified modifier in hotkey combination. Valid modifiers are CTRL, ALT, SHIFT, WIN.", hotkey_str))
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
