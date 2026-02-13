//! System observers for hotkey keep-alive and wakeup events.
//!
//! This module installs observers that help maintain Carbon hotkeys
//! across system events like sleep/wake, session changes, and space changes.

use block2::RcBlock;
use crate::platform::macos::ffi::bridge::{get_class, id, msg_send, nil, sel, ObjectExt, YES};

use crate::platform::macos::input::hotkeys::{reinstall_hotkeys, uninstall_hotkeys, HotkeyHandler};

/// Install an observer that cleans up Carbon resources when app terminates.
///
/// # Safety
/// - `view` must be a valid, non-null pointer to a CustomViewMulti.
/// - Must be called from main thread with valid autorelease pool.
pub unsafe fn install_termination_observer(view: id, handler: HotkeyHandler) {
    let center: id = msg_send![get_class("NSNotificationCenter"), defaultCenter];
    let queue: id = nil; // main thread

    // Use handler indirectly to avoid capturing it
    let _ = handler; // We only use uninstall_hotkeys in termination
    let block = RcBlock::new(move |_note: id| {
        unsafe {
            uninstall_hotkeys(view);
        }
    });

    let name: id = msg_send![
        get_class("NSString"),
        stringWithUTF8String: c"NSApplicationWillTerminateNotification".as_ptr()
    ];
    let _: id =
        msg_send![center, addObserverForName: name, object: nil, queue: queue, usingBlock: &*block];
}

/// Start a repeating NSTimer to periodically re-install hotkeys (defensive).
///
/// This helps recover from scenarios where Carbon hotkeys get dropped.
///
/// # Safety
/// - `view` must be a valid, non-null pointer to a CustomViewMulti.
/// - Must be called from main thread with valid autorelease pool.
pub unsafe fn start_hotkey_keepalive(view: id) {
    // Clear previous timer if any
    let prev: id = *(*view).load_ivar::<id>("_hkKeepAliveTimer");
    if prev != nil {
        let _: () = msg_send![prev, invalidate];
        (*view).store_ivar::<id>("_hkKeepAliveTimer", nil);
    }

    // 60s interval; cheap operation (reinstall is idempotent)
    let timer_class = get_class("NSTimer");
    let timer: id = msg_send![
        timer_class,
        scheduledTimerWithTimeInterval: 60.0f64,
        target: view,
        selector: sel!(hotkeyKeepAlive),
        userInfo: nil,
        repeats: YES
    ];
    (*view).store_ivar::<id>("_hkKeepAliveTimer", timer);
}

/// Observe system events that may disrupt Carbon hotkeys and re-install on demand.
///
/// Watches for:
/// - Wake from sleep
/// - Session became active (unlock/login)
/// - Active Space changed (Mission Control / Spaces)
///
/// # Safety
/// - `view` must be a valid, non-null pointer to a CustomViewMulti.
/// - `handler` must be a valid function pointer.
/// - Must be called from main thread with valid autorelease pool.
pub unsafe fn install_wakeup_space_observers(view: id, handler: HotkeyHandler) {
    let ws: id = msg_send![get_class("NSWorkspace"), sharedWorkspace];
    let nc: id = msg_send![ws, notificationCenter];

    // Helper to add an observer for a given notification name (C string)
    let add_obs = |name_cstr: &std::ffi::CStr| {
        let name: id = msg_send![get_class("NSString"), stringWithUTF8String: name_cstr.as_ptr()];
        let block = RcBlock::new(move |_note: id| {
            unsafe {
                reinstall_hotkeys(view, handler);
            }
        });
        let _: id =
            msg_send![nc, addObserverForName: name, object: nil, queue: nil, usingBlock: &*block];
    };

    // Wake from sleep
    add_obs(c"NSWorkspaceDidWakeNotification");
    // Session became active (unlock/login)
    add_obs(c"NSWorkspaceSessionDidBecomeActiveNotification");
    // Active Space changed (Mission Control / Spaces)
    add_obs(c"NSWorkspaceActiveSpaceDidChangeNotification");
}
