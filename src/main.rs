#![allow(unexpected_cfgs)] // Silence cfg warnings inside objc/cocoa macros
#![windows_subsystem = "windows"] // Hide console on Windows (ignored on non-Windows)

// ============================================================================
// Platform-specific entry points
// ============================================================================

#[cfg(target_os = "macos")]
mod macos_main;

#[cfg(target_os = "windows")]
mod windows_main;

fn main() {
    #[cfg(target_os = "macos")]
    {
        lumbus::events::init_event_bus();
        macos_main::run();
    }

    #[cfg(target_os = "windows")]
    windows_main::run();

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        eprintln!("Lumbus is only supported on macOS and Windows");
        std::process::exit(1);
    }
}
