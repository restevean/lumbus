//! FFI bindings for macOS frameworks.
//!
//! This module encapsulates all `extern "C"` declarations and types
//! needed to interact with Carbon, CoreText, CoreGraphics, and Cocoa.

pub mod carbon;
pub mod coretext;
pub mod coregraphics;
pub mod accessibility;
pub mod cocoa_utils;
pub mod types;

// Re-exports for convenient access
pub use carbon::*;
pub use coretext::*;
pub use coregraphics::*;
pub use accessibility::*;
pub use cocoa_utils::*;
#[allow(unused_imports)]
pub use types::*;
