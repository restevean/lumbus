//! Persistence of state to NSUserDefaults.
//!
//! This module provides functions to load and save overlay state
//! to macOS NSUserDefaults.

use crate::platform::macos::ffi::bridge::{get_class, id, msg_send, nil, nsstring_id};

use crate::model::constants::*;
use crate::model::OverlayState;

/// Reads a double from NSUserDefaults, returns default if not set.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn prefs_get_double(key: &str, default: f64) -> f64 {
    let ud: id = msg_send![get_class("NSUserDefaults"), standardUserDefaults];
    let k = nsstring_id(key);
    let obj: id = msg_send![ud, objectForKey: k];
    if obj == nil {
        default
    } else {
        msg_send![ud, doubleForKey: k]
    }
}

/// Saves a double to NSUserDefaults.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn prefs_set_double(key: &str, val: f64) {
    let ud: id = msg_send![get_class("NSUserDefaults"), standardUserDefaults];
    let k = nsstring_id(key);
    let _: () = msg_send![ud, setDouble: val, forKey: k];
}

/// Reads an integer from NSUserDefaults, returns default if not set.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn prefs_get_int(key: &str, default: i32) -> i32 {
    let ud: id = msg_send![get_class("NSUserDefaults"), standardUserDefaults];
    let k = nsstring_id(key);
    let obj: id = msg_send![ud, objectForKey: k];
    if obj == nil {
        default
    } else {
        // NSInteger is i64 on 64-bit macOS
        let val: i64 = msg_send![ud, integerForKey: k];
        val as i32
    }
}

/// Saves an integer to NSUserDefaults.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn prefs_set_int(key: &str, val: i32) {
    let ud: id = msg_send![get_class("NSUserDefaults"), standardUserDefaults];
    let k = nsstring_id(key);
    // NSInteger is i64 on 64-bit macOS
    let _: () = msg_send![ud, setInteger: val as i64, forKey: k];
}

/// Loads complete state from NSUserDefaults.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn load_state() -> OverlayState {
    let mut state = OverlayState {
        radius: prefs_get_double(PREF_RADIUS, DEFAULT_DIAMETER / 2.0),
        border_width: prefs_get_double(PREF_BORDER, DEFAULT_BORDER_WIDTH),
        stroke_r: prefs_get_double(PREF_STROKE_R, DEFAULT_COLOR.0),
        stroke_g: prefs_get_double(PREF_STROKE_G, DEFAULT_COLOR.1),
        stroke_b: prefs_get_double(PREF_STROKE_B, DEFAULT_COLOR.2),
        stroke_a: prefs_get_double(PREF_STROKE_A, DEFAULT_COLOR.3),
        fill_transparency_pct: prefs_get_double(
            PREF_FILL_TRANSPARENCY,
            DEFAULT_FILL_TRANSPARENCY_PCT,
        ),
        lang: prefs_get_int(PREF_LANG, LANG_EN),
        overlay_enabled: true,
        display_mode: DISPLAY_MODE_CIRCLE,
    };
    state.validate();
    state
}

/// Saves complete state to NSUserDefaults.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn save_state(state: &OverlayState) {
    prefs_set_double(PREF_RADIUS, state.radius);
    prefs_set_double(PREF_BORDER, state.border_width);
    prefs_set_double(PREF_STROKE_R, state.stroke_r);
    prefs_set_double(PREF_STROKE_G, state.stroke_g);
    prefs_set_double(PREF_STROKE_B, state.stroke_b);
    prefs_set_double(PREF_STROKE_A, state.stroke_a);
    prefs_set_double(PREF_FILL_TRANSPARENCY, state.fill_transparency_pct);
    prefs_set_int(PREF_LANG, state.lang);
}
