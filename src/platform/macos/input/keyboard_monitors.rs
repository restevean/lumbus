//! Local keyboard monitors for input handling.
//!
//! This module provides keyboard monitoring that doesn't rely on Carbon,
//! serving as a backup for the Ctrl+A toggle functionality.

use block::ConcreteBlock;
use cocoa::base::{id, nil};
use objc::{class, msg_send, sel, sel_impl};


/// Install a local monitor for Ctrl+A key combination.
///
/// This serves as a backup for the Carbon hotkey in case it gets dropped.
/// Local monitors work when the app has focus (e.g., settings window is open).
pub unsafe fn install_local_ctrl_a_monitor(view: id) {
    let existing: id = *(*view).get_ivar::<id>("_localKeyMonitor");
    if existing != nil {
        return;
    }

    const KEY_DOWN_MASK: u64 = 1 << 10;
    const CTRL_FLAG: u64 = 1 << 18;
    const KEYCODE_A: u16 = 0;

    let host = view;
    let block = ConcreteBlock::new(move |event: id| {
        unsafe {
            let keycode: u16 = msg_send![event, keyCode];
            let flags: u64 = msg_send![event, modifierFlags];
            if keycode == KEYCODE_A && (flags & CTRL_FLAG) != 0 {
                let _: () = msg_send![
                    host,
                    performSelectorOnMainThread: sel!(requestToggle)
                    withObject: nil
                    waitUntilDone: cocoa::base::NO
                ];
            }
        }
        event
    })
        .copy();

    let mon: id = msg_send![
        class!(NSEvent),
        addLocalMonitorForEventsMatchingMask: KEY_DOWN_MASK
        handler: &*block
    ];
    (*view).set_ivar::<id>("_localKeyMonitor", mon);
}
