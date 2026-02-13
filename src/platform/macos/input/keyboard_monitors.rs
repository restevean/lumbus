//! Local keyboard monitors for input handling.
//!
//! This module provides keyboard monitoring that doesn't rely on Carbon,
//! serving as a backup for the Ctrl+A toggle functionality.

use crate::platform::macos::ffi::bridge::{get_class, id, msg_send, nil, sel, ObjectExt, NO};
use block2::RcBlock;

/// Install a local monitor for Ctrl+A key combination.
///
/// This serves as a backup for the Carbon hotkey in case it gets dropped.
/// Local monitors work when the app has focus (e.g., settings window is open).
///
/// # Safety
/// - `view` must be a valid, non-null pointer to a CustomViewMulti.
/// - Must be called from main thread with valid autorelease pool.
pub unsafe fn install_local_ctrl_a_monitor(view: id) {
    let existing: id = *(*view).load_ivar::<id>("_localKeyMonitor");
    if existing != nil {
        return;
    }

    const KEY_DOWN_MASK: u64 = 1 << 10;
    const CTRL_FLAG: u64 = 1 << 18;
    const KEYCODE_A: u16 = 0;

    let host = view;
    let block = RcBlock::new(move |event: id| -> id {
        unsafe {
            let keycode: u16 = msg_send![event, keyCode];
            let flags: u64 = msg_send![event, modifierFlags];
            if keycode == KEYCODE_A && (flags & CTRL_FLAG) != 0 {
                let _: () = msg_send![
                    host,
                    performSelectorOnMainThread: sel!(requestToggle),
                    withObject: nil,
                    waitUntilDone: NO
                ];
            }
        }
        event
    });

    let mon: id = msg_send![
        get_class("NSEvent"),
        addLocalMonitorForEventsMatchingMask: KEY_DOWN_MASK,
        handler: &*block
    ];
    (*view).store_ivar::<id>("_localKeyMonitor", mon);
}
