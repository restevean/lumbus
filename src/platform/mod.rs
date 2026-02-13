//! Platform-specific implementations.
//!
//! This module contains platform-specific code for macOS and Windows.
//! Each platform has its own submodule with implementations of:
//! - FFI bindings
//! - UI components (overlay, settings, dialogs)
//! - Input handling (hotkeys, mouse, keyboard)
//! - Storage (preferences/config persistence)

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

// Re-export the current platform's modules for convenience
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "windows")]
pub use windows::*;
