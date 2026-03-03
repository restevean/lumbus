//! Overlay view module.
//!
//! Contains the CustomView class and drawing logic for the cursor overlay.

pub mod drawing;
pub mod view;

pub use drawing::{draw_circle, draw_letter, ClickLetter, DrawParams};
pub use view::register_and_create_view;
