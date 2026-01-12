//! User interface module.
//!
//! This module contains UI components:
//!
//! ## overlay/
//! - drawing.rs: Circle and letter drawing logic
//!
//! ## dialogs/
//! - quit_dialog.rs: confirm_and_maybe_quit
//!
//! ## settings/
//! - window.rs: open_settings_window, close_settings_window
//!
//! ## status_bar.rs
//! - Status bar icon with dropdown menu

pub mod overlay;
pub mod dialogs;
pub mod settings;
pub mod status_bar;

pub use overlay::{DrawParams, ClickLetter, draw_circle, draw_letter};
pub use dialogs::confirm_and_maybe_quit;
pub use settings::{close_settings_window, open_settings_window};
pub use status_bar::{install_status_bar, update_status_bar_language};
