//! FFI bindings for macOS frameworks.
//!
//! This module encapsulates all `extern "C"` declarations and types
//! needed to interact with Carbon, CoreText, CoreGraphics, and Cocoa.
//!
//! ## Migration Note
//! The `bridge` module provides compatibility types for migrating from
//! the deprecated `cocoa`/`objc`/`block` crates to `objc2`.

pub mod accessibility;
pub mod bridge;
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

// Bridge re-exports (for migration)
pub use bridge::{
    autoreleasepool, get_class, id, nil, nsstring, nsstring_id, NSApp, ObjectExt, NO, YES,
};
