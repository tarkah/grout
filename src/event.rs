use std::mem;
use std::ptr;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{select, Receiver};

use winapi::shared::{
    minwindef::DWORD,
    windef::{HWINEVENTHOOK, HWND},
};
use winapi::um::winnt::LONG;
use winapi::um::winuser::{
    DispatchMessageW, PeekMessageW, SetWinEventHook, TranslateMessage, EVENT_SYSTEM_FOREGROUND,
    WINEVENT_OUTOFCONTEXT,
};

use crate::common::get_active_monitor_name;
use crate::window::Window;
use crate::Message;
use crate::CHANNEL;

pub fn spawn_foreground_hook(close_msg: Receiver<()>) {
    thread::spawn(move || unsafe {
        SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            ptr::null_mut(),
            Some(callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );

        let mut msg = mem::zeroed();
        loop {
            if PeekMessageW(&mut msg, ptr::null_mut(), 0, 0, 1) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            };

            select! {
                recv(close_msg) -> _ => {
                    break;
                }
                default(Duration::from_millis(10)) => {}
            }
        }
    });
}

pub fn spawn_track_monitor_thread(close_msg: Receiver<()>) {
    thread::spawn(move || unsafe {
        let sender = &CHANNEL.0.clone();

        let mut previous_monitor = get_active_monitor_name();

        loop {
            let current_monitor = get_active_monitor_name();

            if current_monitor != previous_monitor {
                previous_monitor = current_monitor.clone();

                let _ = sender.send(Message::MonitorChange);
            }

            select! {
                recv(close_msg) -> _ => {
                    break;
                }
                default(Duration::from_millis(10)) => {}
            }
        }
    });
}

unsafe extern "system" fn callback(
    _hWinEventHook: HWINEVENTHOOK,
    _event: DWORD,
    hwnd: HWND,
    _idObject: LONG,
    _idChild: LONG,
    _idEventThread: DWORD,
    _dwmsEventTime: DWORD,
) {
    let sender = &CHANNEL.0.clone();
    let _ = sender.send(Message::ActiveWindowChange(Window(hwnd)));
}
