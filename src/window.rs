use std::mem;
use std::ptr;

use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    GetWindowInfo, GetWindowRect, SetWindowPos, ShowWindow, SWP_NOACTIVATE, SW_RESTORE, WINDOWINFO,
};

use crate::common::Rect;

mod grid;
pub use grid::spawn_grid_window;

mod preview;
pub use preview::spawn_preview_window;

#[derive(Clone, Copy, Debug)]
pub struct Window(pub HWND);

unsafe impl Send for Window {}

impl Window {
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
                SWP_NOACTIVATE,
            );
        }
    }

    pub unsafe fn info(self) -> WindowInfo {
        let mut info: WINDOWINFO = mem::zeroed();
        info.cbSize = mem::size_of::<WINDOWINFO>() as u32;

        GetWindowInfo(self.0, &mut info);

        info.into()
    }

    pub fn transparent_border(self) -> (i32, i32) {
        let info = unsafe { self.info() };

        let x = {
            (info.window_rect.x - info.client_rect.x)
                + (info.window_rect.width - info.client_rect.width)
        };

        let y = {
            (info.window_rect.y - info.client_rect.y)
                + (info.window_rect.height - info.client_rect.height)
        };

        (x, y)
    }

    pub fn restore(&mut self) {
        unsafe {
            ShowWindow(self.0, SW_RESTORE);
        };
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

#[derive(Debug)]
pub struct WindowInfo {
    pub window_rect: Rect,
    pub client_rect: Rect,
    pub styles: u32,
    pub extended_styles: u32,
    pub x_borders: u32,
    pub y_borders: u32,
}

impl From<WINDOWINFO> for WindowInfo {
    fn from(info: WINDOWINFO) -> Self {
        WindowInfo {
            window_rect: info.rcWindow.into(),
            client_rect: info.rcClient.into(),
            styles: info.dwStyle,
            extended_styles: info.dwExStyle,
            x_borders: info.cxWindowBorders,
            y_borders: info.cxWindowBorders,
        }
    }
}
