//! Settings window for Windows.
//!
//! A modal dialog with controls for configuring the overlay appearance.

use crate::model::constants::*;
use crate::platform::windows::storage::config;
use crate::platform::windows::ui::tray;
use std::cell::RefCell;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::Dialogs::{ChooseColorW, CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, LoadCursorW, PostMessageW, RegisterClassW, SendMessageW, SetWindowLongPtrW,
    SetWindowTextW, ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT,
    GWLP_USERDATA, HMENU, IDC_ARROW, MSG, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE,
    WM_COMMAND, WM_CREATE, WM_DESTROY, WM_HSCROLL, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_OVERLAPPED,
    WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};

// Control IDs
const ID_RADIUS_SLIDER: i32 = 102;
const ID_RADIUS_VALUE: i32 = 103;
const ID_BORDER_SLIDER: i32 = 104;
const ID_BORDER_VALUE: i32 = 105;
const ID_COLOR_BUTTON: i32 = 106;
const ID_TRANSP_SLIDER: i32 = 107;
const ID_TRANSP_VALUE: i32 = 108;
const ID_LANG_COMBO: i32 = 109;
const ID_CLOSE_BUTTON: i32 = 110;

// Trackbar messages (from commctrl.h)
const TBM_SETRANGE: u32 = 0x0406;
const TBM_SETPOS: u32 = 0x0405;
const TBM_GETPOS: u32 = 0x0400;

// ComboBox messages
const CB_ADDSTRING: u32 = 0x0143;
const CB_SETCURSEL: u32 = 0x014E;
const CB_GETCURSEL: u32 = 0x0147;
const CBN_SELCHANGE: u32 = 1;

// Window dimensions
const WINDOW_WIDTH: i32 = 400;
const WINDOW_HEIGHT: i32 = 300;

// Layout constants
const MARGIN: i32 = 20;
const ROW_HEIGHT: i32 = 40;
const LABEL_WIDTH: i32 = 140;
const VALUE_WIDTH: i32 = 50;
const SLIDER_WIDTH: i32 = 150;

thread_local! {
    static SETTINGS_HWND: RefCell<Option<HWND>> = const { RefCell::new(None) };
    static PARENT_HWND: RefCell<Option<HWND>> = const { RefCell::new(None) };
    static ON_SETTINGS_CHANGED: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

/// Set callback for when settings change.
pub fn set_on_settings_changed<F: Fn() + 'static>(callback: F) {
    ON_SETTINGS_CHANGED.with(|c| {
        *c.borrow_mut() = Some(Box::new(callback));
    });
}

fn notify_settings_changed() {
    ON_SETTINGS_CHANGED.with(|c| {
        if let Some(ref callback) = *c.borrow() {
            callback();
        }
    });
}

/// Open the settings window.
pub fn open_settings_window(parent_hwnd: HWND) {
    // Check if already open
    let already_open = SETTINGS_HWND.with(|h| h.borrow().is_some());
    if already_open {
        return;
    }

    PARENT_HWND.with(|h| *h.borrow_mut() = Some(parent_hwnd));

    unsafe {
        // Register window class
        let class_name = w!("LumbusSettings");
        let hinstance = GetModuleHandleW(None).unwrap_or_default();

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(settings_wnd_proc),
            hInstance: hinstance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH(GetStockObject(windows::Win32::Graphics::Gdi::WHITE_BRUSH).0),
            lpszClassName: class_name,
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);

        // Create window
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("Settings"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            Some(parent_hwnd),
            None,
            Some(hinstance.into()),
            None,
        );

        let hwnd = match hwnd {
            Ok(h) => h,
            Err(_) => {
                eprintln!("Failed to create settings window");
                return;
            }
        };

        SETTINGS_HWND.with(|h| *h.borrow_mut() = Some(hwnd));

        // Disable parent window (modal behavior)
        let _ = EnableWindow(parent_hwnd, false);

        let _ = ShowWindow(hwnd, SW_SHOW);

        // Message loop for the settings window
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let still_open = SETTINGS_HWND.with(|h| h.borrow().is_some());
            if !still_open {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Close the settings window.
pub fn close_settings_window() {
    // Take the HWND first, releasing the borrow before calling DestroyWindow
    // (DestroyWindow sends WM_DESTROY synchronously which would cause a borrow conflict)
    let hwnd_to_destroy = SETTINGS_HWND.with(|h| h.borrow_mut().take());

    if let Some(hwnd) = hwnd_to_destroy {
        unsafe {
            PARENT_HWND.with(|p| {
                if let Some(parent) = *p.borrow() {
                    let _ = EnableWindow(parent, true);
                }
            });
            let _ = DestroyWindow(hwnd);
        }
    }
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            create_controls(hwnd);
            LRESULT(0)
        }

        WM_COMMAND => {
            let control_id = (wparam.0 & 0xFFFF) as i32;
            let notification = ((wparam.0 >> 16) & 0xFFFF) as u32;
            handle_command(hwnd, control_id, notification, lparam);
            LRESULT(0)
        }

        WM_HSCROLL => {
            let slider_hwnd = HWND(lparam.0 as *mut _);
            handle_slider_change(slider_hwnd);
            LRESULT(0)
        }

        WM_CLOSE => {
            close_settings_window();
            LRESULT(0)
        }

        WM_DESTROY => {
            // Cleanup already done by close_settings_window, just post quit
            PostMessageW(None, 0x0012, WPARAM(0), LPARAM(0)).ok();
            LRESULT(0)
        }

        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn create_controls(hwnd: HWND) {
    let hinstance = GetModuleHandleW(None).unwrap_or_default();
    let state = config::load_state();
    let is_spanish = state.lang == LANG_ES;

    // Update window title based on language
    let title = if is_spanish {
        "Configuración"
    } else {
        "Settings"
    };
    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let _ = SetWindowTextW(hwnd, PCWSTR(title_wide.as_ptr()));

    let mut y = MARGIN;

    // Radius row
    let radius_label = if is_spanish {
        "Radio (px)"
    } else {
        "Radius (px)"
    };
    create_label(hwnd, hinstance.into(), MARGIN, y, radius_label);
    let radius_value = create_value_label(
        hwnd,
        hinstance.into(),
        MARGIN + LABEL_WIDTH,
        y,
        ID_RADIUS_VALUE,
    );
    set_value_text(radius_value, state.radius as i32);
    let radius_slider = create_slider(
        hwnd,
        hinstance.into(),
        MARGIN + LABEL_WIDTH + VALUE_WIDTH + 10,
        y,
        ID_RADIUS_SLIDER,
    );
    init_slider(
        radius_slider,
        MIN_RADIUS as i32,
        MAX_RADIUS as i32,
        state.radius as i32,
    );
    SetWindowLongPtrW(radius_slider, GWLP_USERDATA, radius_value.0 as isize);

    y += ROW_HEIGHT;

    // Border row
    let border_label = if is_spanish {
        "Borde (px)"
    } else {
        "Border (px)"
    };
    create_label(hwnd, hinstance.into(), MARGIN, y, border_label);
    let border_value = create_value_label(
        hwnd,
        hinstance.into(),
        MARGIN + LABEL_WIDTH,
        y,
        ID_BORDER_VALUE,
    );
    set_value_text(border_value, state.border_width as i32);
    let border_slider = create_slider(
        hwnd,
        hinstance.into(),
        MARGIN + LABEL_WIDTH + VALUE_WIDTH + 10,
        y,
        ID_BORDER_SLIDER,
    );
    init_slider(
        border_slider,
        MIN_BORDER as i32,
        MAX_BORDER as i32,
        state.border_width as i32,
    );
    SetWindowLongPtrW(border_slider, GWLP_USERDATA, border_value.0 as isize);

    y += ROW_HEIGHT;

    // Color row
    create_label(hwnd, hinstance.into(), MARGIN, y, "Color");
    let choose_label = if is_spanish { "Elegir..." } else { "Choose..." };
    create_button(
        hwnd,
        hinstance.into(),
        MARGIN + LABEL_WIDTH,
        y,
        choose_label,
        ID_COLOR_BUTTON,
        80,
    );

    y += ROW_HEIGHT;

    // Transparency row
    let transp_label = if is_spanish {
        "Transparencia (%)"
    } else {
        "Fill Transparency (%)"
    };
    create_label(hwnd, hinstance.into(), MARGIN, y, transp_label);
    let transp_value = create_value_label(
        hwnd,
        hinstance.into(),
        MARGIN + LABEL_WIDTH,
        y,
        ID_TRANSP_VALUE,
    );
    set_value_text(transp_value, state.fill_transparency_pct as i32);
    let transp_slider = create_slider(
        hwnd,
        hinstance.into(),
        MARGIN + LABEL_WIDTH + VALUE_WIDTH + 10,
        y,
        ID_TRANSP_SLIDER,
    );
    init_slider(
        transp_slider,
        MIN_TRANSPARENCY as i32,
        MAX_TRANSPARENCY as i32,
        state.fill_transparency_pct as i32,
    );
    SetWindowLongPtrW(transp_slider, GWLP_USERDATA, transp_value.0 as isize);

    y += ROW_HEIGHT;

    // Language row
    let lang_label = if is_spanish { "Idioma" } else { "Language" };
    create_label(hwnd, hinstance.into(), MARGIN, y, lang_label);
    let lang_combo = create_combobox(
        hwnd,
        hinstance.into(),
        MARGIN + LABEL_WIDTH,
        y,
        ID_LANG_COMBO,
    );
    // Add language options
    let en_text: Vec<u16> = "English".encode_utf16().chain(std::iter::once(0)).collect();
    let es_text: Vec<u16> = "Español".encode_utf16().chain(std::iter::once(0)).collect();
    SendMessageW(
        lang_combo,
        CB_ADDSTRING,
        None,
        Some(LPARAM(en_text.as_ptr() as isize)),
    );
    SendMessageW(
        lang_combo,
        CB_ADDSTRING,
        None,
        Some(LPARAM(es_text.as_ptr() as isize)),
    );
    // Set current selection (0 = English, 1 = Spanish)
    let current_lang = if state.lang == LANG_ES { 1 } else { 0 };
    SendMessageW(lang_combo, CB_SETCURSEL, Some(WPARAM(current_lang)), None);

    y += ROW_HEIGHT + 10;

    // Close button
    let close_label = if is_spanish { "Cerrar" } else { "Close" };
    create_button(
        hwnd,
        hinstance.into(),
        WINDOW_WIDTH - 100 - MARGIN,
        y,
        close_label,
        ID_CLOSE_BUTTON,
        80,
    );
}

unsafe fn create_label(
    hwnd: HWND,
    hinstance: windows::Win32::Foundation::HINSTANCE,
    x: i32,
    y: i32,
    text: &str,
) {
    let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("STATIC"),
        PCWSTR(text_wide.as_ptr()),
        WS_CHILD | WS_VISIBLE,
        x,
        y + 4,
        LABEL_WIDTH,
        20,
        Some(hwnd),
        None,
        Some(hinstance),
        None,
    );
}

unsafe fn create_value_label(
    hwnd: HWND,
    hinstance: windows::Win32::Foundation::HINSTANCE,
    x: i32,
    y: i32,
    id: i32,
) -> HWND {
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("STATIC"),
        w!("0"),
        WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0x0001), // SS_CENTER
        x,
        y + 2,
        VALUE_WIDTH,
        22,
        Some(hwnd),
        Some(HMENU(id as *mut _)),
        Some(hinstance),
        None,
    )
    .unwrap_or_default()
}

unsafe fn create_slider(
    hwnd: HWND,
    hinstance: windows::Win32::Foundation::HINSTANCE,
    x: i32,
    y: i32,
    id: i32,
) -> HWND {
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("msctls_trackbar32"),
        None,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        x,
        y,
        SLIDER_WIDTH,
        28,
        Some(hwnd),
        Some(HMENU(id as *mut _)),
        Some(hinstance),
        None,
    )
    .unwrap_or_default()
}

unsafe fn create_button(
    hwnd: HWND,
    hinstance: windows::Win32::Foundation::HINSTANCE,
    x: i32,
    y: i32,
    text: &str,
    id: i32,
    width: i32,
) {
    let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let _ = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("BUTTON"),
        PCWSTR(text_wide.as_ptr()),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
        x,
        y,
        width,
        26,
        Some(hwnd),
        Some(HMENU(id as *mut _)),
        Some(hinstance),
        None,
    );
}

unsafe fn create_combobox(
    hwnd: HWND,
    hinstance: windows::Win32::Foundation::HINSTANCE,
    x: i32,
    y: i32,
    id: i32,
) -> HWND {
    // CBS_DROPDOWNLIST = 0x0003
    CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        w!("COMBOBOX"),
        None,
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(0x0003),
        x,
        y,
        120,
        100, // Height includes dropdown area
        Some(hwnd),
        Some(HMENU(id as *mut _)),
        Some(hinstance),
        None,
    )
    .unwrap_or_default()
}

unsafe fn init_slider(slider: HWND, min: i32, max: i32, pos: i32) {
    let range = ((max as isize) << 16) | (min as isize);
    SendMessageW(slider, TBM_SETRANGE, Some(WPARAM(1)), Some(LPARAM(range)));
    SendMessageW(
        slider,
        TBM_SETPOS,
        Some(WPARAM(1)),
        Some(LPARAM(pos as isize)),
    );
}

unsafe fn set_value_text(hwnd: HWND, value: i32) {
    let text = format!("{}", value);
    let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let _ = SetWindowTextW(hwnd, PCWSTR(text_wide.as_ptr()));
}

unsafe fn handle_command(hwnd: HWND, control_id: i32, notification: u32, lparam: LPARAM) {
    match control_id {
        ID_CLOSE_BUTTON => {
            close_settings_window();
        }
        ID_COLOR_BUTTON => {
            show_color_picker(hwnd);
        }
        ID_LANG_COMBO => {
            if notification == CBN_SELCHANGE {
                let combo_hwnd = HWND(lparam.0 as *mut _);
                let selection = SendMessageW(combo_hwnd, CB_GETCURSEL, None, None).0 as i32;
                let new_lang = if selection == 1 { LANG_ES } else { LANG_EN };
                config::prefs_set_int(PREF_LANG, new_lang);
                // Update tray menu language
                tray::update_tray_language(new_lang == LANG_ES);
                notify_settings_changed();
            }
        }
        _ => {}
    }
}

unsafe fn handle_slider_change(slider_hwnd: HWND) {
    use windows::Win32::UI::WindowsAndMessaging::GetDlgCtrlID;

    let control_id = GetDlgCtrlID(slider_hwnd);
    let pos = SendMessageW(slider_hwnd, TBM_GETPOS, None, None).0 as i32;
    let value_hwnd = HWND(GetWindowLongPtrW(slider_hwnd, GWLP_USERDATA) as *mut _);

    if !value_hwnd.is_invalid() {
        set_value_text(value_hwnd, pos);
    }

    match control_id {
        ID_RADIUS_SLIDER => {
            // Snap to 5px increments
            let snapped = ((pos + 2) / 5) * 5;
            if snapped != pos {
                SendMessageW(
                    slider_hwnd,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(snapped as isize)),
                );
                if !value_hwnd.is_invalid() {
                    set_value_text(value_hwnd, snapped);
                }
            }
            config::prefs_set_double(PREF_RADIUS, snapped as f64);
        }
        ID_BORDER_SLIDER => {
            config::prefs_set_double(PREF_BORDER, pos as f64);
        }
        ID_TRANSP_SLIDER => {
            // Snap to 5% increments
            let snapped = ((pos + 2) / 5) * 5;
            if snapped != pos {
                SendMessageW(
                    slider_hwnd,
                    TBM_SETPOS,
                    Some(WPARAM(1)),
                    Some(LPARAM(snapped as isize)),
                );
                if !value_hwnd.is_invalid() {
                    set_value_text(value_hwnd, snapped);
                }
            }
            config::prefs_set_double(PREF_FILL_TRANSPARENCY, snapped as f64);
        }
        _ => return,
    }

    notify_settings_changed();
}

unsafe fn show_color_picker(hwnd: HWND) {
    let state = config::load_state();
    let r = (state.stroke_r * 255.0) as u32;
    let g = (state.stroke_g * 255.0) as u32;
    let b = (state.stroke_b * 255.0) as u32;
    let initial_color = COLORREF(r | (g << 8) | (b << 16));

    let mut custom_colors = [COLORREF(0xFFFFFF); 16];

    let mut cc = CHOOSECOLORW {
        lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
        hwndOwner: hwnd,
        rgbResult: initial_color,
        lpCustColors: custom_colors.as_mut_ptr(),
        Flags: CC_FULLOPEN | CC_RGBINIT,
        ..Default::default()
    };

    if ChooseColorW(&mut cc).as_bool() {
        let new_r = (cc.rgbResult.0 & 0xFF) as f64 / 255.0;
        let new_g = ((cc.rgbResult.0 >> 8) & 0xFF) as f64 / 255.0;
        let new_b = ((cc.rgbResult.0 >> 16) & 0xFF) as f64 / 255.0;

        config::prefs_set_double(PREF_STROKE_R, new_r);
        config::prefs_set_double(PREF_STROKE_G, new_g);
        config::prefs_set_double(PREF_STROKE_B, new_b);

        notify_settings_changed();
    }
}
