//! User interface module.
//!
//! This module contains UI components:
//!
//! ## dialogs/
//! - quit_dialog.rs: confirm_and_maybe_quit
//!
//! ## settings/
//! - window.rs: open_settings_window, close_settings_window, ensure_hotkey_menu
//!
//! ## TODO: overlay/
//! - view_class.rs: CustomView class registration and ivars
//! - view_methods.rs: extern "C" methods for the view
//! - drawing.rs: Circle and letter drawing logic
//! - window.rs: make_window_for_screen

pub mod dialogs;
pub mod settings;

pub use dialogs::confirm_and_maybe_quit;
pub use settings::{close_settings_window, open_settings_window};
