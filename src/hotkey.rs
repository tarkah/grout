use std::mem;
use std::ptr;
use std::thread;

use winapi::um::winuser::{
    DispatchMessageW, GetMessageW, RegisterHotKey, TranslateMessage, MOD_ALT, MOD_CONTROL,
    MOD_NOREPEAT, WM_HOTKEY,
};

use crate::Message;
use crate::CHANNEL;

pub fn spawn_hotkey_thread() {
    thread::spawn(move || unsafe {
        let sender = &CHANNEL.0.clone();

        let _result = RegisterHotKey(
            ptr::null_mut(),
            0,
            MOD_CONTROL as u32 | MOD_ALT as u32 | MOD_NOREPEAT as u32,
            0x53, // S
        );

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
