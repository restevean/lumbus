//! Input handling module.
//!
//! This module contains input handlers for:
//! - hotkeys.rs: Carbon hotkey registration and handling
//! - observers.rs: System observers (wake, space change, termination)
//! - keyboard_monitors.rs: Local keyboard monitors (Ctrl+A backup)
//! - mouse_monitors.rs: Global mouse event monitors

pub mod hotkeys;
pub mod keyboard_monitors;
pub mod mouse_monitors;
pub mod observers;

pub use hotkeys::{hotkey_event_handler, install_hotkeys, reinstall_hotkeys};
pub use keyboard_monitors::install_local_ctrl_a_monitor;
pub use mouse_monitors::install_mouse_monitors;
pub use observers::{
    install_termination_observer, install_wakeup_space_observers, start_hotkey_keepalive,
};
