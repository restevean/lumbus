//! FFI bindings for macOS frameworks.
//!
//! This module encapsulates all `extern "C"` declarations and types
//! needed to interact with Carbon, CoreText, CoreGraphics, and Cocoa.

pub mod accessibility;
pub mod carbon;
pub mod cocoa_utils;
pub mod coregraphics;
pub mod coretext;
pub mod types;

// Re-exports for convenient access
pub use accessibility::*;
pub use carbon::*;
pub use cocoa_utils::*;
pub use coregraphics::*;
pub use coretext::*;
#[allow(unused_imports)]
pub use types::*;
