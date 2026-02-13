//! FFI bindings for Windows APIs.
//!
//! This module encapsulates Win32 API calls for:
//! - Window management (layered windows, transparency)
//! - GDI rendering (circles, text)
//! - Global hotkeys
//! - Mouse position tracking

pub mod types;

pub use types::*;

// Re-exports and helpers will be added as needed
