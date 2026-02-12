//! About dialog for Windows.
//!
//! Shows application name, version, and description.

use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

/// Application version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Show the About dialog.
pub fn show_about_dialog(hwnd: HWND) {
    let title = "About Lumbus";
    let message = format!(
        "Lumbus v{}\n\n\
        Mouse pointer highlighter for presentations\n\
        and screen recordings.\n\n\
        Features:\n\
        • Configurable circle overlay\n\
        • Click indicators (L/R)\n\
        • Customizable colors and size\n\n\
        © 2024 restevean\n\
        Apache-2.0 License",
        VERSION
    );

    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let message_wide: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        MessageBoxW(
            Some(hwnd),
            PCWSTR(message_wide.as_ptr()),
            PCWSTR(title_wide.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}
