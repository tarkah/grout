//#![windows_subsystem = "windows"]

#![allow(non_snake_case)]

use std::mem;
use std::ptr;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{bounded, select, unbounded, Receiver, Sender};
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
    BeginPaint, CreateWindowExW, DefWindowProcW, DispatchMessageW, DrawEdge, EndPaint, FillRect,
    FrameRect, GetForegroundWindow, GetMessageW, GetMonitorInfoW, GetWindowLongW, GetWindowRect,
    LoadCursorW, MonitorFromWindow, PeekMessageW, PostQuitMessage, RedrawWindow, RegisterClassExW,
    RegisterHotKey, SendMessageW, SetLayeredWindowAttributes, SetWinEventHook, SetWindowLongW,
    SetWindowPos, ShowWindow, TranslateMessage, UnhookWinEvent, BDR_RAISEDOUTER, BF_FLAT,
    COLOR_3DFACE, COLOR_BTNSHADOW, CS_OWNDC, EVENT_SYSTEM_FOREGROUND, GWL_STYLE, IDC_ARROW,
    LWA_ALPHA, LWA_COLORKEY, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, RDW_INTERNALPAINT, RDW_INVALIDATE, SWP_FRAMECHANGED, SW_SHOW,
    VK_CONTROL, VK_ESCAPE, VK_SHIFT, WINEVENT_OUTOFCONTEXT, WM_DESTROY, WM_HOTKEY, WM_KEYDOWN,
    WM_KEYUP, WM_PAINT, WNDCLASSEXW, WS_BORDER, WS_CAPTION, WS_EX_COMPOSITED, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_MAXIMIZE, WS_POPUP,
    WS_SIZEBOX, WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
};

use crate::common::{get_work_area, Rect};

mod common;
mod grid;
mod window;

lazy_static! {
    static ref CHANNEL: (Sender<Message>, Receiver<Message>) = unbounded();
}

enum Message {
    HighlighterWindow(Window),
    PickerWindow(Window),
    HighlightZone(Rect),
    HotkeyPressed,
    InitializeWindows,
    CloseWindows,
}

fn main() {
    let receiver = &CHANNEL.1.clone();
    let sender = &CHANNEL.0.clone();

    let close_channel = bounded::<()>(3);

    spawn_hotkey_thread();

    let mut highlighter_window = None;
    let mut picker_window = None;

    loop {
        select! {
            recv(receiver) -> msg => {
                match msg.unwrap() {
                    Message::HighlighterWindow(window) => {
                        highlighter_window = Some(window);
                    }
                    Message::PickerWindow(window) => {
                        picker_window = Some(window);

                        spawn_highlighter_window(close_channel.1.clone());
                    }
                    Message::HighlightZone(rect) => {
                        let mut highlighter = highlighter_window.unwrap_or_default();
                        let picker = picker_window.unwrap_or_default();

                        highlighter.set_pos(rect, Some(picker));
                    }
                    Message::HotkeyPressed => {
                        if highlighter_window.is_some() && picker_window.is_some() {
                            let _ = sender.send(Message::CloseWindows);
                        } else {
                            let _ = sender.send(Message::InitializeWindows);
                        }
                    }
                    Message::InitializeWindows => {
                        spawn_foreground_hook(close_channel.1.clone());
                        spawn_picker_window(close_channel.1.clone());
                    }
                    Message::CloseWindows => {
                        highlighter_window.take();
                        picker_window.take();

                        for _ in 0..3 {
                            let _ = close_channel.0.send(());
                        }
                    }
                }
            },
        }
    }
}

fn spawn_foreground_hook(close_msg: Receiver<()>) {
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
                default => {}
            }

            thread::sleep(Duration::from_millis(10));
        }
    });
}

fn spawn_hotkey_thread() {
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

fn spawn_picker_window(close_msg: Receiver<()>) {
    thread::spawn(move || unsafe {
        let hInstance = GetModuleHandleW(ptr::null());

        let class_name = "Grout Zone Picker";
        let mut class_name = class_name.encode_utf16().collect::<Vec<_>>();
        class_name.push(0);

        let mut class = mem::zeroed::<WNDCLASSEXW>();
        class.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        class.lpfnWndProc = Some(callback3);
        class.hInstance = hInstance;
        class.lpszClassName = class_name.as_ptr();
        class.hbrBackground = CreateSolidBrush(RGB(0, 255, 0));
        class.hCursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);

        RegisterClassExW(&class);

        let work_area = get_work_area();

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_COMPOSITED | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            ptr::null(),
            WS_POPUP | WS_VISIBLE | WS_SYSMENU,
            work_area.width / 2 - (48 * 2 + 6 * 3) / 2,
            work_area.height / 2 - (48 * 2 + 6 * 3) / 2,
            48 * 2 + 6 * 3,
            48 * 2 + 6 * 3,
            ptr::null_mut(),
            ptr::null_mut(),
            hInstance,
            ptr::null_mut(),
        );

        SetLayeredWindowAttributes(hwnd, RGB(0, 255, 0), 0, LWA_COLORKEY);

        let _ = &CHANNEL.0.clone().send(Message::PickerWindow(Window(hwnd)));

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
                default => {}
            }

            thread::sleep(Duration::from_millis(10));
        }
    });
}

fn spawn_highlighter_window(close_msg: Receiver<()>) {
    thread::spawn(move || unsafe {
        let hInstance = GetModuleHandleW(ptr::null());

        let class_name = "Grout Zone Highlighter";
        let mut class_name = class_name.encode_utf16().collect::<Vec<_>>();
        class_name.push(0);

        let mut class = mem::zeroed::<WNDCLASSEXW>();
        class.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        class.lpfnWndProc = Some(callback2);
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
        let _ = &CHANNEL.0.clone().send(Message::HighlighterWindow(window));

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
                default => {}
            }

            thread::sleep(Duration::from_millis(10));
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
    let _ = sender.send(Message::HighlightZone(Window::new(hwnd).rect()));
}

unsafe extern "system" fn callback2(
    hWnd: HWND,
    Msg: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hWnd, Msg, wParam, lParam)
}

unsafe extern "system" fn callback3(
    hWnd: HWND,
    Msg: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    //dbg!((Msg, wParam));

    let sender = &CHANNEL.0.clone();

    match Msg {
        WM_PAINT => {
            let mut paint = mem::zeroed();

            let hdc = BeginPaint(hWnd, &mut paint);

            let mut rect: Rect = paint.rcPaint.into();
            rect.x = 6;
            rect.y = 6;
            rect.width = 48;
            rect.height = 48;

            FillRect(hdc, &rect.into(), CreateSolidBrush(RGB(0, 100, 148)));
            FrameRect(hdc, &rect.into(), CreateSolidBrush(RGB(0, 0, 0)));

            let mut rect: Rect = paint.rcPaint.into();
            rect.x = 60;
            rect.y = 6;
            rect.width = 48;
            rect.height = 48;

            FillRect(
                hdc,
                &rect.into(),
                CreateSolidBrush(RGB(
                    (255.0 * (70.0 / 100.0)) as u8,
                    (255.0 * (70.0 / 100.0)) as u8,
                    (255.0 * (70.0 / 100.0)) as u8,
                )),
            );
            FrameRect(hdc, &rect.into(), CreateSolidBrush(RGB(0, 0, 0)));

            let mut rect: Rect = paint.rcPaint.into();
            rect.x = 6;
            rect.y = 60;
            rect.width = 48;
            rect.height = 48;

            FillRect(
                hdc,
                &rect.into(),
                CreateSolidBrush(RGB(
                    (255.0 * (70.0 / 100.0)) as u8,
                    (255.0 * (70.0 / 100.0)) as u8,
                    (255.0 * (70.0 / 100.0)) as u8,
                )),
            );
            FrameRect(hdc, &rect.into(), CreateSolidBrush(RGB(0, 0, 0)));

            let mut rect: Rect = paint.rcPaint.into();
            rect.x = 60;
            rect.y = 60;
            rect.width = 48;
            rect.height = 48;

            FillRect(hdc, &rect.into(), CreateSolidBrush(RGB(0, 77, 128)));
            FrameRect(hdc, &rect.into(), CreateSolidBrush(RGB(0, 0, 0)));

            EndPaint(hWnd, &paint);
        }
        WM_KEYDOWN => match wParam as i32 {
            VK_ESCAPE => {
                let _ = sender.send(Message::CloseWindows);
            }
            VK_CONTROL => {}
            VK_SHIFT => {}
            _ => {}
        },
        WM_KEYUP => match wParam as i32 {
            VK_CONTROL => {}
            VK_SHIFT => {}
            _ => {}
        },
        _ => {}
    }

    DefWindowProcW(hWnd, Msg, wParam, lParam)
}

#[derive(Clone, Copy)]
struct Window(HWND);

unsafe impl Send for Window {}

impl Window {
    pub fn new(hwnd: HWND) -> Self {
        Window(hwnd)
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

            GetWindowRect(self.0, &mut rect);

            rect.into()
        }
    }

    pub fn set_pos(&mut self, rect: Rect, insert_after: Option<Window>) {
        unsafe {
            SetWindowPos(
                self.0,
                insert_after.unwrap_or_default().0,
                rect.x,
                rect.y,
                rect.width,
                rect.height,
                0x0040, // Show
            );
        }
    }
}

impl Default for Window {
    fn default() -> Self {
        Window(ptr::null_mut())
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Window) -> bool {
        self.0 == other.0
    }
}
