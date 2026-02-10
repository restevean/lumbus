//! Event dispatcher for Windows.

use super::super::ffi::HWND;

/// Dispatch pending events from the event bus.
///
/// # Safety
/// Must be called from the main thread.
pub unsafe fn dispatch_events(
    _hwnd: HWND,
    _open_settings_fn: unsafe fn(HWND),
    _confirm_quit_fn: unsafe fn(HWND),
    _reinstall_hotkeys_fn: unsafe fn(HWND),
) {
    // TODO: Implement event dispatching for Windows
}
