//! FFI bindings for Windows APIs.
//!
//! This module encapsulates Win32 API calls for:
//! - Window management (layered windows, transparency)
//! - Direct2D rendering
//! - Global hotkeys
//! - Mouse hooks

pub mod types;

pub use types::*;

// TODO: Add Direct2D bindings
// TODO: Add Win32 window management
