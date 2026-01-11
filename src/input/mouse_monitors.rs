//! Global mouse event monitors.
//!
//! This module handles global mouse events to show L/R indicators
//! when clicking and to track mouse movement.

use block::ConcreteBlock;
use cocoa::base::{id, YES};
use objc::{class, msg_send, sel, sel_impl};

use crate::ffi::*;
use crate::app::apply_to_all_views;

/// Install global mouse monitors for tracking clicks and movement.
///
/// Sets up monitors for:
/// - Left mouse down → show "L"
/// - Left mouse up → show circle
/// - Right mouse down → show "R"
/// - Right mouse up → show circle
/// - Mouse moved → update cursor position
pub unsafe fn install_mouse_monitors(view: id) {
    // NSEvent masks: leftDown=1<<1, leftUp=1<<2, rightDown=1<<3, rightUp=1<<4, mouseMoved=1<<6
    const LEFT_DOWN_MASK: u64 = 1 << 1;
    const LEFT_UP_MASK: u64 = 1 << 2;
    const RIGHT_DOWN_MASK: u64 = 1 << 3;
    const RIGHT_UP_MASK: u64 = 1 << 4;
    const MOUSE_MOVED_MASK: u64 = 1 << 6;

    let cls = class!(NSEvent);

    // LEFT DOWN -> L mode
    let h1 = ConcreteBlock::new(move |_e: id| {
        unsafe {
            apply_to_all_views(|v| { *(*v).get_mut_ivar::<i32>("_displayMode") = 1 });
            apply_to_all_views(|v| { let _: () = msg_send![v, setNeedsDisplay: YES]; });
        }
    })
        .copy();
    let mon_ld: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: LEFT_DOWN_MASK handler: &*h1];
    (*view).set_ivar::<id>("_monLeftDown", mon_ld);

    // LEFT UP -> circle
    let h2 = ConcreteBlock::new(move |_e: id| {
        unsafe {
            apply_to_all_views(|v| { *(*v).get_mut_ivar::<i32>("_displayMode") = 0 });
            apply_to_all_views(|v| { let _: () = msg_send![v, setNeedsDisplay: YES]; });
        }
    })
        .copy();
    let mon_lu: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: LEFT_UP_MASK handler: &*h2];
    (*view).set_ivar::<id>("_monLeftUp", mon_lu);

    // RIGHT DOWN -> R mode
    let h3 = ConcreteBlock::new(move |_e: id| {
        unsafe {
            apply_to_all_views(|v| { *(*v).get_mut_ivar::<i32>("_displayMode") = 2 });
            apply_to_all_views(|v| { let _: () = msg_send![v, setNeedsDisplay: YES]; });
        }
    })
        .copy();
    let mon_rd: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: RIGHT_DOWN_MASK handler: &*h3];
    (*view).set_ivar::<id>("_monRightDown", mon_rd);

    // RIGHT UP -> circle
    let h4 = ConcreteBlock::new(move |_e: id| {
        unsafe {
            apply_to_all_views(|v| { *(*v).get_mut_ivar::<i32>("_displayMode") = 0 });
            apply_to_all_views(|v| { let _: () = msg_send![v, setNeedsDisplay: YES]; });
        }
    })
        .copy();
    let mon_ru: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: RIGHT_UP_MASK handler: &*h4];
    (*view).set_ivar::<id>("_monRightUp", mon_ru);

    // mouseMoved → schedule update on the main thread
    let host = view;
    let hmove = ConcreteBlock::new(move |_e: id| {
        unsafe {
            let _: () = msg_send![
                host,
                performSelectorOnMainThread: sel!(update_cursor_multi)
                withObject: cocoa::base::nil
                waitUntilDone: cocoa::base::NO
            ];
        }
    })
        .copy();
    let mon_move: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: MOUSE_MOVED_MASK handler: &*hmove];
    (*view).set_ivar::<id>("_monMove", mon_move);
}
