//! Input handling for Windows.

use super::ffi::HWND;

/// Install global hotkeys using RegisterHotKey.
pub unsafe fn install_hotkeys(_hwnd: HWND) {
    // TODO: Implement with RegisterHotKey
}

/// Reinstall hotkeys (called after dialogs close).
pub unsafe fn reinstall_hotkeys(_hwnd: HWND) {
    // TODO: Implement
}

/// Install mouse hook for click detection.
pub unsafe fn install_mouse_hook(_hwnd: HWND) {
    // TODO: Implement with SetWindowsHookEx
}

/// Start hotkey keepalive (not needed on Windows).
pub fn start_hotkey_keepalive() {
    // No-op on Windows - hotkeys don't need keepalive
}
