//! Windows-specific implementation using Win32 API and Direct2D.
//!
//! This module contains all Windows-specific code:
//! - FFI bindings to Win32, Direct2D
//! - UI components (layered window overlays, settings, dialogs)
//! - Input handling (global hotkeys, mouse hooks)
//! - Storage (JSON config file persistence)

pub mod ffi;
pub mod storage;
pub mod ui;

// Re-export commonly used items
pub use storage::*;
pub use ui::*;
