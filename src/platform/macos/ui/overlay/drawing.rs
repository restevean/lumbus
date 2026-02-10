//! Drawing functions for the overlay view.
//!
//! This module contains the pure drawing logic extracted from the view's
//! draw_rect method. While still using unsafe FFI calls to Cocoa, the
//! logic is isolated and easier to understand/maintain.

use crate::platform::macos::ffi::bridge::{get_class, id, msg_send, NSPoint, NSRect, NSSize};

use crate::clamp;
use crate::platform::macos::ffi::{
    CFRelease, CGPathRef, CGPathRelease, CTFontCreatePathForGlyph, CTFontCreateWithName,
    CTFontGetGlyphsForCharacters, CTFontRef,
};

/// Drawing parameters extracted from view ivars.
///
/// This struct groups all parameters needed for drawing operations,
/// making function signatures cleaner and reducing coupling.
#[derive(Clone, Copy)]
pub struct DrawParams {
    /// Center point in view coordinates
    pub center: NSPoint,
    /// Circle radius in pixels
    pub radius: f64,
    /// Border/stroke width in pixels
    pub border_width: f64,
    /// Stroke color red component (0.0 - 1.0)
    pub stroke_r: f64,
    /// Stroke color green component (0.0 - 1.0)
    pub stroke_g: f64,
    /// Stroke color blue component (0.0 - 1.0)
    pub stroke_b: f64,
    /// Stroke color alpha component (0.0 - 1.0)
    pub stroke_a: f64,
    /// Fill transparency percentage (0 = opaque, 100 = fully transparent)
    pub fill_transparency: f64,
}

impl DrawParams {
    /// Calculate the fill alpha based on stroke alpha and transparency setting.
    #[inline]
    pub fn fill_alpha(&self) -> f64 {
        self.stroke_a * (1.0 - clamp(self.fill_transparency, 0.0, 100.0) / 100.0)
    }
}

/// Draw a circle at the specified position.
///
/// # Safety
///
/// Must be called from the main thread within a valid drawing context.
pub unsafe fn draw_circle(params: &DrawParams) {
    let ns_color = get_class("NSColor");
    let ns_bezier = get_class("NSBezierPath");

    let rect = NSRect::new(
        NSPoint::new(
            params.center.x - params.radius,
            params.center.y - params.radius,
        ),
        NSSize::new(params.radius * 2.0, params.radius * 2.0),
    );

    let circle: id = msg_send![ns_bezier, bezierPathWithOvalInRect: rect];

    // Fill
    let fill_alpha = params.fill_alpha();
    if fill_alpha > 0.0 {
        let fill: id = msg_send![
            ns_color,
            colorWithCalibratedRed: params.stroke_r,
            green: params.stroke_g,
            blue: params.stroke_b,
            alpha: fill_alpha
        ];
        let _: () = msg_send![fill, set];
        let _: () = msg_send![circle, fill];
    }

    // Stroke
    let stroke: id = msg_send![
        ns_color,
        colorWithCalibratedRed: params.stroke_r,
        green: params.stroke_g,
        blue: params.stroke_b,
        alpha: params.stroke_a
    ];
    let _: () = msg_send![stroke, set];
    let _: () = msg_send![circle, setLineWidth: params.border_width];
    let _: () = msg_send![circle, stroke];
}

/// The letter to draw when mouse button is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickLetter {
    /// Left mouse button - draws "L"
    Left,
    /// Right mouse button - draws "R"
    Right,
}

impl ClickLetter {
    /// Get the character to render based on language.
    ///
    /// - English (es=false): "L" for left, "R" for right
    /// - Spanish (es=true): "I" for izquierdo, "D" for derecho
    fn as_char(self, es: bool) -> char {
        match (self, es) {
            (ClickLetter::Left, false) => 'L',
            (ClickLetter::Left, true) => 'I',
            (ClickLetter::Right, false) => 'R',
            (ClickLetter::Right, true) => 'D',
        }
    }
}

/// Draw a letter (L/R or I/D) at the specified position.
///
/// Uses CoreText for glyph rendering, producing high-quality
/// vector letters that scale with the radius setting.
///
/// The letter displayed depends on the language:
/// - English (es=false): "L" for left click, "R" for right click
/// - Spanish (es=true): "I" for left click (izquierdo), "D" for right click (derecho)
///
/// # Safety
///
/// Must be called from the main thread within a valid drawing context.
pub unsafe fn draw_letter(params: &DrawParams, letter: ClickLetter, es: bool) {
    let ns_color = get_class("NSColor");
    let ns_bezier = get_class("NSBezierPath");
    let ns_affine = get_class("NSAffineTransform");
    let font_class = get_class("NSFont");

    // Letter height = 1.5 x diameter (3 x radius)
    let target_letter_height = 3.0 * params.radius;

    let font: id = msg_send![font_class, boldSystemFontOfSize: target_letter_height];
    let font_name: id = msg_send![font, fontName];

    let ct_font: CTFontRef = CTFontCreateWithName(
        font_name as *const _,
        target_letter_height,
        std::ptr::null(),
    );

    let ch_u16: u16 = letter.as_char(es) as u16;
    let mut glyph: u16 = 0;

    let mapped =
        CTFontGetGlyphsForCharacters(ct_font, &ch_u16 as *const u16, &mut glyph as *mut u16, 1);

    if !mapped || glyph == 0 {
        CFRelease(ct_font as *const _);
        return;
    }

    let cg_path: CGPathRef = CTFontCreatePathForGlyph(ct_font, glyph, std::ptr::null());
    if cg_path.is_null() {
        CFRelease(ct_font as *const _);
        return;
    }

    let path: id = msg_send![ns_bezier, bezierPathWithCGPath: cg_path];

    // Center the letter on the cursor position
    let pbounds: NSRect = msg_send![path, bounds];
    let mid_x = pbounds.origin.x + pbounds.size.width / 2.0;
    let mid_y = pbounds.origin.y + pbounds.size.height / 2.0;

    let transform: id = msg_send![ns_affine, transform];
    let dx = params.center.x - mid_x;
    let dy = params.center.y - mid_y;
    let _: () = msg_send![transform, translateXBy: dx, yBy: dy];
    let _: () = msg_send![path, transformUsingAffineTransform: transform];

    // Round line joins for smoother appearance
    let _: () = msg_send![path, setLineJoinStyle: 1u64];

    // Fill (same style as circle)
    let fill_alpha = params.fill_alpha();
    if fill_alpha > 0.0 {
        let fill: id = msg_send![
            ns_color,
            colorWithCalibratedRed: params.stroke_r,
            green: params.stroke_g,
            blue: params.stroke_b,
            alpha: fill_alpha
        ];
        let _: () = msg_send![fill, set];
        let _: () = msg_send![path, fill];
    }

    // Stroke
    let stroke: id = msg_send![
        ns_color,
        colorWithCalibratedRed: params.stroke_r,
        green: params.stroke_g,
        blue: params.stroke_b,
        alpha: params.stroke_a
    ];
    let _: () = msg_send![stroke, set];
    let _: () = msg_send![path, setLineWidth: params.border_width];
    let _: () = msg_send![path, stroke];

    CGPathRelease(cg_path);
    CFRelease(ct_font as *const _);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_params_fill_alpha_zero_transparency() {
        let params = DrawParams {
            center: NSPoint::new(0.0, 0.0),
            radius: 50.0,
            border_width: 2.0,
            stroke_r: 1.0,
            stroke_g: 0.0,
            stroke_b: 0.0,
            stroke_a: 1.0,
            fill_transparency: 0.0,
        };
        assert!((params.fill_alpha() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_draw_params_fill_alpha_full_transparency() {
        let params = DrawParams {
            center: NSPoint::new(0.0, 0.0),
            radius: 50.0,
            border_width: 2.0,
            stroke_r: 1.0,
            stroke_g: 0.0,
            stroke_b: 0.0,
            stroke_a: 1.0,
            fill_transparency: 100.0,
        };
        assert!(params.fill_alpha().abs() < 0.001);
    }

    #[test]
    fn test_draw_params_fill_alpha_half_transparency() {
        let params = DrawParams {
            center: NSPoint::new(0.0, 0.0),
            radius: 50.0,
            border_width: 2.0,
            stroke_r: 1.0,
            stroke_g: 0.0,
            stroke_b: 0.0,
            stroke_a: 1.0,
            fill_transparency: 50.0,
        };
        assert!((params.fill_alpha() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_click_letter_as_char() {
        // English
        assert_eq!(ClickLetter::Left.as_char(false), 'L');
        assert_eq!(ClickLetter::Right.as_char(false), 'R');
        // Spanish
        assert_eq!(ClickLetter::Left.as_char(true), 'I');
        assert_eq!(ClickLetter::Right.as_char(true), 'D');
    }
}
