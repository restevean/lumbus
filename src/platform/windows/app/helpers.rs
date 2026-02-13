//! Helper functions for Windows application management.

use super::super::ffi::HWND;

/// Apply a function to all overlay windows.
pub fn apply_to_all_windows<F>(_f: F)
where
    F: Fn(HWND),
{
    // TODO: Implement - iterate over overlay windows
}

/// Check if current language is Spanish.
pub fn lang_is_es() -> bool {
    // TODO: Read from config
    false
}

/// Sync visual preferences to all overlay windows.
pub fn sync_visual_prefs_to_all_windows(_hwnd: HWND) {
    // TODO: Implement
}
