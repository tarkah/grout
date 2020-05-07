use std::mem;
use std::process;
use std::ptr;
use std::thread;

use winapi::um::winuser::{
    DispatchMessageW, GetMessageW, MessageBoxW, RegisterHotKey, TranslateMessage, MB_OK, MOD_ALT,
    MOD_CONTROL, MOD_NOREPEAT, WM_HOTKEY,
};

use crate::Message;
use crate::CHANNEL;

pub fn spawn_hotkey_thread() {
    thread::spawn(move || unsafe {
        let sender = &CHANNEL.0.clone();

        let result = RegisterHotKey(
            ptr::null_mut(),
            0,
            MOD_CONTROL as u32 | MOD_ALT as u32 | MOD_NOREPEAT as u32,
            0x53, // S
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
