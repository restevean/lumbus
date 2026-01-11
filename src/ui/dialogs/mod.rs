//! Dialog windows.
//!
//! This module contains dialog windows like quit confirmation.

pub mod quit_dialog;

pub use quit_dialog::{confirm_and_maybe_quit, OnDialogClose};
