//! macOS-specific implementation using Cocoa/AppKit via objc2.
//!
//! This module contains all macOS-specific code:
//! - FFI bindings to Cocoa, Carbon, CoreText, CoreGraphics
//! - UI components (NSWindow overlays, settings, dialogs)
//! - Input handling (Carbon hotkeys, NSEvent monitors)
//! - Storage (NSUserDefaults persistence)

pub mod app;
pub mod ffi;
pub mod handlers;
pub mod input;
pub mod storage;
pub mod ui;

// Re-export commonly used items
pub use app::*;
pub use ffi::bridge;
pub use handlers::*;
pub use storage::*;
pub use ui::*;
