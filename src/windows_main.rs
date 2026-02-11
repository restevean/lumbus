//! Windows-specific entry point and application logic.
//!
//! This module contains the main application loop for Windows,
//! including overlay window creation and message processing.

use std::cell::RefCell;

use lumbus::model::constants::*;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, InvalidateRect, HDC,
    PAINTSTRUCT,
};
use windows::Win32::Graphics::GdiPlus::{
    GdipCreateFromHDC, GdipCreatePen1, GdipDeleteGraphics, GdipDeletePen, GdipDrawEllipse,
    GdipSetSmoothingMode, GdiplusShutdown, GdiplusStartup, GdiplusStartupInput, GpGraphics, GpPen,
    SmoothingModeAntiAlias, Unit,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics,
    LoadCursorW, PostQuitMessage, RegisterClassW, SetLayeredWindowAttributes, SetTimer, ShowWindow,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, IDC_ARROW, LWA_COLORKEY, MSG, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOW, WM_CREATE, WM_DESTROY,
    WM_HOTKEY, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP, WS_VISIBLE,
};

// Application-specific constants
const HOTKEY_TOGGLE: i32 = 1;
const HOTKEY_SETTINGS: i32 = 2;
const HOTKEY_QUIT: i32 = 3;
const TIMER_CURSOR: usize = 1;
const TIMER_INTERVAL_MS: u32 = 16; // ~60 FPS
const TRANSPARENT_COLOR: u32 = 0x00FF00FF; // Magenta for transparency

thread_local! {
    static STATE: RefCell<OverlayState> = RefCell::new(OverlayState::default());
}

#[allow(dead_code)] // Fields will be used as features are implemented
struct OverlayState {
    hwnd: HWND,
    radius: f64,
    border_width: f64,
    stroke_r: u8,
    stroke_g: u8,
    stroke_b: u8,
    visible: bool,
    display_mode: i32, // 0=circle, 1=L, 2=R (for click indicators)
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            hwnd: HWND::default(),
            radius: DEFAULT_DIAMETER / 2.0,
            border_width: DEFAULT_BORDER_WIDTH,
            stroke_r: 255,
            stroke_g: 255,
            stroke_b: 255,
            visible: true,
            display_mode: DISPLAY_MODE_CIRCLE,
        }
    }
}

/// Main entry point for Windows.
pub fn run() {
    if let Err(e) = run_app() {
        eprintln!("Lumbus error: {}", e);
        std::process::exit(1);
    }
}

fn run_app() -> windows::core::Result<()> {
    unsafe {
        // Initialize GDI+
        let mut gdiplus_token: usize = 0;
        let startup_input = GdiplusStartupInput {
            GdiplusVersion: 1,
            ..Default::default()
        };
        let status = GdiplusStartup(&mut gdiplus_token, &startup_input, std::ptr::null_mut());
        if status.0 != 0 {
            eprintln!("Failed to initialize GDI+: {:?}", status);
            return Err(windows::core::Error::from_win32());
        }

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
            WS_POPUP | WS_VISIBLE,
            vx,
            vy,
            vw,
            vh,
            None,
            None,
            instance,
            None,
        )?;

        // Set transparency using color key
        SetLayeredWindowAttributes(hwnd, COLORREF(TRANSPARENT_COLOR), 255, LWA_COLORKEY)?;

        // Store HWND in state
        STATE.with(|s| {
            s.borrow_mut().hwnd = hwnd;
        });

        // Register global hotkeys
        let _ = RegisterHotKey(hwnd, HOTKEY_TOGGLE, MOD_CONTROL | MOD_SHIFT, 0x41); // Ctrl+Shift+A
        let _ = RegisterHotKey(hwnd, HOTKEY_SETTINGS, MOD_CONTROL, 0xBC); // Ctrl+,
        let _ = RegisterHotKey(hwnd, HOTKEY_QUIT, MOD_CONTROL | MOD_SHIFT, 0x58); // Ctrl+Shift+X

        // Start timer for cursor tracking
        SetTimer(hwnd, TIMER_CURSOR, TIMER_INTERVAL_MS, None);

        // Show window
        let _ = ShowWindow(hwnd, SW_SHOW);

        // Message loop
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        let _ = UnregisterHotKey(hwnd, HOTKEY_TOGGLE);
        let _ = UnregisterHotKey(hwnd, HOTKEY_SETTINGS);
        let _ = UnregisterHotKey(hwnd, HOTKEY_QUIT);

        // Shutdown GDI+
        GdiplusShutdown(gdiplus_token);

        Ok(())
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_CREATE => LRESULT(0),

            WM_TIMER => {
                if wparam.0 == TIMER_CURSOR {
                    let _ = InvalidateRect(hwnd, None, true);
                }
                LRESULT(0)
            }

            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);

                STATE.with(|s| {
                    let state = s.borrow();
                    if state.visible {
                        draw_overlay(hdc, &state);
                    } else {
                        // Clear to transparent when hidden
                        clear_background(hdc, &state);
                    }
                });

                let _ = EndPaint(hwnd, &ps);
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
                        let _ = InvalidateRect(hwnd, None, true);
                    }
                    HOTKEY_SETTINGS => {
                        // TODO: Open settings window
                        eprintln!("Settings hotkey pressed (not yet implemented)");
                    }
                    HOTKEY_QUIT => {
                        PostQuitMessage(0);
                    }
                    _ => {}
                }
                LRESULT(0)
            }

            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Clear the background with transparent color.
unsafe fn clear_background(hdc: HDC, state: &OverlayState) {
    let bg_brush = CreateSolidBrush(COLORREF(TRANSPARENT_COLOR));
    let mut client_rect = RECT::default();
    let _ = GetClientRect(state.hwnd, &mut client_rect);
    FillRect(hdc, &client_rect, bg_brush);
    let _ = DeleteObject(bg_brush);
}

/// Draw the overlay circle using GDI+ for anti-aliased rendering.
unsafe fn draw_overlay(hdc: HDC, state: &OverlayState) {
    // Get cursor position
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);

    // Convert to window coordinates
    let vx = GetSystemMetrics(SM_XVIRTUALSCREEN);
    let vy = GetSystemMetrics(SM_YVIRTUALSCREEN);
    let x = (cursor.x - vx) as f32;
    let y = (cursor.y - vy) as f32;

    let radius = state.radius as f32;
    let border = state.border_width as f32;

    // Clear background with transparent color
    clear_background(hdc, state);

    // Create GDI+ Graphics object from HDC
    let mut graphics: *mut GpGraphics = std::ptr::null_mut();
    let status = GdipCreateFromHDC(hdc, &mut graphics);
    if status.0 != 0 || graphics.is_null() {
        return;
    }

    // Enable anti-aliasing
    GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);

    // Create pen with ARGB color (fully opaque white)
    let argb_color: u32 = 0xFF000000
        | ((state.stroke_r as u32) << 16)
        | ((state.stroke_g as u32) << 8)
        | (state.stroke_b as u32);

    let mut pen: *mut GpPen = std::ptr::null_mut();
    let status = GdipCreatePen1(argb_color, border, Unit(0), &mut pen); // Unit 0 = UnitWorld
    if status.0 != 0 || pen.is_null() {
        GdipDeleteGraphics(graphics);
        return;
    }

    // Draw anti-aliased circle (ellipse with equal width/height)
    let diameter = radius * 2.0;
    GdipDrawEllipse(graphics, pen, x - radius, y - radius, diameter, diameter);

    // Cleanup
    GdipDeletePen(pen);
    GdipDeleteGraphics(graphics);
}
