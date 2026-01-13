//! Cocoa utility functions.
//!
//! This module provides helper functions for common Cocoa operations
//! like NSString conversion, mouse position, and display ID retrieval.

use cocoa::base::{id, nil};
use cocoa::foundation::NSPoint;
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::CString;

/// Window level slightly above context menus and Dock.
pub fn nspop_up_menu_window_level() -> i64 {
    201
}

/// Window level for overlay (above popup menus).
pub fn overlay_window_level() -> i64 {
    nspop_up_menu_window_level() + 1
}

/// Global mouse position in Cocoa coordinates (origin bottom-left).
pub fn get_mouse_position_cocoa() -> (f64, f64) {
    unsafe {
        let cls = class!(NSEvent);
        let p: NSPoint = msg_send![cls, mouseLocation];
        (p.x, p.y)
    }
}

/// Create NSString* from &str.
///
/// # Safety
/// Caller must ensure the returned id is used within a valid autorelease pool.
pub unsafe fn nsstring(s: &str) -> id {
    let cstr = CString::new(s).unwrap();
    let ns: id = msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()];
    ns
}

/// Get stable CGDirectDisplayID for an NSScreen.
///
/// This ID does not change across Space switches or sleep/wake cycles.
///
/// # Safety
/// Caller must ensure `screen` is a valid NSScreen pointer.
pub unsafe fn display_id_for_screen(screen: id) -> u32 {
    let desc: id = msg_send![screen, deviceDescription];
    let key = nsstring("NSScreenNumber");
    let num: id = msg_send![desc, objectForKey: key];
    if num == nil {
        0
    } else {
        let v: u64 = msg_send![num, unsignedIntegerValue];
        v as u32
    }
}
