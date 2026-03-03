//! Direct2D overlay rendering.
//!
//! GPU-accelerated, high-quality anti-aliased rendering with per-pixel alpha
//! transparency via UpdateLayeredWindow.
//!
//! Expensive resources (memory DC, bitmap, render target, stroke style) are
//! cached in thread-local storage and reused across frames. Only brushes
//! are created per-frame since colors can change via settings.

use std::cell::RefCell;

use windows::core::{w, BOOL};
use windows::Win32::Foundation::{COLORREF, POINT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    ID2D1DCRenderTarget, ID2D1Factory, ID2D1PathGeometry, ID2D1RenderTarget, ID2D1StrokeStyle,
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_CAP_STYLE_ROUND, D2D1_DASH_STYLE_SOLID, D2D1_ELLIPSE,
    D2D1_LINE_JOIN_ROUND, D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_DEFAULT,
    D2D1_RENDER_TARGET_USAGE_NONE, D2D1_STROKE_STYLE_PROPERTIES,
};
use windows::Win32::Graphics::DirectWrite::{
    IDWriteFactory, IDWriteFontCollection, IDWriteFontFace, DWRITE_GLYPH_OFFSET,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, SetWindowPos, UpdateLayeredWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, ULW_ALPHA,
};
use windows_numerics::{Matrix3x2, Vector2};

use crate::model::constants::*;
use crate::platform::windows::app::state::{WindowsRuntimeState, STATE};

/// Cached rendering resources to avoid per-frame allocations.
struct RenderCache {
    screen_dc: HDC,
    mem_dc: HDC,
    bitmap: HBITMAP,
    dc_render_target: ID2D1DCRenderTarget,
    stroke_style: ID2D1StrokeStyle,
    width: i32,
    height: i32,
}

impl Drop for RenderCache {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.mem_dc);
            ReleaseDC(None, self.screen_dc);
        }
    }
}

thread_local! {
    pub static D2D_FACTORY: RefCell<Option<ID2D1Factory>> = const { RefCell::new(None) };
    pub static DWRITE_FACTORY: RefCell<Option<IDWriteFactory>> = const { RefCell::new(None) };
    pub static FONT_FACE: RefCell<Option<IDWriteFontFace>> = const { RefCell::new(None) };
    static RENDER_CACHE: RefCell<Option<RenderCache>> = const { RefCell::new(None) };
}

/// Create or retrieve cached rendering resources.
///
/// Recreates resources only if screen dimensions changed.
unsafe fn get_or_create_cache(factory: &ID2D1Factory, width: i32, height: i32) -> Option<()> {
    RENDER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();

        // Reuse if dimensions match
        if let Some(ref c) = *cache {
            if c.width == width && c.height == height {
                return Some(());
            }
        }

        // Drop old cache (triggers cleanup via Drop)
        *cache = None;

        // Create new resources
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
            return None;
        }
        let bitmap = bitmap.unwrap();
        let _ = SelectObject(mem_dc, bitmap.into());

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

        let dc_render_target = factory.CreateDCRenderTarget(&rt_props).ok()?;

        // Create stroke style (never changes)
        let stroke_props = D2D1_STROKE_STYLE_PROPERTIES {
            startCap: D2D1_CAP_STYLE_ROUND,
            endCap: D2D1_CAP_STYLE_ROUND,
            dashCap: D2D1_CAP_STYLE_ROUND,
            lineJoin: D2D1_LINE_JOIN_ROUND,
            miterLimit: 1.0,
            dashStyle: D2D1_DASH_STYLE_SOLID,
            dashOffset: 0.0,
        };
        let stroke_style = factory.CreateStrokeStyle(&stroke_props, None).ok()?;

        *cache = Some(RenderCache {
            screen_dc,
            mem_dc,
            bitmap,
            dc_render_target,
            stroke_style,
            width,
            height,
        });

        Some(())
    })
}

/// Release cached rendering resources (call on app exit).
pub fn release_render_cache() {
    RENDER_CACHE.with(|cache| {
        *cache.borrow_mut() = None;
    });
}

/// Create a font face for the Arial Bold font.
pub unsafe fn create_arial_bold_font_face(
    dwrite_factory: &IDWriteFactory,
) -> Option<IDWriteFontFace> {
    // Get system font collection (windows-rs 0.62 uses output parameter)
    let mut font_collection: Option<IDWriteFontCollection> = None;
    dwrite_factory
        .GetSystemFontCollection(&mut font_collection, false)
        .ok()?;
    let font_collection = font_collection?;

    // Find Arial font family
    let mut index: u32 = 0;
    let mut exists = BOOL::default();
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
    // Direct2D handles Y coordinates correctly, just need to center
    let transform = Matrix3x2 {
        M11: 1.0,
        M12: 0.0,
        M21: 0.0,
        M22: 1.0, // No flip needed
        M31: center_x - bounds.left - glyph_width / 2.0,
        M32: center_y - bounds.top - glyph_height / 2.0,
    };

    // Create transformed geometry
    let transformed_geometry = d2d_factory
        .CreateTransformedGeometry(&path_geometry, &transform)
        .ok()?;

    // Create final path geometry from transformed geometry
    let final_geometry: ID2D1PathGeometry = d2d_factory.CreatePathGeometry().ok()?;
    let final_sink = final_geometry.Open().ok()?;

    // 0.25 is the default flattening tolerance for D2D
    transformed_geometry.Outline(None, 0.25, &final_sink).ok()?;
    final_sink.Close().ok()?;

    Some(final_geometry)
}

/// Update the overlay using Direct2D rendering.
///
/// Skips the expensive redraw if the cursor hasn't moved and
/// the display state (visibility, mode, settings) hasn't changed.
pub fn update_overlay() {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    // Read cursor position early to decide if we need to redraw
    let mut cursor = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut cursor);
    }

    let needs_redraw = STATE.with(|s| {
        let mut state = s.borrow_mut();
        let changed = state.dirty
            || cursor.x != state.last_cursor_x
            || cursor.y != state.last_cursor_y
            || state.display_mode != state.last_display_mode
            || state.visible != state.last_visible;

        if changed {
            state.last_cursor_x = cursor.x;
            state.last_cursor_y = cursor.y;
            state.last_display_mode = state.display_mode;
            state.last_visible = state.visible;
            state.dirty = false;
        }

        changed
    });

    if !needs_redraw {
        return;
    }

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

/// Draw using Direct2D and apply with UpdateLayeredWindow.
///
/// Uses cached rendering resources (DC, bitmap, render target, stroke style)
/// to avoid expensive per-frame allocations.
unsafe fn update_layered_window_d2d(
    state: &WindowsRuntimeState,
    factory: &ID2D1Factory,
    font_face: Option<&IDWriteFontFace>,
) {
    let hwnd = state.hwnd;
    let width = state.width;
    let height = state.height;

    // Ensure cached resources exist (only allocates on first call or resize)
    if get_or_create_cache(factory, width, height).is_none() {
        return;
    }

    RENDER_CACHE.with(|cache| {
        let cache = cache.borrow();
        let cache = cache.as_ref().unwrap();

        let rect = windows::Win32::Foundation::RECT {
            left: 0,
            top: 0,
            right: width,
            bottom: height,
        };

        if cache.dc_render_target.BindDC(cache.mem_dc, &rect).is_err() {
            return;
        }

        let rt: ID2D1RenderTarget = cache.dc_render_target.clone().into();

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
                            if state.lang == LANG_ES {
                                'I'
                            } else {
                                'L'
                            }
                        } else if state.lang == LANG_ES {
                            'D'
                        } else {
                            'R'
                        };

                        let font_size = 3.0 * radius;

                        let drew_outline = if let Some(ff) = font_face {
                            if let Some(letter_geom) =
                                create_letter_geometry(factory, ff, letter, font_size, x, y)
                            {
                                let fill_alpha = 1.0 - (state.fill_transparency_pct as f32 / 100.0);
                                if fill_alpha > 0.0 {
                                    let fill_color = D2D1_COLOR_F {
                                        r: state.stroke_r,
                                        g: state.stroke_g,
                                        b: state.stroke_b,
                                        a: fill_alpha,
                                    };
                                    if let Ok(fill_brush) =
                                        rt.CreateSolidColorBrush(&fill_color, None)
                                    {
                                        rt.FillGeometry(&letter_geom, &fill_brush, None);
                                    }
                                }

                                rt.DrawGeometry(
                                    &letter_geom,
                                    &brush,
                                    border,
                                    Some(&cache.stroke_style),
                                );
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        if !drew_outline {
                            let ellipse = D2D1_ELLIPSE {
                                point: Vector2::new(x, y),
                                radiusX: radius * 0.5,
                                radiusY: radius * 0.5,
                            };
                            rt.FillEllipse(&ellipse, &brush);
                        }
                    }
                    _ => {
                        let ellipse = D2D1_ELLIPSE {
                            point: Vector2::new(x, y),
                            radiusX: radius,
                            radiusY: radius,
                        };

                        let fill_alpha = 1.0 - (state.fill_transparency_pct as f32 / 100.0);
                        if fill_alpha > 0.0 {
                            let fill_color = D2D1_COLOR_F {
                                r: state.stroke_r,
                                g: state.stroke_g,
                                b: state.stroke_b,
                                a: fill_alpha,
                            };
                            if let Ok(fill_brush) = rt.CreateSolidColorBrush(&fill_color, None) {
                                rt.FillEllipse(&ellipse, &fill_brush);
                            }
                        }

                        rt.DrawEllipse(&ellipse, &brush, border, Some(&cache.stroke_style));
                    }
                }
            }
        }

        let _ = rt.EndDraw(None, None);

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
            Some(cache.screen_dc),
            Some(&pt_dst),
            Some(&size),
            Some(cache.mem_dc),
            Some(&pt_src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        // Keep window above taskbar
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    });
}
