//! Windows-specific entry point (stub).
//!
//! This module will contain the main application loop for Windows
//! once the Windows implementation is complete.

/// Main entry point for Windows.
///
/// Currently a stub - Windows support is not yet implemented.
pub fn run() {
    eprintln!("Lumbus: Windows support is not yet implemented.");
    eprintln!("Please use the macOS version for now.");
    std::process::exit(1);
}
