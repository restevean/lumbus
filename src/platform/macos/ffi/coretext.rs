//! FFI bindings for CoreText (glyph rendering).
//!
//! This module provides the CoreText API declarations needed
//! for rendering letter glyphs (L/R) when mouse buttons are clicked.

use objc2::encode::{Encoding, RefEncode};

// === Types ===

pub type CTFontRef = *const std::ffi::c_void;

/// Opaque CGPath type for correct objc2 encoding.
/// objc2 expects `^{CGPath=}` not `^v` (void pointer).
#[repr(C)]
pub struct CGPath {
    _private: [u8; 0],
}

// SAFETY: CGPath is an opaque Core Graphics type
unsafe impl RefEncode for CGPath {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("CGPath", &[]));
}

pub type CGPathRef = *const CGPath;

/// Opaque CGColor type for correct objc2 encoding.
/// objc2 expects `^{CGColor=}` not `^v` (void pointer).
#[repr(C)]
pub struct CGColor {
    _private: [u8; 0],
}

// SAFETY: CGColor is an opaque Core Graphics type
unsafe impl RefEncode for CGColor {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct("CGColor", &[]));
}

pub type CGColorRef = *const CGColor;

// === FFI Declarations ===

#[link(name = "CoreText", kind = "framework")]
extern "C" {
    pub fn CTFontCreateWithName(
        name: *const std::ffi::c_void,
        size: f64,
        matrix: *const std::ffi::c_void,
    ) -> CTFontRef;

    pub fn CTFontGetGlyphsForCharacters(
        font: CTFontRef,
        chars: *const u16,
        glyphs: *mut u16,
        count: isize,
    ) -> bool;

    pub fn CTFontCreatePathForGlyph(
        font: CTFontRef,
        glyph: u16,
        transform: *const std::ffi::c_void,
    ) -> CGPathRef;
}
