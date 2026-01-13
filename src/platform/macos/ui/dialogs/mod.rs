//! Dialog windows.
//!
//! This module contains dialog windows like quit confirmation and help overlay.

pub mod help_overlay;
pub mod quit_dialog;

pub use help_overlay::show_help_overlay;
pub use quit_dialog::confirm_and_maybe_quit;
