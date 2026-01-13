//! Hotkey constants and registration helpers for Windows.

use windows::Win32::UI::Input::KeyboardAndMouse::{
    MOD_CONTROL, MOD_SHIFT, HOT_KEY_MODIFIERS,
};

/// Virtual key code for 'A'.
pub const VK_A: u32 = 0x41;

/// Virtual key code for 'H'.
pub const VK_H: u32 = 0x48;

/// Virtual key code for 'X'.
pub const VK_X: u32 = 0x58;

/// Virtual key code for comma.
pub const VK_OEM_COMMA: u32 = 0xBC;

/// Hotkey IDs.
pub const HKID_TOGGLE: i32 = 1;
pub const HKID_SETTINGS: i32 = 2;
pub const HKID_QUIT: i32 = 3;
pub const HKID_HELP: i32 = 4;

/// Control modifier.
pub const MOD_CTRL: HOT_KEY_MODIFIERS = MOD_CONTROL;

/// Shift modifier.
pub const MOD_SH: HOT_KEY_MODIFIERS = MOD_SHIFT;
