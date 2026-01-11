//! FFI bindings for ApplicationServices (Accessibility).
//!
//! This module provides the TCC Accessibility API to check/prompt
//! for accessibility permissions on macOS.

// === FFI Declarations ===

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;

    pub static kAXTrustedCheckOptionPrompt: *const std::ffi::c_void;
}
