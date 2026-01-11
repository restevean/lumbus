//! Application domain model.
//!
//! This module contains pure business logic (no FFI where possible)
//! including overlay state and configuration persistence.

pub mod constants;
pub mod app_state;
pub mod preferences;

pub use constants::*;
pub use app_state::OverlayState;
pub use preferences::{
    prefs_get_double, prefs_set_double,
    prefs_get_int, prefs_set_int,
    load_state, save_state,
};
