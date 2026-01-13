//! Quit confirmation dialog for Windows.

use windows::Win32::Foundation::HWND;

/// Show quit confirmation dialog.
///
/// Returns true if user confirmed quit, false if cancelled.
pub fn confirm_and_maybe_quit(_hwnd: HWND) -> bool {
    // TODO: Implement
    false
}
