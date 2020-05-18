use std::mem;
use std::ptr;
use std::thread;

use winapi::shared::{
    minwindef::{LOWORD, LPARAM, LRESULT, UINT, WPARAM},
    windef::{HWND, POINT},
};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::shellapi::{
    ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NOTIFYICONDATAW,
};
use winapi::um::wingdi::{CreateSolidBrush, RGB};
use winapi::um::winuser::{
    CheckMenuItem, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DispatchMessageW, GetCursorPos, GetMessageW, InsertMenuW, MessageBoxW,
    PostMessageW, PostQuitMessage, RegisterClassExW, SendMessageW, SetFocus, SetForegroundWindow,
    SetMenuDefaultItem, SetMenuItemBitmaps, TrackPopupMenu, TranslateMessage, LR_DEFAULTCOLOR,
    MB_ICONINFORMATION, MB_OK, MF_BYPOSITION, MF_CHECKED, MF_STRING, MF_UNCHECKED, SW_SHOW,
    TPM_LEFTALIGN, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_CLOSE, WM_COMMAND,
    WM_CREATE, WM_INITMENUPOPUP, WM_LBUTTONDBLCLK, WM_RBUTTONUP, WNDCLASSEXW, WS_EX_NOACTIVATE,
};

use crate::autostart;
use crate::config;
use crate::str_to_wide;
use crate::Message;
use crate::CHANNEL;
use crate::CONFIG;

const ID_ABOUT: u16 = 2000;
const ID_EXIT: u16 = 2001;
const ID_CONFIG: u16 = 2002;
const ID_AUTOSTART: u16 = 2003;
static mut MODAL_SHOWN: bool = false;

pub unsafe fn spawn_sys_tray() {
    thread::spawn(|| {
        let hInstance = GetModuleHandleW(ptr::null());

        let class_name = str_to_wide!("Grout Tray");

        let mut class = mem::zeroed::<WNDCLASSEXW>();
        class.cbSize = mem::size_of::<WNDCLASSEXW>() as u32;
        class.lpfnWndProc = Some(callback);
        class.hInstance = hInstance;
        class.lpszClassName = class_name.as_ptr();
        class.hbrBackground = CreateSolidBrush(RGB(0, 77, 128));

        RegisterClassExW(&class);

        CreateWindowExW(
            WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            ptr::null(),
            0,
            0,
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            hInstance,
            ptr::null_mut(),
        );

        let mut msg = mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

unsafe fn add_icon(hwnd: HWND) {
    let icon_bytes = include_bytes!("../assets/icon_32.png");

    let icon_handle = CreateIconFromResourceEx(
        icon_bytes.as_ptr() as *mut _,
        icon_bytes.len() as u32,
        1,
        0x00_030_000,
        32,
        32,
        LR_DEFAULTCOLOR,
    );

    let mut tooltip_array = [0u16; 128];
    let tooltip = "Grout";
    let mut tooltip = tooltip.encode_utf16().collect::<Vec<_>>();
    tooltip.extend(vec![0; 128 - tooltip.len()]);
    tooltip_array.swap_with_slice(&mut tooltip[..]);

    let mut icon_data: NOTIFYICONDATAW = mem::zeroed();
    icon_data.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
    icon_data.hWnd = hwnd;
    icon_data.uID = 1;
    icon_data.uCallbackMessage = WM_APP;
    icon_data.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    icon_data.hIcon = icon_handle;
    icon_data.szTip = tooltip_array;

    Shell_NotifyIconW(NIM_ADD, &mut icon_data);
}

unsafe fn remove_icon(hwnd: HWND) {
    let mut icon_data: NOTIFYICONDATAW = mem::zeroed();
    icon_data.hWnd = hwnd;
    icon_data.uID = 1;

    Shell_NotifyIconW(NIM_DELETE, &mut icon_data);
}

unsafe fn show_popup_menu(hwnd: HWND) {
    if MODAL_SHOWN {
        return;
    }

    let menu = CreatePopupMenu();

    let mut about = str_to_wide!("About...");
    let mut auto_start = str_to_wide!("Launch at startup");
    let mut open_config = str_to_wide!("Open Config");
    let mut exit = str_to_wide!("Exit");

    InsertMenuW(
        menu,
        0,
        MF_BYPOSITION | MF_STRING,
        ID_ABOUT as usize,
        about.as_mut_ptr(),
    );

    InsertMenuW(
        menu,
        1,
        MF_BYPOSITION | MF_STRING,
        ID_AUTOSTART as usize,
        auto_start.as_mut_ptr(),
    );

    SetMenuItemBitmaps(menu, 1, MF_BYPOSITION, ptr::null_mut(), ptr::null_mut());

    let checked = if CONFIG.lock().unwrap().auto_start {
        MF_CHECKED
    } else {
        MF_UNCHECKED
    };

    CheckMenuItem(menu, 1, MF_BYPOSITION | checked);

    InsertMenuW(
        menu,
        2,
        MF_BYPOSITION | MF_STRING,
        ID_CONFIG as usize,
        open_config.as_mut_ptr(),
    );

    InsertMenuW(
        menu,
        3,
        MF_BYPOSITION | MF_STRING,
        ID_EXIT as usize,
        exit.as_mut_ptr(),
    );

    SetMenuDefaultItem(menu, ID_ABOUT as u32, 0);
    SetFocus(hwnd);
    SendMessageW(hwnd, WM_INITMENUPOPUP, menu as usize, 0);

    let mut point: POINT = mem::zeroed();
    GetCursorPos(&mut point);

    let cmd = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
        point.x,
        point.y,
        0,
        hwnd,
        ptr::null_mut(),
    );

    SendMessageW(hwnd, WM_COMMAND, cmd as usize, 0);

    DestroyMenu(menu);
}

unsafe fn show_about() {
    let mut title = str_to_wide!("About");

    let msg = format!(
        "Grout - v{}\n\nCopyright © 2020 Cory Forsstrom",
        env!("CARGO_PKG_VERSION")
    );

    let mut msg = str_to_wide!(msg);

    MessageBoxW(
        ptr::null_mut(),
        msg.as_mut_ptr(),
        title.as_mut_ptr(),
        MB_ICONINFORMATION | MB_OK,
    );
}

unsafe extern "system" fn callback(
    hWnd: HWND,
    Msg: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    match Msg {
        WM_CREATE => {
            add_icon(hWnd);
            return 0;
        }
        WM_CLOSE => {
            remove_icon(hWnd);
            PostQuitMessage(0);
            let _ = &CHANNEL.0.clone().send(Message::Exit);
        }
        WM_COMMAND => {
            if MODAL_SHOWN {
                return 1;
            }

            match LOWORD(wParam as u32) {
                ID_ABOUT => {
                    MODAL_SHOWN = true;

                    show_about();

                    MODAL_SHOWN = false;
                }
                ID_AUTOSTART => {
                    config::toggle_autostart();

                    let mut config = CONFIG.lock().unwrap();
                    *config = config::load_config();

                    autostart::toggle_autostart_registry_key(config.auto_start);
                }
                ID_CONFIG => {
                    if let Some(mut config_path) = dirs::config_dir() {
                        config_path.push("grout");
                        config_path.push("config.yml");

                        if config_path.exists() {
                            let mut operation = str_to_wide!("open");
                            let mut config_path = str_to_wide!(config_path.to_str().unwrap());

                            ShellExecuteW(
                                hWnd,
                                operation.as_mut_ptr(),
                                config_path.as_mut_ptr(),
                                ptr::null_mut(),
                                ptr::null_mut(),
                                SW_SHOW,
                            );
                        }
                    }
                }
                ID_EXIT => {
                    PostMessageW(hWnd, WM_CLOSE, 0, 0);
                }
                _ => {}
            }

            return 0;
        }
        WM_APP => {
            match lParam as u32 {
                WM_LBUTTONDBLCLK => show_about(),
                WM_RBUTTONUP => {
                    SetForegroundWindow(hWnd);
                    show_popup_menu(hWnd);
                    PostMessageW(hWnd, WM_APP + 1, 0, 0);
                }
                _ => {}
            }

            return 0;
        }
        _ => {}
    }

    DefWindowProcW(hWnd, Msg, wParam, lParam)
}
