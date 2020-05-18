use std::mem;
use std::ptr;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{select, Receiver};

use winapi::shared::{
    minwindef::{LPARAM, LRESULT, UINT, WPARAM},
    windef::HWND,
};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::{CreateSolidBrush, RGB};

use winapi::um::winuser::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, PeekMessageW, RegisterClassExW,
    SetLayeredWindowAttributes, TranslateMessage, LWA_ALPHA, WNDCLASSEXW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_SYSMENU, WS_VISIBLE,
};

use crate::str_to_wide;
use crate::window::Window;
use crate::Message;
use crate::CHANNEL;

pub fn spawn_preview_window(close_msg: Receiver<()>) {
    thread::spawn(move || unsafe {
        let hInstance = GetModuleHandleW(ptr::null());

        let class_name = str_to_wide!("Grout Zone Preview");

        let mut class = mem::zeroed::<WNDCLASSEXW>();
        class.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        class.lpfnWndProc = Some(callback);
        class.hInstance = hInstance;
        class.lpszClassName = class_name.as_ptr();
        class.hbrBackground = CreateSolidBrush(RGB(0, 77, 128));

        RegisterClassExW(&class);

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            ptr::null(),
            WS_POPUP | WS_VISIBLE | WS_SYSMENU,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            hInstance,
            ptr::null_mut(),
        );

        SetLayeredWindowAttributes(hwnd, 0, 107, LWA_ALPHA);

        let _ = &CHANNEL.0.clone().send(Message::PreviewWindow(Window(hwnd)));

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

unsafe extern "system" fn callback(
    hWnd: HWND,
    Msg: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hWnd, Msg, wParam, lParam)
}
