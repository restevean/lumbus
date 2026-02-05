//! Cocoa utility functions.
//!
//! This module provides helper functions for common Cocoa operations
//! like NSString conversion, mouse position, and display ID retrieval.

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::NSEvent;
use objc2_foundation::NSString;

use super::types::{Id, NIL};

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
    let p = NSEvent::mouseLocation();
    (p.x, p.y)
}

/// Create NSString from &str.
///
/// Returns a retained NSString that manages its own memory.
pub fn nsstring(s: &str) -> Retained<NSString> {
    NSString::from_str(s)
}

/// Create NSString* (raw pointer) from &str for FFI compatibility.
///
/// # Safety
/// Caller must ensure the returned pointer is used within a valid context
/// and properly released or autoreleased.
pub unsafe fn nsstring_raw(s: &str) -> Id {
    let ns = NSString::from_str(s);
    Retained::into_raw(ns) as Id
}

/// Get stable CGDirectDisplayID for an NSScreen.
///
/// This ID does not change across Space switches or sleep/wake cycles.
///
/// # Safety
/// Caller must ensure `screen` is a valid NSScreen pointer.
pub unsafe fn display_id_for_screen(screen: Id) -> u32 {
    let desc: Id = msg_send![screen, deviceDescription];
    let key = nsstring("NSScreenNumber");
    let key_ptr = Retained::as_ptr(&key) as *mut AnyObject;
    let num: Id = msg_send![desc, objectForKey: key_ptr];
    if num == NIL {
        0
    } else {
        let v: u64 = msg_send![num, unsignedIntegerValue];
        v as u32
    }
}
