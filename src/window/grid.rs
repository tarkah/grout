use std::mem;
use std::ptr;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{select, Receiver};

use winapi::shared::{
    minwindef::{HIWORD, LOWORD, LPARAM, LRESULT, UINT, WPARAM},
    windef::HWND,
};

use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::{CreateSolidBrush, RGB};
use winapi::um::winuser::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, InvalidateRect, LoadCursorW, PeekMessageW,
    RegisterClassExW, SendMessageW, ShowWindow, TranslateMessage, IDC_ARROW, SW_RESTORE,
    VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_F1, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_LEFT, VK_RIGHT,
    VK_SHIFT, VK_UP, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSELEAVE,
    WM_MOUSEMOVE, WM_PAINT, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::common::{get_work_area, Rect};
use crate::str_to_wide;
use crate::window::Window;
use crate::Message;
use crate::{CHANNEL, GRID};

pub fn spawn_grid_window(close_msg: Receiver<()>) {
    thread::spawn(move || unsafe {
        let hInstance = GetModuleHandleW(ptr::null());

        let class_name = str_to_wide!("Grout Zone Grid");

        let mut class = mem::zeroed::<WNDCLASSEXW>();
        class.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        class.lpfnWndProc = Some(callback);
        class.hInstance = hInstance;
        class.lpszClassName = class_name.as_ptr();
        class.hbrBackground = CreateSolidBrush(RGB(44, 44, 44));
        class.hCursor = LoadCursorW(ptr::null_mut(), IDC_ARROW);

        RegisterClassExW(&class);

        let work_area = get_work_area();
        let dimensions = GRID.lock().unwrap().dimensions();

        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            ptr::null(),
            WS_POPUP,
            work_area.width / 2 - dimensions.0 as i32 / 2 + work_area.x,
            work_area.height / 2 - dimensions.1 as i32 / 2 + work_area.y,
            dimensions.0 as i32,
            dimensions.1 as i32,
            ptr::null_mut(),
            ptr::null_mut(),
            hInstance,
            ptr::null_mut(),
        );

        let _ = &CHANNEL.0.clone().send(Message::GridWindow(Window(hwnd)));

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
    let sender = &CHANNEL.0.clone();

    let repaint = match Msg {
        WM_PAINT => {
            GRID.lock().unwrap().draw(Window(hWnd));
            false
        }
        WM_KEYDOWN => match wParam as i32 {
            VK_ESCAPE => {
                let _ = sender.send(Message::CloseWindows);
                false
            }
            VK_CONTROL => {
                GRID.lock().unwrap().control_down = true;
                false
            }
            VK_SHIFT => {
                GRID.lock().unwrap().shift_down = true;
                false
            }
            VK_RIGHT => {
                if GRID.lock().unwrap().control_down {
                    GRID.lock().unwrap().add_column();
                    GRID.lock().unwrap().reposition();
                }
                false
            }
            VK_LEFT => {
                if GRID.lock().unwrap().control_down {
                    GRID.lock().unwrap().remove_column();
                    GRID.lock().unwrap().reposition();
                }
                false
            }
            VK_UP => {
                if GRID.lock().unwrap().control_down {
                    GRID.lock().unwrap().add_row();
                    GRID.lock().unwrap().reposition();
                }
                false
            }
            VK_DOWN => {
                if GRID.lock().unwrap().control_down {
                    GRID.lock().unwrap().remove_row();
                    GRID.lock().unwrap().reposition();
                }
                false
            }
            _ => false,
        },
        WM_KEYUP => match wParam as i32 {
            VK_CONTROL => {
                GRID.lock().unwrap().control_down = false;
                false
            }
            VK_SHIFT => {
                GRID.lock().unwrap().shift_down = false;
                false
            }
            VK_F1 => {
                let _ = sender.send(Message::ProfileChange("Default"));
                false
            }
            VK_F2 => {
                let _ = sender.send(Message::ProfileChange("Profile2"));
                false
            }
            VK_F3 => {
                let _ = sender.send(Message::ProfileChange("Profile3"));
                false
            }
            VK_F4 => {
                let _ = sender.send(Message::ProfileChange("Profile4"));
                false
            }
            VK_F5 => {
                let _ = sender.send(Message::ProfileChange("Profile5"));
                false
            }
            VK_F6 => {
                let _ = sender.send(Message::ProfileChange("Profile6"));
                false
            }
            _ => false,
        },
        WM_MOUSEMOVE => {
            let x = LOWORD(lParam as u32) as i32;
            let y = HIWORD(lParam as u32) as i32;

            let _ = sender.send(Message::TrackMouse(Window(hWnd)));

            if let Some(rect) = GRID.lock().unwrap().highlight_tiles((x, y)) {
                let _ = sender.send(Message::HighlightZone(rect));

                true
            } else {
                false
            }
        }
        WM_LBUTTONDOWN => {
            let x = LOWORD(lParam as u32) as i32;
            let y = HIWORD(lParam as u32) as i32;

            let mut grid = GRID.lock().unwrap();

            let repaint = grid.select_tile((x, y));

            grid.cursor_down = true;

            repaint
        }
        WM_LBUTTONUP => {
            let mut grid = GRID.lock().unwrap();

            let repaint = if let Some(mut rect) = grid.selected_area() {
                if let Some(mut active_window) = grid.active_window {
                    if grid.previous_resize != Some((active_window, rect)) {
                        ShowWindow(active_window.0, SW_RESTORE);

                        rect.adjust_for_border(active_window.transparent_border());

                        active_window.set_pos(rect, None);

                        grid.previous_resize = Some((active_window, rect));

                        if grid.quick_resize {
                            let _ = sender.send(Message::CloseWindows);
                        }
                    }
                }

                true
            } else {
                false
            };

            grid.cursor_down = false;

            repaint
        }
        WM_MOUSELEAVE => {
            GRID.lock().unwrap().unhighlight_all_tiles();

            let _ = sender.send(Message::MouseLeft);
            let _ = sender.send(Message::HighlightZone(Rect::zero()));

            true
        }
        _ => false,
    };

    if repaint {
        let dimensions = GRID.lock().unwrap().dimensions();
        let rect = Rect {
            x: 0,
            y: 0,
            width: dimensions.0 as i32,
            height: dimensions.1 as i32,
        };

        InvalidateRect(hWnd, &rect.into(), 0);
        SendMessageW(hWnd, WM_PAINT, 0, 0);
    }

    DefWindowProcW(hWnd, Msg, wParam, lParam)
}
