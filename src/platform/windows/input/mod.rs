//! Input handling for Windows (hotkeys, mouse hooks).

pub mod hotkeys;

pub use hotkeys::{
    mouse_hook_proc, HOTKEY_HELP, HOTKEY_QUIT, HOTKEY_SETTINGS, HOTKEY_TOGGLE, MOUSE_HOOK,
    TIMER_CURSOR, TIMER_INTERVAL_MS,
};
