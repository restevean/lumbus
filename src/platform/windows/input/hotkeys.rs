//! Global hotkeys and mouse hooks for Windows.

use std::sync::atomic::{AtomicIsize, Ordering};

use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HHOOK, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

use crate::model::constants::*;
use crate::platform::windows::app::state::STATE;

// Hotkey IDs
pub const HOTKEY_TOGGLE: i32 = 1;
pub const HOTKEY_SETTINGS: i32 = 2;
pub const HOTKEY_QUIT: i32 = 3;
pub const HOTKEY_HELP: i32 = 4;

// Timer constants
pub const TIMER_CURSOR: usize = 1;
pub const TIMER_INTERVAL_MS: u32 = 16; // ~60 FPS

/// Global mouse hook handle (must be static for the hook callback).
pub static MOUSE_HOOK: AtomicIsize = AtomicIsize::new(0);

/// Low-level mouse hook procedure for detecting mouse button presses.
pub extern "system" fn mouse_hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if ncode >= 0 {
            let new_mode = match wparam.0 as u32 {
                WM_LBUTTONDOWN => Some(DISPLAY_MODE_LEFT),
                WM_RBUTTONDOWN => Some(DISPLAY_MODE_RIGHT),
                WM_LBUTTONUP | WM_RBUTTONUP => Some(DISPLAY_MODE_CIRCLE),
                _ => None,
            };

            if let Some(mode) = new_mode {
                STATE.with(|s| {
                    let mut state = s.borrow_mut();
                    state.display_mode = mode;
                });
            }
        }

        let hook = MOUSE_HOOK.load(Ordering::SeqCst);
        CallNextHookEx(Some(HHOOK(hook as *mut _)), ncode, wparam, lparam)
    }
}
