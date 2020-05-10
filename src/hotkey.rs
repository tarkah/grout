use std::mem;
use std::process;
use std::ptr;
use std::thread;

use winapi::um::winuser::{
    DispatchMessageW, GetMessageW, MessageBoxW, RegisterHotKey, TranslateMessage,
    GetKeyboardLayout, VkKeyScanExA, MB_OK, MOD_ALT, MOD_CONTROL, MOD_SHIFT, 
    MOD_NOREPEAT, WM_HOTKEY,
};

use crate::Message;
use crate::CHANNEL;

use crate::config;

pub fn spawn_hotkey_thread() {
    let mut activators: Vec<String> = vec!();
    let virtual_key_char: char;
    
    let mut hotkey: Vec<String> = config::load_config().hotkey
                                .split("+")
                                .map(|s| s.trim().to_string())
                                .collect();

    if hotkey.len() != 3 {
        panic!("Invalid hotkey combination.");
    }
    
    virtual_key_char = hotkey.pop().unwrap()
                        .chars().next().unwrap();
    
    activators.push(hotkey.pop().unwrap());
    activators.push(hotkey.pop().unwrap());

    thread::spawn(move || unsafe {
        let sender = &CHANNEL.0.clone();

        let result = RegisterHotKey(
            ptr::null_mut(),
            0,
            compile_activators(&activators) | MOD_NOREPEAT as u32,
            get_vkcode(virtual_key_char),
        );

        if result == 0 {
            let error_msg = "Failed to assign hot key. Is program already running?";
            let mut error_msg = error_msg.encode_utf16().collect::<Vec<_>>();
            error_msg.push(0);

            MessageBoxW(
                ptr::null_mut(),
                error_msg.as_mut_ptr(),
                ptr::null_mut(),
                MB_OK,
            );

            process::exit(1);
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

fn compile_activators(activators: &Vec<String>) -> u32 {
    let mut code: u32 = 0;
    for key in activators.iter() {
        match key.as_str() {
            "ALT"   => code = code | MOD_ALT as u32,
            "CTRL"  => code = code | MOD_CONTROL as u32,
            "SHIFT" => code = code | MOD_SHIFT as u32,
            &_      => panic!("Invalid hotkey combination."),
        }
    }
    code
}

unsafe fn get_vkcode(key_char: char) -> u32 {
    let keyboard_layout = GetKeyboardLayout(0);
    let vk_code = VkKeyScanExA(key_char as i8, keyboard_layout).to_be_bytes();
    
    vk_code[1] as u32
}
