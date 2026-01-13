//! Helper functions for Windows application management.

use windows::Win32::Foundation::HWND;

/// Apply a function to all overlay windows.
pub fn apply_to_all_windows<F>(_f: F)
where
    F: Fn(HWND),
{
    // TODO: Implement
}

/// Check if current language is Spanish.
pub fn lang_is_es() -> bool {
    // TODO: Read from config
    false
}
