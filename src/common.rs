use std::mem;
use std::process;
use std::ptr;

use winapi::shared::windef::RECT;
use winapi::um::winuser::{
    GetMonitorInfoW, MessageBoxW, MonitorFromWindow, MB_OK, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};

use crate::window::Window;

/// x & y coordinates are relative to top left of screen
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn contains_point(self, point: (i32, i32)) -> bool {
        point.0 >= self.x
            && point.0 <= self.x + self.width
            && point.1 >= self.y
            && point.1 <= self.y + self.height
    }

    pub fn zero() -> Self {
        Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

impl From<RECT> for Rect {
    fn from(rect: RECT) -> Self {
        Rect {
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        }
    }
}

impl From<Rect> for RECT {
    fn from(rect: Rect) -> Self {
        RECT {
            left: rect.x,
            top: rect.y,
            right: rect.x + rect.width,
            bottom: rect.y + rect.height,
        }
    }
}

pub unsafe fn get_work_area() -> Rect {
    let active_monitor = {
        let active_window = Window::get_foreground();
        MonitorFromWindow(active_window.0, MONITOR_DEFAULTTONEAREST)
    };

    let work_area: Rect = {
        let mut info: MONITORINFO = mem::zeroed();
        info.cbSize = mem::size_of::<MONITORINFO>() as u32;

        GetMonitorInfoW(active_monitor, &mut info);

        info.rcWork.into()
    };

    work_area
}

pub unsafe fn report_and_exit(error_msg: &str) {
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
