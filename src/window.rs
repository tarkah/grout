use std::mem;
use std::ptr;

use winapi::shared::windef::HWND;
use winapi::um::winuser::{GetForegroundWindow, GetWindowRect, SetWindowPos, SWP_NOACTIVATE};

use crate::common::Rect;

mod grid;
pub use grid::spawn_grid_window;

mod preview;
pub use preview::spawn_preview_window;

#[derive(Clone, Copy)]
pub struct Window(pub HWND);

unsafe impl Send for Window {}

impl Window {
    pub fn get_foreground() -> Self {
        unsafe {
            let hwnd = GetForegroundWindow();
            Window(hwnd)
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
                SWP_NOACTIVATE,
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
