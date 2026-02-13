//! Application domain model.
//!
//! This module contains pure business logic (no FFI dependencies)
//! including overlay state definition and configuration constants.
//!
//! Platform-specific persistence is in `platform::{macos,windows}::storage`.

pub mod app_state;
pub mod constants;

pub use app_state::OverlayState;
pub use constants::*;
