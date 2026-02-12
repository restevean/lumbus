//! System tray (notification area) icon for Windows.
//!
//! Provides a tray icon with context menu for controlling the overlay.

use std::cell::RefCell;
use windows::core::w;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, LoadImageW, SetForegroundWindow,
    TrackPopupMenu, HMENU, IMAGE_ICON, LR_DEFAULTSIZE, LR_SHARED, MF_STRING, TPM_BOTTOMALIGN,
    TPM_LEFTALIGN, TPM_RIGHTBUTTON, WM_USER,
};

// Custom message for tray icon events
pub const WM_TRAYICON: u32 = WM_USER + 1;

// Menu item IDs
pub const MENU_TOGGLE: u32 = 1001;
pub const MENU_SETTINGS: u32 = 1002;
pub const MENU_HELP: u32 = 1003;
pub const MENU_ABOUT: u32 = 1004;
pub const MENU_QUIT: u32 = 1005;

// Tray icon ID
const TRAY_ICON_ID: u32 = 1;

thread_local! {
    static TRAY_HWND: RefCell<Option<HWND>> = const { RefCell::new(None) };
    static TRAY_MENU: RefCell<Option<HMENU>> = const { RefCell::new(None) };
}

/// Install the system tray icon with context menu.
pub fn install_tray_icon(hwnd: HWND) {
    unsafe {
        TRAY_HWND.with(|h| *h.borrow_mut() = Some(hwnd));

        // Load the custom icon from resources (resource ID 1)
        let hinstance = GetModuleHandleW(None).unwrap_or_default();
        let icon = LoadImageW(
            Some(hinstance.into()),
            windows::core::PCWSTR(1 as *const u16), // Resource ID 1
            IMAGE_ICON,
            16, // Small icon for tray
            16,
            LR_DEFAULTSIZE | LR_SHARED,
        );
        let hicon = match icon {
            Ok(handle) => windows::Win32::UI::WindowsAndMessaging::HICON(handle.0),
            Err(_) => windows::Win32::UI::WindowsAndMessaging::HICON::default(),
        };

        // Create the notification icon
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: hicon,
            ..Default::default()
        };

        // Set tooltip
        let tip = "Lumbus - Mouse Highlighter";
        let tip_wide: Vec<u16> = tip.encode_utf16().collect();
        for (i, &c) in tip_wide.iter().enumerate().take(127) {
            nid.szTip[i] = c;
        }

        let _ = Shell_NotifyIconW(NIM_ADD, &nid);

        // Create context menu
        let menu = CreatePopupMenu().unwrap_or_default();
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            MENU_TOGGLE as usize,
            w!("Toggle (Ctrl+Shift+A)"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            MENU_SETTINGS as usize,
            w!("Settings (Ctrl+,)"),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            MENU_HELP as usize,
            w!("Help (Ctrl+Shift+H)"),
        );
        let _ = AppendMenuW(menu, MF_STRING, MENU_ABOUT as usize, w!("About..."));
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            MENU_QUIT as usize,
            w!("Quit (Ctrl+Shift+X)"),
        );
        let _ = AppendMenuW(menu, MF_STRING, MENU_ABOUT as usize, w!("About..."));
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            MENU_QUIT as usize,
            w!("Quit (Ctrl+Shift+X)"),
        );

        TRAY_MENU.with(|m| *m.borrow_mut() = Some(menu));
    }
}

/// Remove the tray icon.
pub fn remove_tray_icon() {
    TRAY_HWND.with(|h| {
        if let Some(hwnd) = *h.borrow() {
            unsafe {
                let nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: hwnd,
                    uID: TRAY_ICON_ID,
                    ..Default::default()
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            }
        }
    });

    TRAY_MENU.with(|m| {
        if let Some(menu) = m.borrow_mut().take() {
            unsafe {
                let _ = DestroyMenu(menu);
            }
        }
    });
}

/// Show the context menu at the cursor position.
pub fn show_tray_menu(hwnd: HWND) {
    TRAY_MENU.with(|m| {
        if let Some(menu) = *m.borrow() {
            unsafe {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);

                // Required for menu to close when clicking outside
                let _ = SetForegroundWindow(hwnd);

                let _ = TrackPopupMenu(
                    menu,
                    TPM_BOTTOMALIGN | TPM_LEFTALIGN | TPM_RIGHTBUTTON,
                    pt.x,
                    pt.y,
                    None, // nReserved - must be None/0
                    hwnd,
                    None,
                );
            }
        }
    });
}

/// Update tray tooltip to show current state.
pub fn update_tray_tooltip(visible: bool) {
    TRAY_HWND.with(|h| {
        if let Some(hwnd) = *h.borrow() {
            unsafe {
                let mut nid = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: hwnd,
                    uID: TRAY_ICON_ID,
                    uFlags: NIF_TIP,
                    ..Default::default()
                };

                let tip = if visible {
                    "Lumbus - Visible"
                } else {
                    "Lumbus - Hidden"
                };
                let tip_wide: Vec<u16> = tip.encode_utf16().collect();
                for (i, &c) in tip_wide.iter().enumerate().take(127) {
                    nid.szTip[i] = c;
                }

                let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
            }
        }
    });
}

/// Update tray menu language.
pub fn update_tray_language(is_spanish: bool) {
    // Recreate menu with new language
    TRAY_MENU.with(|m| {
        if let Some(old_menu) = m.borrow_mut().take() {
            unsafe {
                let _ = DestroyMenu(old_menu);
            }
        }
    });

    unsafe {
        let menu = CreatePopupMenu().unwrap_or_default();

        if is_spanish {
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_TOGGLE as usize,
                w!("Alternar (Ctrl+Shift+A)"),
            );
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_SETTINGS as usize,
                w!("Configuración (Ctrl+,)"),
            );
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_HELP as usize,
                w!("Ayuda (Ctrl+Shift+H)"),
            );
            let _ = AppendMenuW(menu, MF_STRING, MENU_ABOUT as usize, w!("Acerca de..."));
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_QUIT as usize,
                w!("Salir (Ctrl+Shift+X)"),
            );
        } else {
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_TOGGLE as usize,
                w!("Toggle (Ctrl+Shift+A)"),
            );
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_SETTINGS as usize,
                w!("Settings (Ctrl+,)"),
            );
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_HELP as usize,
                w!("Help (Ctrl+Shift+H)"),
            );
            let _ = AppendMenuW(menu, MF_STRING, MENU_ABOUT as usize, w!("About..."));
            let _ = AppendMenuW(
                menu,
                MF_STRING,
                MENU_QUIT as usize,
                w!("Quit (Ctrl+Shift+X)"),
            );
        }

        TRAY_MENU.with(|m| *m.borrow_mut() = Some(menu));
    }
}
