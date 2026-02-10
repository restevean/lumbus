//! Global application helpers.
//!
//! This module contains helper functions that operate on all views
//! and are used across multiple modules (input, ui).

use crate::platform::macos::ffi::bridge::{get_class, id, msg_send, nil, NSApp, ObjectExt};

/// Apply a closure to every contentView whose class is CustomViewMulti.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn apply_to_all_views<F: Fn(id)>(f: F) {
    let app: id = NSApp();
    let windows: id = msg_send![app, windows];
    let wcount: usize = msg_send![windows, count];

    let custom_cls = get_class("CustomViewMulti");

    for j in 0..wcount {
        let win: id = msg_send![windows, objectAtIndex: j];
        let view: id = msg_send![win, contentView];
        if view != nil {
            let is_custom: bool = msg_send![view, isKindOfClass: custom_cls];
            if is_custom {
                f(view);
            }
        }
    }
}

/// Copy visual prefs from src view to all views.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn sync_visual_prefs_to_all_views(src: id) {
    let radius = *(*src).load_ivar::<f64>("_radius");
    let border = *(*src).load_ivar::<f64>("_borderWidth");
    let r = *(*src).load_ivar::<f64>("_strokeR");
    let g = *(*src).load_ivar::<f64>("_strokeG");
    let b = *(*src).load_ivar::<f64>("_strokeB");
    let a = *(*src).load_ivar::<f64>("_strokeA");
    let fill_t = *(*src).load_ivar::<f64>("_fillTransparencyPct");
    let lang = *(*src).load_ivar::<i32>("_lang");

    apply_to_all_views(|v| {
        (*v).store_ivar::<f64>("_radius", radius);
        (*v).store_ivar::<f64>("_borderWidth", border);
        (*v).store_ivar::<f64>("_strokeR", r);
        (*v).store_ivar::<f64>("_strokeG", g);
        (*v).store_ivar::<f64>("_strokeB", b);
        (*v).store_ivar::<f64>("_strokeA", a);
        (*v).store_ivar::<f64>("_fillTransparencyPct", fill_t);
        (*v).store_ivar::<i32>("_lang", lang);
    });
}

/// Check if current language is Spanish.
pub fn lang_is_es(view: id) -> bool {
    unsafe {
        let l = *(*view).load_ivar::<i32>("_lang");
        l == 1
    }
}
