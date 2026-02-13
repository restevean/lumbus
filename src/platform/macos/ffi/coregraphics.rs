//! FFI bindings for CoreGraphics and CoreFoundation.
//!
//! This module provides the CoreGraphics API for path management
//! and CoreFoundation utilities (CFRelease, CFDictionary, etc.).

use super::coretext::CGPathRef;

// === FFI Declarations - CoreGraphics ===

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub fn CGPathRelease(path: CGPathRef);
}

// === FFI Declarations - CoreFoundation ===

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub fn CFRelease(obj: *const std::ffi::c_void);

    pub fn CFAbsoluteTimeGetCurrent() -> f64;

    pub fn CFDictionaryCreate(
        allocator: *const std::ffi::c_void,
        keys: *const *const std::ffi::c_void,
        values: *const *const std::ffi::c_void,
        numValues: isize,
        keyCallBacks: *const std::ffi::c_void,
        valueCallBacks: *const std::ffi::c_void,
    ) -> *const std::ffi::c_void;

    pub static kCFBooleanTrue: *const std::ffi::c_void;
    pub static kCFTypeDictionaryKeyCallBacks: *const std::ffi::c_void;
    pub static kCFTypeDictionaryValueCallBacks: *const std::ffi::c_void;
}
