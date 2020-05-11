use std::mem;
use std::process;
use std::ptr;

use winapi::shared::windef::{POINT, RECT};
use winapi::um::winuser::{
    GetCursorPos, GetMonitorInfoW, MessageBoxW, MonitorFromPoint, MB_OK, MONITORINFOEXW,
    MONITOR_DEFAULTTONEAREST,
};

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
        let mut cursor_pos: POINT = mem::zeroed();
        GetCursorPos(&mut cursor_pos);

        MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST)
    };

    let work_area: Rect = {
        let mut info: MONITORINFOEXW = mem::zeroed();
        info.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

        GetMonitorInfoW(active_monitor, &mut info as *mut MONITORINFOEXW as *mut _);

        info.rcWork.into()
    };

    work_area
}

pub unsafe fn get_active_monitor_name() -> String {
    let active_monitor = {
        let mut cursor_pos: POINT = mem::zeroed();
        GetCursorPos(&mut cursor_pos);

        MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST)
    };

    let mut info: MONITORINFOEXW = mem::zeroed();
    info.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

    GetMonitorInfoW(active_monitor, &mut info as *mut MONITORINFOEXW as *mut _);

    String::from_utf16_lossy(&info.szDevice)
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
