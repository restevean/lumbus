//! User interface module.
//!
//! This module contains UI components:
//!
//! ## dialogs/
//! - quit_dialog.rs: confirm_and_maybe_quit
//!
//! ## TODO: overlay/
//! - view_class.rs: CustomView class registration and ivars
//! - view_methods.rs: extern "C" methods for the view
//! - drawing.rs: Circle and letter drawing logic
//! - window.rs: make_window_for_screen
//!
//! ## TODO: settings/
//! - window.rs: open_settings_window, close_settings_window
//! - controls.rs: mk_label, mk_value_label, mk_slider helpers
//! - actions.rs: Slider/color callbacks

pub mod dialogs;

pub use dialogs::{confirm_and_maybe_quit, OnDialogClose};
