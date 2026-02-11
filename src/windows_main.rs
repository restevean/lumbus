//! Windows-specific entry point and application logic.
//!
//! Uses per-pixel alpha transparency with UpdateLayeredWindow for
//! smooth anti-aliased rendering without artifacts.

use std::cell::RefCell;

use lumbus::model::constants::*;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::Graphics::GdiPlus::{
    GdipCreateFromHDC, GdipCreatePen1, GdipDeleteGraphics, GdipDeletePen, GdipDrawEllipse,
    GdipGraphicsClear, GdipSetSmoothingMode, GdiplusShutdown, GdiplusStartup, GdiplusStartupInput,
    GpGraphics, GpPen, SmoothingModeAntiAlias, Unit,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics,
    LoadCursorW, PostQuitMessage, RegisterClassW, SetTimer, ShowWindow, TranslateMessage,
    UpdateLayeredWindow, CS_HREDRAW, CS_VREDRAW, IDC_ARROW, MSG, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOW, ULW_ALPHA, WM_CREATE,
    WM_DESTROY, WM_HOTKEY, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

// Application-specific constants
const HOTKEY_TOGGLE: i32 = 1;
const HOTKEY_SETTINGS: i32 = 2;
const HOTKEY_QUIT: i32 = 3;
const TIMER_CURSOR: usize = 1;
const TIMER_INTERVAL_MS: u32 = 16; // ~60 FPS

thread_local! {
    static STATE: RefCell<OverlayState> = RefCell::new(OverlayState::default());
}

#[allow(dead_code)]
struct OverlayState {
    hwnd: HWND,
    width: i32,
    height: i32,
    offset_x: i32,
    offset_y: i32,
    radius: f64,
    border_width: f64,
    stroke_r: u8,
    stroke_g: u8,
    stroke_b: u8,
    visible: bool,
    display_mode: i32,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            hwnd: HWND::default(),
            width: 0,
            height: 0,
            offset_x: 0,
            offset_y: 0,
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
            WS_POPUP,
            vx,
            vy,
            vw,
            vh,
            None,
            None,
            instance,
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

        // Register global hotkeys
        let _ = RegisterHotKey(hwnd, HOTKEY_TOGGLE, MOD_CONTROL | MOD_SHIFT, 0x41); // Ctrl+Shift+A
        let _ = RegisterHotKey(hwnd, HOTKEY_SETTINGS, MOD_CONTROL, 0xBC); // Ctrl+,
        let _ = RegisterHotKey(hwnd, HOTKEY_QUIT, MOD_CONTROL | MOD_SHIFT, 0x58); // Ctrl+Shift+X

        // Start timer for cursor tracking
        SetTimer(hwnd, TIMER_CURSOR, TIMER_INTERVAL_MS, None);

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
        let _ = UnregisterHotKey(hwnd, HOTKEY_TOGGLE);
        let _ = UnregisterHotKey(hwnd, HOTKEY_SETTINGS);
        let _ = UnregisterHotKey(hwnd, HOTKEY_QUIT);

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

/// Update the overlay using per-pixel alpha with UpdateLayeredWindow.
fn update_overlay() {
    STATE.with(|s| {
        let state = s.borrow();
        unsafe {
            update_layered_window(&state);
        }
    });
}

/// Draw to an ARGB bitmap and apply with UpdateLayeredWindow.
unsafe fn update_layered_window(state: &OverlayState) {
    let hwnd = state.hwnd;
    let width = state.width;
    let height = state.height;

    // Create a compatible DC
    let screen_dc = GetDC(None);
    let mem_dc = CreateCompatibleDC(screen_dc);

    // Create 32-bit ARGB bitmap
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // Top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    let bitmap = CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0);

    if bitmap.is_err() || bits.is_null() {
        ReleaseDC(None, screen_dc);
        let _ = DeleteDC(mem_dc);
        return;
    }

    let bitmap = bitmap.unwrap();
    let old_bitmap = SelectObject(mem_dc, bitmap);

    // Clear bitmap to fully transparent
    let mut graphics: *mut GpGraphics = std::ptr::null_mut();
    if GdipCreateFromHDC(mem_dc, &mut graphics).0 == 0 && !graphics.is_null() {
        // Clear to transparent (ARGB = 0x00000000)
        GdipGraphicsClear(graphics, 0x00000000);

        if state.visible {
            // Get cursor position
            let mut cursor = POINT::default();
            let _ = GetCursorPos(&mut cursor);

            // Convert to bitmap coordinates
            let x = (cursor.x - state.offset_x) as f32;
            let y = (cursor.y - state.offset_y) as f32;

            let radius = state.radius as f32;
            let border = state.border_width as f32;

            // Enable anti-aliasing
            GdipSetSmoothingMode(graphics, SmoothingModeAntiAlias);

            // Create pen with ARGB color (fully opaque)
            let argb_color: u32 = 0xFF000000
                | ((state.stroke_r as u32) << 16)
                | ((state.stroke_g as u32) << 8)
                | (state.stroke_b as u32);

            let mut pen: *mut GpPen = std::ptr::null_mut();
            if GdipCreatePen1(argb_color, border, Unit(0), &mut pen).0 == 0 && !pen.is_null() {
                // Draw anti-aliased circle
                let diameter = radius * 2.0;
                GdipDrawEllipse(graphics, pen, x - radius, y - radius, diameter, diameter);
                GdipDeletePen(pen);
            }
        }

        GdipDeleteGraphics(graphics);
    }

    // Apply the bitmap to the window using UpdateLayeredWindow
    let pt_src = POINT { x: 0, y: 0 };
    let size = SIZE {
        cx: width,
        cy: height,
    };
    let pt_dst = POINT {
        x: state.offset_x,
        y: state.offset_y,
    };

    // BLENDFUNCTION for per-pixel alpha
    let blend = windows::Win32::Graphics::Gdi::BLENDFUNCTION {
        BlendOp: 0, // AC_SRC_OVER
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: 1, // AC_SRC_ALPHA
    };

    let _ = UpdateLayeredWindow(
        hwnd,
        screen_dc,
        Some(&pt_dst),
        Some(&size),
        mem_dc,
        Some(&pt_src),
        None, // No color key
        Some(&blend),
        ULW_ALPHA,
    );

    // Cleanup
    SelectObject(mem_dc, old_bitmap);
    let _ = DeleteObject(bitmap);
    let _ = DeleteDC(mem_dc);
    ReleaseDC(None, screen_dc);
}
