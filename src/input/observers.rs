//! System observers for hotkey keep-alive and wakeup events.
//!
//! This module installs observers that help maintain Carbon hotkeys
//! across system events like sleep/wake, session changes, and space changes.

use block::ConcreteBlock;
use cocoa::base::{id, nil};
use objc::{class, msg_send, sel, sel_impl};

use crate::ffi::*;
use crate::input::hotkeys::{uninstall_hotkeys, reinstall_hotkeys, HotkeyHandler};

/// Install an observer that cleans up Carbon resources when app terminates.
pub unsafe fn install_termination_observer(view: id, handler: HotkeyHandler) {
    let center: id = msg_send![class!(NSNotificationCenter), defaultCenter];
    let queue: id = nil; // main thread

    // Use handler indirectly to avoid capturing it
    let _ = handler; // We only use uninstall_hotkeys in termination
    let block = ConcreteBlock::new(move |_note: id| {
        unsafe {
            uninstall_hotkeys(view);
        }
    })
        .copy();

    let name: id = msg_send![
        class!(NSString),
        stringWithUTF8String: b"NSApplicationWillTerminateNotification\0".as_ptr() as *const _
    ];
    let _: id =
        msg_send![center, addObserverForName: name object: nil queue: queue usingBlock: &*block];
}

/// Start a repeating NSTimer to periodically re-install hotkeys (defensive).
///
/// This helps recover from scenarios where Carbon hotkeys get dropped.
pub unsafe fn start_hotkey_keepalive(view: id) {
    use objc::runtime::Class;

    // Clear previous timer if any
    let prev: id = *(*view).get_ivar::<id>("_hkKeepAliveTimer");
    if prev != nil {
        let _: () = msg_send![prev, invalidate];
        (*view).set_ivar::<id>("_hkKeepAliveTimer", nil);
    }

    // 60s interval; cheap operation (reinstall is idempotent)
    let timer_class = Class::get("NSTimer").unwrap();
    let timer: id = msg_send![
        timer_class,
        scheduledTimerWithTimeInterval: 60.0f64
        target: view
        selector: sel!(hotkeyKeepAlive)
        userInfo: nil
        repeats: cocoa::base::YES
    ];
    (*view).set_ivar::<id>("_hkKeepAliveTimer", timer);
}

/// Observe system events that may disrupt Carbon hotkeys and re-install on demand.
///
/// Watches for:
/// - Wake from sleep
/// - Session became active (unlock/login)
/// - Active Space changed (Mission Control / Spaces)
pub unsafe fn install_wakeup_space_observers(view: id, handler: HotkeyHandler) {
    let ws: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let nc: id = msg_send![ws, notificationCenter];

    // Helper to add an observer for a given notification name (C string)
    let add_obs = |name_cstr: &'static [u8]| {
        let name: id =
            msg_send![class!(NSString), stringWithUTF8String: name_cstr.as_ptr() as *const _];
        let block = ConcreteBlock::new(move |_note: id| unsafe {
            reinstall_hotkeys(view, handler);
        })
            .copy();
        let _: id = msg_send![nc, addObserverForName: name object: nil queue: nil usingBlock: &*block];
    };

    // Wake from sleep
    add_obs(b"NSWorkspaceDidWakeNotification\0");
    // Session became active (unlock/login)
    add_obs(b"NSWorkspaceSessionDidBecomeActiveNotification\0");
    // Active Space changed (Mission Control / Spaces)
    add_obs(b"NSWorkspaceActiveSpaceDidChangeNotification\0");
}
