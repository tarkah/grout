use std::mem;
use std::ptr;
use std::thread;
use std::time::Duration;

use crossbeam_channel::{bounded, select, unbounded, Receiver, Sender};
use lazy_static::lazy_static;

use winapi::shared::{
    minwindef::{DWORD, LPARAM, LRESULT, UINT, WPARAM},
    windef::{HBRUSH, HDC, HWINEVENTHOOK, HWND, RECT},
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
use crate::Window;

const TILE_WIDTH: u32 = 48;
const TILE_HEIGHT: u32 = 48;

pub struct Grid {
    shift_down: bool,
    control_down: bool,
    margins: u8,
    tiles: Vec<Vec<Tile>>, // tiles[row][column]
}

impl Default for Grid {
    fn default() -> Self {
        Grid {
            shift_down: false,
            control_down: false,
            margins: 6,
            tiles: vec![vec![Tile::default(); 8]; 4],
        }
    }
}

impl Grid {
    pub fn dimensions(&self) -> (u32, u32) {
        let width =
            self.columns() as u32 * TILE_WIDTH + (self.columns() as u32 + 1) * self.margins as u32;

        let height =
            self.rows() as u32 * TILE_HEIGHT + (self.rows() as u32 + 1) * self.margins as u32;

        (width, height)
    }

    fn rows(&self) -> usize {
        self.tiles.len()
    }

    fn columns(&self) -> usize {
        self.tiles[0].len()
    }

    fn add_row(&mut self) {
        self.tiles.push(vec![Tile::default(); self.columns()]);
    }

    fn add_column(&mut self) {
        for row in self.tiles.iter_mut() {
            row.push(Tile::default());
        }
    }

    fn remove_row(&mut self) {
        if self.rows() > 1 {
            self.tiles.pop();
        }
    }

    fn remove_column(&mut self) {
        if self.columns() > 1 {
            for row in self.tiles.iter_mut() {
                row.pop();
            }
        }
    }

    fn tile_area(&self, row: usize, column: usize) -> Rect {
        let x = column as i32 * TILE_WIDTH as i32 + (column as i32 + 1) * self.margins as i32;

        let y = row as i32 * TILE_HEIGHT as i32 + (row as i32 + 1) * self.margins as i32;

        Rect {
            x,
            y,
            width: TILE_WIDTH as i32,
            height: TILE_HEIGHT as i32,
        }
    }

    pub unsafe fn draw(&self, window: Window) {
        let mut paint = mem::zeroed();

        let hdc = BeginPaint(window.0, &mut paint);

        for row in 0..self.rows() {
            for column in 0..self.columns() {
                self.tiles[row][column].draw(hdc, self.tile_area(row, column));
            }
        }

        EndPaint(window.0, &paint);
    }
}

#[derive(Default, Clone, Copy)]
struct Tile {
    selected: bool,
    hovered: bool,
}

impl Tile {
    unsafe fn draw(self, hdc: HDC, area: Rect) {
        FillRect(hdc, &area.into(), self.fill_brush());
        FrameRect(hdc, &area.into(), CreateSolidBrush(RGB(0, 0, 0)));
    }

    unsafe fn fill_brush(self) -> HBRUSH {
        let color = if self.selected {
            RGB(0, 77, 128)
        } else if self.hovered {
            RGB(0, 100, 148)
        } else {
            RGB(
                (255.0 * (70.0 / 100.0)) as u8,
                (255.0 * (70.0 / 100.0)) as u8,
                (255.0 * (70.0 / 100.0)) as u8,
            )
        };

        CreateSolidBrush(color)
    }
}
