//! System tray (notification area) icon for Windows.

use windows::Win32::Foundation::HWND;

/// Install the system tray icon with context menu.
pub fn install_tray_icon(_hwnd: HWND) {
    // TODO: Implement with Shell_NotifyIcon
}

/// Remove the tray icon.
pub fn remove_tray_icon() {
    // TODO: Implement
}

/// Update tray icon tooltip or menu language.
pub fn update_tray_language(_is_spanish: bool) {
    // TODO: Implement
}
