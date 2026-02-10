//! Type definitions for Windows FFI.

/// Window handle type alias.
pub type HWND = *mut std::ffi::c_void;

/// Null window handle.
pub const NULL_HWND: HWND = std::ptr::null_mut();
