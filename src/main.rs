#![windows_subsystem = "windows"]
#![allow(non_snake_case)]

use std::mem;
use std::sync::{Arc, Mutex};

use crossbeam_channel::{bounded, select, unbounded, Receiver, Sender};
use lazy_static::lazy_static;

use winapi::um::winuser::{
    GetForegroundWindow, SetForegroundWindow, ShowWindow, TrackMouseEvent, SW_SHOW, TME_LEAVE,
    TRACKMOUSEEVENT,
};

use crate::common::Rect;
use crate::event::spawn_foreground_hook;
use crate::grid::Grid;
use crate::hotkey::{spawn_hotkey_thread, HotkeyType};
use crate::tray::spawn_sys_tray;
use crate::window::{spawn_grid_window, spawn_preview_window, Window};

mod common;
mod config;
mod event;
mod grid;
mod hotkey;
mod tray;
mod window;

lazy_static! {
    static ref CHANNEL: (Sender<Message>, Receiver<Message>) = unbounded();
    static ref GRID: Arc<Mutex<Grid>> = {
        let config = config::load_config();

        Arc::new(Mutex::new(Grid::from(config)))
    };
}

pub enum Message {
    PreviewWindow(Window),
    GridWindow(Window),
    HighlightZone(Rect),
    HotkeyPressed(HotkeyType),
    TrackMouse(Window),
    ActiveWindowChange(Window),
    MouseLeft,
    InitializeWindows,
    CloseWindows,
    Exit,
}

fn main() {
    let receiver = &CHANNEL.1.clone();
    let sender = &CHANNEL.0.clone();

    let close_channel = bounded::<()>(3);

    let config = config::load_config();

    spawn_hotkey_thread(&config.hotkey, HotkeyType::Main);

    if let Some(hotkey) = &config.hotkey_quick_resize {
        spawn_hotkey_thread(hotkey, HotkeyType::QuickResize);
    }

    unsafe {
        spawn_sys_tray();
    }

    let mut preview_window: Option<Window> = None;
    let mut grid_window: Option<Window> = None;
    let mut track_mouse = false;

    loop {
        select! {
            recv(receiver) -> msg => {
                match msg.unwrap() {
                    Message::PreviewWindow(window) => unsafe {
                        preview_window = Some(window);

                        spawn_foreground_hook(close_channel.1.clone());

                        ShowWindow(grid_window.as_ref().unwrap().0, SW_SHOW);
                        SetForegroundWindow(grid_window.as_ref().unwrap().0);
                    }
                    Message::GridWindow(window) => unsafe {
                        grid_window = Some(window);

                        let mut grid = GRID.lock().unwrap();

                        grid.grid_window = Some(window);
                        grid.active_window = Some(Window(GetForegroundWindow()));

                        spawn_preview_window(close_channel.1.clone());
                    }
                    Message::HighlightZone(rect) => {
                        let mut preview_window = preview_window.unwrap_or_default();
                        let grid_window = grid_window.unwrap_or_default();

                        preview_window.set_pos(rect, Some(grid_window));
                    }
                    Message::HotkeyPressed(hotkey_type) => {
                        if preview_window.is_some() && grid_window.is_some() {
                            let _ = sender.send(Message::CloseWindows);
                        } else {
                            let _ = sender.send(Message::InitializeWindows);

                            if hotkey_type == HotkeyType::QuickResize {
                                GRID.lock().unwrap().quick_resize = true;
                            }
                        }
                    }
                    Message::TrackMouse(window) => unsafe {
                        if !track_mouse {
                            let mut event_track: TRACKMOUSEEVENT = mem::zeroed();
                            event_track.cbSize = mem::size_of::<TRACKMOUSEEVENT>() as u32;
                            event_track.dwFlags = TME_LEAVE;
                            event_track.hwndTrack = window.0;

                            TrackMouseEvent(&mut event_track);

                            track_mouse = true;
                        }
                    }
                    Message::MouseLeft => {
                        track_mouse = false;
                    }
                    Message::ActiveWindowChange(window) => {
                        let mut grid = GRID.lock().unwrap();

                        if grid.grid_window != Some(window) && grid.active_window != Some(window) {
                            grid.active_window = Some(window);
                        }
                    }
                    Message::InitializeWindows => {
                        spawn_grid_window(close_channel.1.clone());
                    }
                    Message::CloseWindows => {
                        preview_window.take();
                        grid_window.take();

                        for _ in 0..3 {
                            let _ = close_channel.0.send(());
                        }

                        let mut grid = GRID.lock().unwrap();

                        grid.reset();
                        track_mouse = false;
                    }
                    Message::Exit => {
                        break;
                    }
                }
            },
        }
    }
}
