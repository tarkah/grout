#![allow(non_snake_case)]
//#![windows_subsystem = "windows"]

use std::mem;
use std::ptr;
use std::thread;

use crossbeam_channel::{select, unbounded, Receiver, Sender};
use lazy_static::lazy_static;

use winapi::shared::{
    minwindef::{DWORD, LPARAM, LRESULT, UINT, WPARAM},
    windef::{HBRUSH, HWINEVENTHOOK, HWND, RECT},
};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::{CreateSolidBrush, RGB};
use winapi::um::winnt::LONG;
use winapi::um::winuser::{
    BeginPaint, CreateWindowExW, DefWindowProcW, DispatchMessageW, DrawEdge, EndPaint,
    GetForegroundWindow, GetMessageW, GetMonitorInfoW, GetWindowLongW, GetWindowRect,
    MonitorFromWindow, PostQuitMessage, RegisterClassExW, SetLayeredWindowAttributes,
    SetWinEventHook, SetWindowLongW, SetWindowPos, ShowWindow, TranslateMessage, BDR_RAISEDOUTER,
    BF_FLAT, COLOR_3DFACE, COLOR_BTNSHADOW, CS_OWNDC, EVENT_SYSTEM_FOREGROUND, GWL_STYLE,
    LWA_ALPHA, LWA_COLORKEY, MONITORINFO, MONITOR_DEFAULTTONEAREST, SWP_FRAMECHANGED, SW_SHOW,
    WINEVENT_OUTOFCONTEXT, WNDCLASSEXW, WS_BORDER, WS_CAPTION, WS_EX_COMPOSITED, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MAXIMIZE, WS_POPUP, WS_SIZEBOX,
    WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
};

mod common;
use common::Rect;

lazy_static! {
    static ref CHANNEL: (Sender<Message>, Receiver<Message>) = unbounded();
}

enum Message {
    MainWindowSetup(Window),
    HighlightZone(Rect),
}

fn main() {
    thread::spawn(|| unsafe {
        let hInstance = GetModuleHandleW(ptr::null());

        let _hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            ptr::null_mut(),
            Some(callback),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );

        let class_name = "Grout Zone Highlighter";
        let mut class_name = class_name.encode_utf16().collect::<Vec<_>>();
        class_name.push(0);

        let mut class = mem::zeroed::<WNDCLASSEXW>();
        class.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        class.style = CS_OWNDC;
        class.lpfnWndProc = Some(DefWindowProcW);
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

        let window = Window::new(hwnd);
        let _ = &CHANNEL.0.clone().send(Message::MainWindowSetup(window));

        let mut msg = mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });

    let receiver = &CHANNEL.1.clone();
    let mut main_window = None;

    loop {
        select! {
            recv(receiver) -> msg => {
                match msg.unwrap() {
                    Message::MainWindowSetup(window) => {
                        main_window = Some(window);
                    }
                    Message::HighlightZone(rect) => {
                        if let Some(mut main) = main_window {
                            main.set_pos_from_rect(rect);
                        }
                    }
                }
            },
        }
    }
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
    let _ = sender.send(Message::HighlightZone(Window::new(hwnd).rect()));
}

#[derive(Clone, Copy)]
struct Window {
    hwnd: HWND,
}

unsafe impl Send for Window {}

impl Window {
    pub fn new(hwnd: HWND) -> Self {
        Window { hwnd }
    }

    pub fn get_foreground() -> Self {
        unsafe {
            let hwnd = GetForegroundWindow();
            Window::new(hwnd)
        }
    }

    pub fn rect(self) -> Rect {
        unsafe {
            let mut rect = mem::zeroed();

            GetWindowRect(self.hwnd, &mut rect);

            rect.into()
        }
    }

    pub fn set_pos(&mut self, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(), // Place window at top of z order
                x,
                y,
                width,
                height,
                0x0040, // Show
            );
        }
    }

    pub fn set_pos_from_rect(&mut self, rect: Rect) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                ptr::null_mut(), // Place window at top of z order
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                0x0040, // Show
            );
        }
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Window) -> bool {
        self.hwnd == other.hwnd
    }
}
