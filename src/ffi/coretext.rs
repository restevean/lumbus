//! FFI bindings for CoreText (glyph rendering).
//!
//! This module provides the CoreText API declarations needed
//! for rendering letter glyphs (L/R) when mouse buttons are clicked.

// === Types ===

pub type CTFontRef = *const std::ffi::c_void;
pub type CGPathRef = *const std::ffi::c_void;

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
