//! Input handling module.
//!
//! This module will contain input handlers once UI is decoupled:
//! - hotkeys.rs: Carbon hotkey registration and handling
//! - mouse_monitors.rs: Global mouse event monitors
//! - keyboard_monitors.rs: Local keyboard monitors
//! - observers.rs: System observers (wake, space change, termination)
//!
//! Currently these functions remain in main.rs due to coupling with
//! UI functions (open_settings_window, confirm_and_maybe_quit) and
//! global helpers (apply_to_all_views).
//!
//! TODO: After Phase 5 (UI modularization), move:
//! - hotkey_event_handler
//! - install_hotkeys / uninstall_hotkeys / reinstall_hotkeys
//! - install_mouse_monitors
//! - install_local_ctrl_a_monitor
//! - install_termination_observer
//! - start_hotkey_keepalive
//! - install_wakeup_space_observers

// Placeholder - functions will be moved here after UI decoupling
