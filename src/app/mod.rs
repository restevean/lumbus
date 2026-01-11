//! Application-wide shared functionality.
//!
//! This module contains helpers that are used across multiple modules
//! (input, ui) and operate on the global application state.

pub mod helpers;

pub use helpers::{apply_to_all_views, lang_is_es, redraw_all_views, sync_visual_prefs_to_all_views};
