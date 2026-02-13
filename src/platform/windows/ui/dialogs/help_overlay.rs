//! Help overlay showing keyboard shortcuts for Windows.
//!
//! Displays a dialog with all available hotkeys.
//! Dismisses on button click or Enter key.

use std::sync::atomic::{AtomicBool, Ordering};

use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

/// Guard to prevent multiple help overlays
static HELP_OPENING: AtomicBool = AtomicBool::new(false);

/// All hotkeys to display (Windows versions)
const HOTKEYS_EN: &[(&str, &str)] = &[
    ("Ctrl + Shift + A", "Toggle overlay"),
    ("Ctrl + Shift + S", "Open settings"),
    ("Ctrl + Shift + H", "Show help"),
    ("Ctrl + Shift + Q", "Quit application"),
];

const HOTKEYS_ES: &[(&str, &str)] = &[
    ("Ctrl + Shift + A", "Alternar overlay"),
    ("Ctrl + Shift + S", "Abrir configuración"),
    ("Ctrl + Shift + H", "Mostrar ayuda"),
    ("Ctrl + Shift + Q", "Salir de la aplicación"),
];

/// Show the help overlay with keyboard shortcuts.
///
/// Uses a MessageBox for simplicity on Windows.
/// Could be replaced with a custom dialog for a more polished look.
pub fn show_help_overlay(hwnd: HWND, is_spanish: bool) {
    // Atomic guard: only one help overlay can be opening at a time
    if HELP_OPENING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    let hotkeys = if is_spanish { HOTKEYS_ES } else { HOTKEYS_EN };

    let title = if is_spanish {
        "Atajos de Teclado"
    } else {
        "Keyboard Shortcuts"
    };

    // Build message with aligned columns
    let mut message = String::new();
    for (keys, desc) in hotkeys {
        message.push_str(&format!("{:<20} {}\n", keys, desc));
    }
    message.push('\n');
    message.push_str(if is_spanish {
        "Pulsa Aceptar para cerrar"
    } else {
        "Press OK to close"
    });

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

    // Reset atomic guard
    HELP_OPENING.store(false, Ordering::SeqCst);
}
