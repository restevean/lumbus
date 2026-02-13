//! FFI bindings for ApplicationServices (Accessibility).
//!
//! This module provides the TCC Accessibility API to check/prompt
//! for accessibility permissions on macOS.

use super::coregraphics::{
    kCFBooleanTrue, kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks,
    CFDictionaryCreate, CFRelease,
};

// === FFI Declarations ===

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;

    pub static kAXTrustedCheckOptionPrompt: *const std::ffi::c_void;
}

/// Ensure accessibility permissions are granted, prompting the user if needed.
///
/// This function triggers the macOS accessibility permission dialog if
/// the app doesn't have accessibility access yet.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn ensure_accessibility_prompt() {
    // Create CFDictionary with kAXTrustedCheckOptionPrompt = true
    let keys = [kAXTrustedCheckOptionPrompt];
    let values = [kCFBooleanTrue];

    let dict = CFDictionaryCreate(
        std::ptr::null(), // default allocator
        keys.as_ptr() as *const _,
        values.as_ptr() as *const _,
        1, // one key-value pair
        kCFTypeDictionaryKeyCallBacks,
        kCFTypeDictionaryValueCallBacks,
    );

    let _trusted: bool = AXIsProcessTrustedWithOptions(dict);

    // Clean up
    if !dict.is_null() {
        CFRelease(dict);
    }
    // We ignore the boolean: if not trusted this triggers the system prompt.
}
