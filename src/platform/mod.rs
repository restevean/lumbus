//! Platform-specific implementations.
//!
//! This module contains platform-specific code that cannot be shared
//! between operating systems (FFI bindings, UI, input handling).

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
#[allow(unused_imports)]
pub use macos::*;
