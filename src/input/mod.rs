//! Input handling module.
//!
//! This module contains input handlers for:
//! - hotkeys.rs: Carbon hotkey registration and handling
//! - mouse_monitors.rs: Global mouse event monitors (TODO)
//! - keyboard_monitors.rs: Local keyboard monitors (TODO)
//! - observers.rs: System observers (wake, space change, termination) (TODO)

pub mod hotkeys;

pub use hotkeys::{install_hotkeys, uninstall_hotkeys, reinstall_hotkeys, HotkeyHandler};
