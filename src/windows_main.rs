//! Windows-specific entry point and application logic.
//!
//! Uses Direct2D for GPU-accelerated, high-quality anti-aliased rendering
//! with per-pixel alpha transparency via UpdateLayeredWindow.

use std::sync::atomic::Ordering;

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Direct2D::{D2D1CreateFactory, D2D1_FACTORY_TYPE_SINGLE_THREADED};
use windows::Win32::Graphics::DirectWrite::{DWriteCreateFactory, DWRITE_FACTORY_TYPE_SHARED};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetSystemMetrics, LoadCursorW,
    PostQuitMessage, RegisterClassW, SetTimer, SetWindowsHookExW, ShowWindow, TranslateMessage,
    UnhookWindowsHookEx, CS_HREDRAW, CS_VREDRAW, HHOOK, IDC_ARROW, MSG, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOW, WH_MOUSE_LL, WM_COMMAND,
    WM_CREATE, WM_DESTROY, WM_HOTKEY, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

use lumbus::model::constants::*;
use lumbus::platform::windows::app::{reload_settings_from_config, STATE};
use lumbus::platform::windows::input::{
    mouse_hook_proc, HOTKEY_HELP, HOTKEY_QUIT, HOTKEY_SETTINGS, HOTKEY_TOGGLE, MOUSE_HOOK,
    TIMER_CURSOR, TIMER_INTERVAL_MS,
};
use lumbus::platform::windows::storage::config;
use lumbus::platform::windows::ui::dialogs::{show_about_dialog, show_help_overlay};
use lumbus::platform::windows::ui::overlay::{
    create_arial_bold_font_face, update_overlay, D2D_FACTORY, DWRITE_FACTORY, FONT_FACE,
};
use lumbus::platform::windows::ui::settings::window as settings_window;
use lumbus::platform::windows::ui::tray::{
    self, MENU_ABOUT, MENU_HELP, MENU_QUIT, MENU_SETTINGS, MENU_TOGGLE, WM_TRAYICON,
};

/// Main entry point for Windows.
pub fn run() {
    if let Err(e) = run_app() {
        eprintln!("Lumbus error: {}", e);
        std::process::exit(1);
    }
}

fn run_app() -> windows::core::Result<()> {
    unsafe {
        // Initialize COM
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;

        // Create Direct2D factory
        let factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        D2D_FACTORY.with(|f| *f.borrow_mut() = Some(factory));

        // Create DirectWrite factory for text rendering
        let dwrite_factory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

        // Create and cache font face for letter rendering
        if let Some(font_face) = create_arial_bold_font_face(&dwrite_factory) {
            FONT_FACE.with(|f| *f.borrow_mut() = Some(font_face));
        }

        DWRITE_FACTORY.with(|f| *f.borrow_mut() = Some(dwrite_factory));

        let instance = GetModuleHandleW(None)?;
        let class_name = w!("LumbusOverlay");

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        // Get virtual screen dimensions (all monitors)
        let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let vw = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let vh = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        // Create layered, transparent, topmost window
        let ex_style =
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW;

        let hwnd = CreateWindowExW(
            ex_style,
            class_name,
            w!("Lumbus Overlay"),
            WS_POPUP,
            vx,
            vy,
            vw,
            vh,
            None,
            None,
            Some(instance.into()),
            None,
        )?;

        // Store state
        STATE.with(|s| {
            let mut state = s.borrow_mut();
            state.hwnd = hwnd;
            state.width = vw;
            state.height = vh;
            state.offset_x = vx;
            state.offset_y = vy;
        });

        // Load settings from config file
        reload_settings_from_config();

        // Install low-level mouse hook for click detection
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)?;
        MOUSE_HOOK.store(hook.0 as isize, Ordering::SeqCst);

        // Register global hotkeys
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_TOGGLE, MOD_CONTROL | MOD_SHIFT, 0x41); // Ctrl+Shift+A
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_SETTINGS, MOD_CONTROL | MOD_SHIFT, 0x53); // Ctrl+Shift+S
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_HELP, MOD_CONTROL | MOD_SHIFT, 0x48); // Ctrl+Shift+H
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_QUIT, MOD_CONTROL | MOD_SHIFT, 0x51); // Ctrl+Shift+Q

        // Install system tray icon
        tray::install_tray_icon(hwnd);

        // Update tray menu language based on loaded settings
        let is_spanish = STATE.with(|s| s.borrow().lang == LANG_ES);
        tray::update_tray_language(is_spanish);

        // Start timer for cursor tracking
        SetTimer(Some(hwnd), TIMER_CURSOR, TIMER_INTERVAL_MS, None);

        // Initial draw and show
        update_overlay();
        let _ = ShowWindow(hwnd, SW_SHOW);

        // Message loop
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        let hook_handle = MOUSE_HOOK.load(Ordering::SeqCst);
        if hook_handle != 0 {
            let _ = UnhookWindowsHookEx(HHOOK(hook_handle as *mut _));
        }

        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_TOGGLE);
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_SETTINGS);
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY_QUIT);

        // Remove system tray icon
        tray::remove_tray_icon();

        FONT_FACE.with(|f| *f.borrow_mut() = None);
        DWRITE_FACTORY.with(|f| *f.borrow_mut() = None);
        D2D_FACTORY.with(|f| *f.borrow_mut() = None);

        CoUninitialize();

        Ok(())
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_CREATE => LRESULT(0),

            WM_TIMER => {
                if wparam.0 == TIMER_CURSOR {
                    update_overlay();
                }
                LRESULT(0)
            }

            WM_HOTKEY => {
                let hotkey_id = wparam.0 as i32;
                match hotkey_id {
                    HOTKEY_TOGGLE => {
                        let new_visible = STATE.with(|s| {
                            let mut state = s.borrow_mut();
                            state.visible = !state.visible;
                            state.visible
                        });
                        eprintln!(
                            "Toggle: overlay {}",
                            if new_visible { "visible" } else { "hidden" }
                        );
                        update_overlay();
                    }
                    HOTKEY_SETTINGS => {
                        eprintln!("Opening settings window");
                        let hwnd = STATE.with(|s| s.borrow().hwnd);
                        settings_window::open_settings_window(hwnd);
                        reload_settings_from_config();
                        update_overlay();
                    }
                    HOTKEY_HELP => {
                        eprintln!("Showing help overlay");
                        let (hwnd, is_spanish) = STATE.with(|s| {
                            let state = s.borrow();
                            (state.hwnd, state.lang == 1)
                        });
                        show_help_overlay(hwnd, is_spanish);
                    }
                    HOTKEY_QUIT => {
                        PostQuitMessage(0);
                    }
                    _ => {}
                }
                LRESULT(0)
            }

            WM_DESTROY => {
                config::flush_config();
                PostQuitMessage(0);
                LRESULT(0)
            }

            // System tray icon messages
            msg if msg == WM_TRAYICON => {
                let event = lparam.0 as u32;
                if event == 0x0205 {
                    // Right-click: show context menu
                    tray::show_tray_menu(hwnd);
                } else if event == 0x0203 {
                    // Double-click: toggle visibility
                    let new_visible = STATE.with(|s| {
                        let mut state = s.borrow_mut();
                        state.visible = !state.visible;
                        state.visible
                    });
                    tray::update_tray_tooltip(new_visible);
                    update_overlay();
                }
                LRESULT(0)
            }

            // Context menu commands
            WM_COMMAND => {
                let cmd = (wparam.0 & 0xFFFF) as u32;
                match cmd {
                    MENU_TOGGLE => {
                        let new_visible = STATE.with(|s| {
                            let mut state = s.borrow_mut();
                            state.visible = !state.visible;
                            state.visible
                        });
                        tray::update_tray_tooltip(new_visible);
                        update_overlay();
                    }
                    MENU_SETTINGS => {
                        settings_window::open_settings_window(hwnd);
                        reload_settings_from_config();
                        update_overlay();
                    }
                    MENU_ABOUT => {
                        show_about_dialog(hwnd);
                    }
                    MENU_HELP => {
                        let is_spanish = STATE.with(|s| s.borrow().lang == 1);
                        show_help_overlay(hwnd, is_spanish);
                    }
                    MENU_QUIT => {
                        PostQuitMessage(0);
                    }
                    _ => {}
                }
                LRESULT(0)
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
