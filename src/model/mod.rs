//! Application domain model.
//!
//! This module contains pure business logic (no FFI where possible)
//! including overlay state and configuration persistence.

pub mod app_state;
pub mod constants;
pub mod preferences;

pub use app_state::OverlayState;
pub use constants::*;
pub use preferences::{
    load_state, prefs_get_double, prefs_get_int, prefs_set_double, prefs_set_int, save_state,
};
