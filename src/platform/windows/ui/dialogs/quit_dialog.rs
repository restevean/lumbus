//! Quit confirmation dialog for Windows.

use crate::platform::windows::ffi::HWND;

/// Show quit confirmation dialog.
pub fn confirm_and_maybe_quit(_hwnd: HWND) {
    // TODO: Implement with MessageBox or custom dialog
}
