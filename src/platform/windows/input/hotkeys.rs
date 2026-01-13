//! Global hotkey registration for Windows.
//!
//! Uses RegisterHotKey and handles WM_HOTKEY messages.

use windows::Win32::Foundation::HWND;

/// Install global hotkeys for the application.
///
/// Registers:
/// - Ctrl+A: Toggle overlay
/// - Ctrl+Shift+H: Show help
/// - Ctrl+Shift+X: Quit
/// - Ctrl+, : Settings
pub fn install_hotkeys(_hwnd: HWND) -> bool {
    // TODO: Implement
    true
}

/// Uninstall all registered hotkeys.
pub fn uninstall_hotkeys(_hwnd: HWND) {
    // TODO: Implement
}

/// Reinstall hotkeys (e.g., after dialog closes).
pub fn reinstall_hotkeys(_hwnd: HWND) {
    // TODO: Implement
}
