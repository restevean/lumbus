//! Global application helpers.
//!
//! This module contains helper functions that operate on all views
//! and are used across multiple modules (input, ui).

use cocoa::appkit::NSApp;
use cocoa::base::{id, nil};
use objc::runtime::Class;
use objc::{msg_send, sel, sel_impl};

/// Apply a closure to every contentView whose class is CustomViewMulti.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn apply_to_all_views<F: Fn(id)>(f: F) {
    let app: id = NSApp();
    let windows: id = msg_send![app, windows];
    let wcount: usize = msg_send![windows, count];

    let custom_cls = Class::get("CustomViewMulti").unwrap();

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
    let radius = *(*src).get_ivar::<f64>("_radius");
    let border = *(*src).get_ivar::<f64>("_borderWidth");
    let r = *(*src).get_ivar::<f64>("_strokeR");
    let g = *(*src).get_ivar::<f64>("_strokeG");
    let b = *(*src).get_ivar::<f64>("_strokeB");
    let a = *(*src).get_ivar::<f64>("_strokeA");
    let fill_t = *(*src).get_ivar::<f64>("_fillTransparencyPct");
    let lang = *(*src).get_ivar::<i32>("_lang");

    apply_to_all_views(|v| {
        (*v).set_ivar::<f64>("_radius", radius);
        (*v).set_ivar::<f64>("_borderWidth", border);
        (*v).set_ivar::<f64>("_strokeR", r);
        (*v).set_ivar::<f64>("_strokeG", g);
        (*v).set_ivar::<f64>("_strokeB", b);
        (*v).set_ivar::<f64>("_strokeA", a);
        (*v).set_ivar::<f64>("_fillTransparencyPct", fill_t);
        (*v).set_ivar::<i32>("_lang", lang);
    });
}

/// Check if current language is Spanish.
pub fn lang_is_es(view: id) -> bool {
    unsafe {
        let l = *(*view).get_ivar::<i32>("_lang");
        l == 1
    }
}
