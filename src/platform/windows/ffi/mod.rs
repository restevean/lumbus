//! FFI bindings for Windows APIs.
//!
//! This module encapsulates Win32 API calls for hotkeys, hooks,
//! window management, and Direct2D rendering.

pub mod hotkeys;
pub mod types;

pub use hotkeys::*;
pub use types::*;
