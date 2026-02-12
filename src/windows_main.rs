//! Windows-specific entry point and application logic.
//!
//! Uses Direct2D for GPU-accelerated, high-quality anti-aliased rendering
//! with per-pixel alpha transparency via UpdateLayeredWindow.

use std::cell::RefCell;
use std::sync::atomic::{AtomicIsize, Ordering};

use lumbus::model::constants::*;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_MATRIX_3X2_F,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory, ID2D1PathGeometry, ID2D1RenderTarget,
    ID2D1StrokeStyle, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_CAP_STYLE_ROUND,
    D2D1_DASH_STYLE_SOLID, D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_LINE_JOIN_ROUND,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE,
    D2D1_STROKE_STYLE_PROPERTIES,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteFontFace, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_SIMULATIONS_BOLD, DWRITE_GLYPH_OFFSET,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, MOD_CONTROL, MOD_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetMessageW,
    GetSystemMetrics, LoadCursorW, PostQuitMessage, RegisterClassW, SetTimer, SetWindowsHookExW,
    ShowWindow, TranslateMessage, UnhookWindowsHookEx, UpdateLayeredWindow, CS_HREDRAW, CS_VREDRAW,
    HHOOK, IDC_ARROW, MSG, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN, SW_SHOW, ULW_ALPHA, WH_MOUSE_LL, WM_CREATE, WM_DESTROY, WM_HOTKEY,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_TIMER, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

// For Vector2 in D2D1_ELLIPSE
use windows_numerics::Vector2;

// Application-specific constants
const HOTKEY_TOGGLE: i32 = 1;
const HOTKEY_SETTINGS: i32 = 2;
const HOTKEY_QUIT: i32 = 3;
const TIMER_CURSOR: usize = 1;
const TIMER_INTERVAL_MS: u32 = 16; // ~60 FPS

// Global mouse hook handle (must be static for the hook callback)
static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);

thread_local! {
    static STATE: RefCell<OverlayState> = RefCell::new(OverlayState::default());
    static D2D_FACTORY: RefCell<Option<ID2D1Factory>> = const { RefCell::new(None) };
    static DWRITE_FACTORY: RefCell<Option<IDWriteFactory>> = const { RefCell::new(None) };
    static FONT_FACE: RefCell<Option<IDWriteFontFace>> = const { RefCell::new(None) };
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
    stroke_r: f32,
    stroke_g: f32,
    stroke_b: f32,
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
            radius: 50.0,
            border_width: 4.0,
            stroke_r: 1.0,
            stroke_g: 1.0,
            stroke_b: 1.0,
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

/// Create a font face for the Arial Bold font.
unsafe fn create_arial_bold_font_face(dwrite_factory: &IDWriteFactory) -> Option<IDWriteFontFace> {
    // Get system font collection
    let font_collection = dwrite_factory.GetSystemFontCollection(false).ok()?;

    // Find Arial font family
    let mut index: u32 = 0;
    let mut exists = windows::Win32::Foundation::BOOL::default();
    font_collection
        .FindFamilyName(w!("Arial"), &mut index, &mut exists)
        .ok()?;

    if !exists.as_bool() {
        return None;
    }

    // Get the font family
    let font_family = font_collection.GetFontFamily(index).ok()?;

    // Get bold font from the family
    let font = font_family
        .GetFirstMatchingFont(
            windows::Win32::Graphics::DirectWrite::DWRITE_FONT_WEIGHT_BOLD,
            windows::Win32::Graphics::DirectWrite::DWRITE_FONT_STRETCH_NORMAL,
            windows::Win32::Graphics::DirectWrite::DWRITE_FONT_STYLE_NORMAL,
        )
        .ok()?;

    // Create font face
    font.CreateFontFace().ok()
}

fn run_app() -> windows::core::Result<()> {
    unsafe {
        // Initialize COM
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;

        // Create Direct2D factory
        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        D2D_FACTORY.with(|f| {
            *f.borrow_mut() = Some(factory);
        });

        // Create DirectWrite factory for text rendering
        let dwrite_factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;

        // Create and cache font face for letter rendering
        if let Some(font_face) = create_arial_bold_font_face(&dwrite_factory) {
            FONT_FACE.with(|f| {
                *f.borrow_mut() = Some(font_face);
            });
        }

        DWRITE_FACTORY.with(|f| {
            *f.borrow_mut() = Some(dwrite_factory);
        });

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

        // Install low-level mouse hook for click detection
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), None, 0)?;
        MOUSE_HOOK.store(hook.0 as isize, Ordering::SeqCst);

        // Register global hotkeys
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_TOGGLE, MOD_CONTROL | MOD_SHIFT, 0x41);
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_SETTINGS, MOD_CONTROL, 0xBC);
        let _ = RegisterHotKey(Some(hwnd), HOTKEY_QUIT, MOD_CONTROL | MOD_SHIFT, 0x58);

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

        FONT_FACE.with(|f| {
            *f.borrow_mut() = None;
        });
        DWRITE_FACTORY.with(|f| {
            *f.borrow_mut() = None;
        });
        D2D_FACTORY.with(|f| {
            *f.borrow_mut() = None;
        });

        CoUninitialize();

        Ok(())
    }
}

/// Low-level mouse hook procedure for detecting mouse button presses.
extern "system" fn mouse_hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if ncode >= 0 {
            let new_mode = match wparam.0 as u32 {
                WM_LBUTTONDOWN => Some(DISPLAY_MODE_LEFT),
                WM_RBUTTONDOWN => Some(DISPLAY_MODE_RIGHT),
                WM_LBUTTONUP | WM_RBUTTONUP => Some(DISPLAY_MODE_CIRCLE),
                _ => None,
            };

            if let Some(mode) = new_mode {
                STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    state.display_mode = mode;
                });
            }
        }

        let hook = MOUSE_HOOK.load(Ordering::SeqCst);
        CallNextHookEx(Some(HHOOK(hook as *mut _)), ncode, wparam, lparam)
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

/// Update the overlay using Direct2D rendering.
fn update_overlay() {
    STATE.with(|s| {
        let state = s.borrow();
        D2D_FACTORY.with(|d2d_f| {
            FONT_FACE.with(|ff| {
                if let Some(d2d_factory) = d2d_f.borrow().as_ref() {
                    let font_face = ff.borrow();
                    unsafe {
                        update_layered_window_d2d(&state, d2d_factory, font_face.as_ref());
                    }
                }
            });
        });
    });
}

/// Create outlined letter geometry using DirectWrite glyph outlines.
unsafe fn create_letter_geometry(
    d2d_factory: &ID2D1Factory,
    font_face: &IDWriteFontFace,
    letter: char,
    font_size: f32,
    center_x: f32,
    center_y: f32,
) -> Option<ID2D1PathGeometry> {
    // Get glyph index for the letter
    let code_point = letter as u32;
    let mut glyph_index: u16 = 0;
    font_face
        .GetGlyphIndices(&code_point, 1, &mut glyph_index)
        .ok()?;

    if glyph_index == 0 {
        return None;
    }

    // Create path geometry for the glyph outline
    let path_geometry: ID2D1PathGeometry = d2d_factory.CreatePathGeometry().ok()?;
    let sink = path_geometry.Open().ok()?;

    // Get glyph outline at the specified font size
    let glyph_advance: f32 = 0.0;
    let glyph_offset = DWRITE_GLYPH_OFFSET::default();

    font_face
        .GetGlyphRunOutline(
            font_size,
            &glyph_index,
            Some(&glyph_advance),
            Some(&glyph_offset),
            1,
            false, // not sideways
            false, // not right-to-left
            &sink,
        )
        .ok()?;

    sink.Close().ok()?;

    // Get bounds to center the geometry
    let bounds = path_geometry.GetBounds(None).ok()?;
    let glyph_width = bounds.right - bounds.left;
    let glyph_height = bounds.bottom - bounds.top;

    // Create transform to center the glyph on the cursor
    // Font glyphs are Y-up, so we need to flip and translate
    let transform = D2D_MATRIX_3X2_F {
        M11: 1.0,
        M12: 0.0,
        M21: 0.0,
        M22: -1.0, // Flip Y axis
        M31: center_x - bounds.left - glyph_width / 2.0,
        M32: center_y + glyph_height / 2.0 + bounds.top,
    };

    // Create transformed geometry
    let transformed_geometry = d2d_factory
        .CreateTransformedGeometry(&path_geometry, &transform)
        .ok()?;

    // Create final path geometry from transformed geometry
    let final_geometry: ID2D1PathGeometry = d2d_factory.CreatePathGeometry().ok()?;
    let final_sink = final_geometry.Open().ok()?;

    transformed_geometry.Outline(None, &final_sink).ok()?;
    final_sink.Close().ok()?;

    Some(final_geometry)
}

/// Draw using Direct2D and apply with UpdateLayeredWindow.
unsafe fn update_layered_window_d2d(
    state: &OverlayState,
    factory: &ID2D1Factory,
    font_face: Option<&IDWriteFontFace>,
) {
    let hwnd = state.hwnd;
    let width = state.width;
    let height = state.height;

    // Create a compatible DC and ARGB bitmap
    let screen_dc = GetDC(None);
    let mem_dc = CreateCompatibleDC(Some(screen_dc));

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // Top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    let bitmap = CreateDIBSection(Some(mem_dc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0);

    if bitmap.is_err() || bits.is_null() {
        ReleaseDC(None, screen_dc);
        let _ = DeleteDC(mem_dc);
        return;
    }

    let bitmap = bitmap.unwrap();
    let old_bitmap = SelectObject(mem_dc, bitmap.into());

    // Create DC render target
    let rt_props = D2D1_RENDER_TARGET_PROPERTIES {
        r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        usage: D2D1_RENDER_TARGET_USAGE_NONE,
        minLevel: Default::default(),
    };

    // Create stroke style with round caps
    let stroke_props = D2D1_STROKE_STYLE_PROPERTIES {
        startCap: D2D1_CAP_STYLE_ROUND,
        endCap: D2D1_CAP_STYLE_ROUND,
        dashCap: D2D1_CAP_STYLE_ROUND,
        lineJoin: D2D1_LINE_JOIN_ROUND,
        miterLimit: 1.0,
        dashStyle: D2D1_DASH_STYLE_SOLID,
        dashOffset: 0.0,
    };
    let stroke_style: Option<ID2D1StrokeStyle> =
        factory.CreateStrokeStyle(&stroke_props, None).ok();

    let render_target: Result<ID2D1DCRenderTarget, _> = factory.CreateDCRenderTarget(&rt_props);

    if let Ok(dc_rt) = render_target {
        let rect = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };

        if dc_rt.BindDC(mem_dc, &rect).is_ok() {
            let rt: ID2D1RenderTarget = dc_rt.into();

            rt.BeginDraw();

            // Clear to transparent
            rt.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            if state.visible {
                let mut cursor = POINT::default();
                let _ = GetCursorPos(&mut cursor);

                let x = (cursor.x - state.offset_x) as f32;
                let y = (cursor.y - state.offset_y) as f32;

                let radius = state.radius as f32;
                let border = state.border_width as f32;

                rt.SetAntialiasMode(D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);

                let color = D2D1_COLOR_F {
                    r: state.stroke_r,
                    g: state.stroke_g,
                    b: state.stroke_b,
                    a: 1.0,
                };

                if let Ok(brush) = rt.CreateSolidColorBrush(&color, None) {
                    match state.display_mode {
                        DISPLAY_MODE_LEFT | DISPLAY_MODE_RIGHT => {
                            let letter = if state.display_mode == DISPLAY_MODE_LEFT {
                                'L'
                            } else {
                                'R'
                            };

                            // Font size = 3 * radius (matching macOS: letter height = 1.5 * diameter)
                            let font_size = 3.0 * radius;

                            // Try to create outlined letter geometry
                            let drew_outline = if let Some(ff) = font_face {
                                if let Some(letter_geom) =
                                    create_letter_geometry(factory, ff, letter, font_size, x, y)
                                {
                                    // Draw outlined letter (stroke only, no fill)
                                    rt.DrawGeometry(
                                        &letter_geom,
                                        &brush,
                                        border,
                                        stroke_style.as_ref(),
                                    );
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            // Fallback: draw a simple indicator if outline failed
                            if !drew_outline {
                                // Draw a small filled circle as fallback
                                let ellipse = D2D1_ELLIPSE {
                                    point: Vector2::new(x, y),
                                    radiusX: radius * 0.5,
                                    radiusY: radius * 0.5,
                                };
                                rt.FillEllipse(&ellipse, &brush);
                            }
                        }
                        _ => {
                            // Draw circle (default)
                            let ellipse = D2D1_ELLIPSE {
                                point: Vector2::new(x, y),
                                radiusX: radius,
                                radiusY: radius,
                            };

                            rt.DrawEllipse(&ellipse, &brush, border, stroke_style.as_ref());
                        }
                    }
                }
            }

            let _ = rt.EndDraw(None, None);
        }
    }

    // Apply to window
    let pt_src = POINT { x: 0, y: 0 };
    let size = SIZE {
        cx: width,
        cy: height,
    };
    let pt_dst = POINT {
        x: state.offset_x,
        y: state.offset_y,
    };

    let blend = windows::Win32::Graphics::Gdi::BLENDFUNCTION {
        BlendOp: 0,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: 1,
    };

    let _ = UpdateLayeredWindow(
        hwnd,
        Some(screen_dc),
        Some(&pt_dst),
        Some(&size),
        Some(mem_dc),
        Some(&pt_src),
        COLORREF(0),
        Some(&blend),
        ULW_ALPHA,
    );

    // Cleanup
    SelectObject(mem_dc, old_bitmap);
    let _ = DeleteObject(bitmap.into());
    let _ = DeleteDC(mem_dc);
    ReleaseDC(None, screen_dc);
}
