//! CustomView class for the overlay.
//!
//! This module contains the NSView subclass that handles:
//! - Drawing the cursor overlay (circle or L/R letter)
//! - Processing input events from the event bus
//! - Settings UI actions (sliders, color picker, etc.)
//! - Status bar menu actions

use std::ffi::{c_char, CStr};

use crate::events::{publish, AppEvent};
use crate::model::constants::*;
use crate::platform::macos::app::apply_to_all_views;
use crate::platform::macos::ffi::bridge::{get_class, id, msg_send, nil, nsstring_id, ObjectExt, NO, YES};
use crate::platform::macos::ffi::{
    display_id_for_screen, get_mouse_position_cocoa, CFAbsoluteTimeGetCurrent,
};
use crate::platform::macos::handlers::dispatch_events;
use crate::platform::macos::input::{hotkey_event_handler, reinstall_hotkeys};
use crate::platform::macos::storage::{prefs_set_double, prefs_set_int};
use crate::platform::macos::ui::{
    close_settings_window, confirm_and_maybe_quit, draw_circle, draw_letter,
    open_settings_window, update_status_bar_language, ClickLetter, DrawParams,
};
use crate::{color_to_hex, parse_hex_color, tr_key};

use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, Sel};
use objc2::sel;
use objc2_foundation::{NSPoint, NSRect, NSSize};

// ============================================================================
// Helper functions for boolean ivars (stored as u8)
// ============================================================================

/// Load a boolean ivar stored as u8.
///
/// # Safety
/// The object must have an ivar with the given name of type u8.
pub unsafe fn get_bool_ivar(obj: id, name: &str) -> bool {
    let val = *(*obj).load_ivar::<u8>(name);
    val != 0
}

/// Store a boolean ivar as u8.
///
/// # Safety
/// The object must have an ivar with the given name of type u8.
pub unsafe fn set_bool_ivar(obj: id, name: &str, val: bool) {
    (*obj).store_ivar::<u8>(name, if val { 1 } else { 0 });
}

// ============================================================================
// CustomView registration and creation
// ============================================================================

/// Register the CustomView class and create an instance.
///
/// This creates an NSView subclass with all the necessary instance variables
/// and methods for the overlay functionality.
///
/// # Safety
/// Must be called from the main thread. The window must be a valid NSWindow.
pub unsafe fn register_and_create_view(window: id, width: f64, height: f64) -> id {
    let class_name = c"CustomViewMulti";
    let custom_view_class = if let Some(cls) = AnyClass::get(class_name) {
        cls
    } else {
        let superclass = AnyClass::get(c"NSView").unwrap();
        let mut builder = ClassBuilder::new(class_name, superclass).unwrap();

        // ====== Instance Variables ======
        register_ivars(&mut builder);

        // ====== Methods ======
        register_methods(&mut builder);

        builder.register()
    };

    let view: id = msg_send![custom_view_class, alloc];
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
    let view: id = msg_send![view, initWithFrame: frame];

    // Initialize instance variables
    initialize_view_ivars(view);

    let _: () = msg_send![window, setContentView: view];
    view
}

/// Register all instance variables for the CustomView.
///
/// # Safety
/// Must be called during class registration.
unsafe fn register_ivars(builder: &mut ClassBuilder) {
    // Base state
    builder.add_ivar::<f64>(c"_cursorXScreen");
    builder.add_ivar::<f64>(c"_cursorYScreen");
    builder.add_ivar::<u8>(c"_visible"); // visible by screen selection (bool as u8)
    builder.add_ivar::<u8>(c"_overlayEnabled"); // global toggle (bool as u8)
    builder.add_ivar::<i32>(c"_displayMode"); // 0=circle, 1=L, 2=R
    builder.add_ivar::<id>(c"_ownScreen"); // owning NSScreen
    builder.add_ivar::<u32>(c"_ownDisplayID"); // stable DisplayID
    builder.add_ivar::<i32>(c"_lang"); // 0=en, 1=es

    // Visual parameters
    builder.add_ivar::<f64>(c"_radius");
    builder.add_ivar::<f64>(c"_borderWidth");
    builder.add_ivar::<f64>(c"_strokeR");
    builder.add_ivar::<f64>(c"_strokeG");
    builder.add_ivar::<f64>(c"_strokeB");
    builder.add_ivar::<f64>(c"_strokeA");
    builder.add_ivar::<f64>(c"_fillTransparencyPct"); // 0..100

    // Carbon refs
    builder.add_ivar::<*mut std::ffi::c_void>(c"_hkHandler");
    builder.add_ivar::<*mut std::ffi::c_void>(c"_hkToggle");
    builder.add_ivar::<*mut std::ffi::c_void>(c"_hkComma");
    builder.add_ivar::<*mut std::ffi::c_void>(c"_hkHelp");
    builder.add_ivar::<*mut std::ffi::c_void>(c"_hkQuit");

    // Keep-alive timer for hotkeys
    builder.add_ivar::<id>(c"_hkKeepAliveTimer");

    // Global mouse monitors
    builder.add_ivar::<id>(c"_monLeftDown");
    builder.add_ivar::<id>(c"_monLeftUp");
    builder.add_ivar::<id>(c"_monRightDown");
    builder.add_ivar::<id>(c"_monRightUp");
    builder.add_ivar::<id>(c"_monMove");

    // Settings UI refs
    builder.add_ivar::<id>(c"_settingsWindow");
    builder.add_ivar::<id>(c"_labelLang");
    builder.add_ivar::<id>(c"_popupLang");

    builder.add_ivar::<id>(c"_labelRadius");
    builder.add_ivar::<id>(c"_fieldRadius"); // now a label
    builder.add_ivar::<id>(c"_sliderRadius");

    builder.add_ivar::<id>(c"_labelBorder");
    builder.add_ivar::<id>(c"_fieldBorder"); // now a label
    builder.add_ivar::<id>(c"_sliderBorder");

    builder.add_ivar::<id>(c"_labelColor");
    builder.add_ivar::<id>(c"_colorWell");

    builder.add_ivar::<id>(c"_labelHex");
    builder.add_ivar::<id>(c"_fieldHex"); // remains editable

    builder.add_ivar::<id>(c"_labelFillT");
    builder.add_ivar::<id>(c"_fieldFillT"); // now a label
    builder.add_ivar::<id>(c"_sliderFillT");

    builder.add_ivar::<id>(c"_btnClose");
    builder.add_ivar::<id>(c"_previousApp"); // App to restore focus to when closing settings

    // Local key monitor (for Ctrl+A redundancy)
    builder.add_ivar::<id>(c"_localKeyMonitor");

    // Debounce toggles
    builder.add_ivar::<f64>(c"_lastToggleTs");

    // Menu installed flag
    builder.add_ivar::<u8>(c"_menuInstalled"); // bool as u8

    // Refresh timer
    builder.add_ivar::<id>(c"_updateTimer");

    // Monitors used while settings window is open (ESC/Enter)
    builder.add_ivar::<id>(c"_settingsEscMonitor");
    builder.add_ivar::<id>(c"_settingsGlobalMonitor");
}

/// Register all methods for the CustomView.
///
/// # Safety
/// Must be called during class registration.
unsafe fn register_methods(builder: &mut ClassBuilder) {
    // Core methods
    builder.add_method(
        sel!(update_cursor_multi),
        update_cursor_multi as unsafe extern "C-unwind" fn(_, _),
    );
    builder.add_method(
        sel!(toggleVisibility),
        toggle_visibility as unsafe extern "C-unwind" fn(_, _),
    );
    builder.add_method(
        sel!(requestToggle),
        request_toggle as unsafe extern "C-unwind" fn(_, _),
    );
    builder.add_method(
        sel!(hotkeyKeepAlive),
        hotkey_keepalive as unsafe extern "C-unwind" fn(_, _),
    );

    // Settings slider actions
    builder.add_method(
        sel!(setRadius:),
        set_radius as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(setRadiusFromField:),
        set_radius_from_field as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(setBorderWidth:),
        set_border_width as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(setBorderFromField:),
        set_border_from_field as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(setFillTransparency:),
        set_fill_transparency as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(setFillTransparencyFromField:),
        set_fill_transparency_from_field as unsafe extern "C-unwind" fn(_, _, _),
    );

    // Color actions
    builder.add_method(
        sel!(colorChanged:),
        color_changed as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(hexChanged:),
        hex_changed as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(closeSettings:),
        close_settings as unsafe extern "C-unwind" fn(_, _, _),
    );

    // Status bar menu actions
    builder.add_method(
        sel!(statusBarSettings:),
        status_bar_settings as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(statusBarHelp:),
        status_bar_help as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(statusBarAbout:),
        status_bar_about as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(statusBarQuit:),
        status_bar_quit as unsafe extern "C-unwind" fn(_, _, _),
    );

    // Language change
    builder.add_method(
        sel!(langChanged:),
        lang_changed as unsafe extern "C-unwind" fn(_, _, _),
    );
    builder.add_method(
        sel!(controlTextDidChange:),
        control_text_did_change as unsafe extern "C-unwind" fn(_, _, _),
    );

    // Drawing
    builder.add_method(
        sel!(drawRect:),
        draw_rect as unsafe extern "C-unwind" fn(_, _, _),
    );
}

/// Initialize all instance variables to default values.
///
/// # Safety
/// The view must be a valid CustomView instance.
unsafe fn initialize_view_ivars(view: id) {
    // Initial state
    (*view).store_ivar::<f64>("_cursorXScreen", 0.0);
    (*view).store_ivar::<f64>("_cursorYScreen", 0.0);
    set_bool_ivar(view, "_visible", false);
    set_bool_ivar(view, "_overlayEnabled", true); // Circle visible on app start
    (*view).store_ivar::<i32>("_displayMode", 0);
    (*view).store_ivar::<i32>("_lang", 0);
    (*view).store_ivar::<u32>("_ownDisplayID", 0);

    // Visual defaults (overridden by prefs + sync)
    (*view).store_ivar::<f64>("_radius", DEFAULT_DIAMETER / 2.0);
    (*view).store_ivar::<f64>("_borderWidth", DEFAULT_BORDER_WIDTH);
    (*view).store_ivar::<f64>("_strokeR", DEFAULT_COLOR.0);
    (*view).store_ivar::<f64>("_strokeG", DEFAULT_COLOR.1);
    (*view).store_ivar::<f64>("_strokeB", DEFAULT_COLOR.2);
    (*view).store_ivar::<f64>("_strokeA", DEFAULT_COLOR.3);
    (*view).store_ivar::<f64>("_fillTransparencyPct", DEFAULT_FILL_TRANSPARENCY_PCT);

    // Carbon refs
    (*view).store_ivar::<*mut std::ffi::c_void>("_hkHandler", std::ptr::null_mut());
    (*view).store_ivar::<*mut std::ffi::c_void>("_hkToggle", std::ptr::null_mut());
    (*view).store_ivar::<*mut std::ffi::c_void>("_hkComma", std::ptr::null_mut());
    (*view).store_ivar::<*mut std::ffi::c_void>("_hkHelp", std::ptr::null_mut());
    (*view).store_ivar::<*mut std::ffi::c_void>("_hkQuit", std::ptr::null_mut());

    // Keep-alive timer ref
    (*view).store_ivar::<id>("_hkKeepAliveTimer", nil);

    // Mouse monitors
    (*view).store_ivar::<id>("_monLeftDown", nil);
    (*view).store_ivar::<id>("_monLeftUp", nil);
    (*view).store_ivar::<id>("_monRightDown", nil);
    (*view).store_ivar::<id>("_monRightUp", nil);
    (*view).store_ivar::<id>("_monMove", nil);

    // Settings UI refs
    (*view).store_ivar::<id>("_settingsWindow", nil);
    (*view).store_ivar::<id>("_labelLang", nil);
    (*view).store_ivar::<id>("_popupLang", nil);

    (*view).store_ivar::<id>("_labelRadius", nil);
    (*view).store_ivar::<id>("_fieldRadius", nil);
    (*view).store_ivar::<id>("_sliderRadius", nil);

    (*view).store_ivar::<id>("_labelBorder", nil);
    (*view).store_ivar::<id>("_fieldBorder", nil);
    (*view).store_ivar::<id>("_sliderBorder", nil);

    (*view).store_ivar::<id>("_labelColor", nil);
    (*view).store_ivar::<id>("_colorWell", nil);

    (*view).store_ivar::<id>("_labelHex", nil);
    (*view).store_ivar::<id>("_fieldHex", nil);

    (*view).store_ivar::<id>("_labelFillT", nil);
    (*view).store_ivar::<id>("_fieldFillT", nil);
    (*view).store_ivar::<id>("_sliderFillT", nil);

    (*view).store_ivar::<id>("_btnClose", nil);

    (*view).store_ivar::<id>("_localKeyMonitor", nil);
    (*view).store_ivar::<f64>("_lastToggleTs", 0.0);
    set_bool_ivar(view, "_menuInstalled", false);

    // Refresh timer
    (*view).store_ivar::<id>("_updateTimer", nil);

    // ESC monitor (settings)
    (*view).store_ivar::<id>("_settingsEscMonitor", nil);
}

// ============================================================================
// CustomView methods (extern "C-unwind" for Objective-C runtime)
// ============================================================================

/// Wrapper callback for reinstalling hotkeys after dialogs close.
unsafe fn reinstall_hotkeys_callback(view: id) {
    reinstall_hotkeys(view, hotkey_event_handler);
}

/// Process all pending events from the event bus.
unsafe fn process_pending_events(view: id) {
    dispatch_events(
        view,
        open_settings_window,
        confirm_and_maybe_quit,
        reinstall_hotkeys_callback,
    );
}

unsafe extern "C-unwind" fn update_cursor_multi(this: &mut AnyObject, _cmd: Sel) {
    // Process any pending events from the event bus
    process_pending_events(this as *mut _ as id);

    let (x, y) = get_mouse_position_cocoa();
    let screens: id = msg_send![get_class("NSScreen"), screens];
    let count: usize = msg_send![screens, count];

    // Work out the screen under the cursor using a stable DisplayID
    let mut target_id: u32 = 0;
    for i in 0..count {
        let s: id = msg_send![screens, objectAtIndex: i];
        let f: NSRect = msg_send![s, frame];
        if x >= f.origin.x
            && x <= f.origin.x + f.size.width
            && y >= f.origin.y
            && y <= f.origin.y + f.size.height
        {
            target_id = display_id_for_screen(s);
            break;
        }
    }

    let enabled = get_bool_ivar(this as *mut _ as id, "_overlayEnabled");

    apply_to_all_views(|v| {
        *(*v).load_ivar_mut::<f64>("_cursorXScreen") = x;
        *(*v).load_ivar_mut::<f64>("_cursorYScreen") = y;
        let own_id = *(*v).load_ivar::<u32>("_ownDisplayID");
        let vis = enabled && own_id == target_id && target_id != 0;
        set_bool_ivar(v, "_visible", vis);
        let _: () = msg_send![v, setNeedsDisplay: YES];
        let win: id = msg_send![v, window];
        let _: () = msg_send![win, displayIfNeeded];
    });
}

unsafe extern "C-unwind" fn toggle_visibility(this: &mut AnyObject, _cmd: Sel) {
    unsafe {
        let enabled = get_bool_ivar(this as *mut _ as id, "_overlayEnabled");
        let new_enabled = !enabled;

        apply_to_all_views(|v| {
            set_bool_ivar(v, "_overlayEnabled", new_enabled);
        });

        if new_enabled {
            let _: () = msg_send![
                this,
                performSelectorOnMainThread: sel!(update_cursor_multi),
                withObject: nil,
                waitUntilDone: NO
            ];
        } else {
            apply_to_all_views(|v| {
                set_bool_ivar(v, "_visible", false);
                let _: () = msg_send![v, setNeedsDisplay: YES];
                let win: id = msg_send![v, window];
                let _: () = msg_send![win, displayIfNeeded];
            });
        }
    }
}

// Debounced toggle used by the temporary menu / local key equivalents
unsafe extern "C-unwind" fn request_toggle(this: &mut AnyObject, _cmd: Sel) {
    unsafe {
        let now = CFAbsoluteTimeGetCurrent();
        let last = *this.load_ivar::<f64>("_lastToggleTs");
        if now - last < 0.15 {
            return;
        }
        apply_to_all_views(|v| {
            *(*v).load_ivar_mut::<f64>("_lastToggleTs") = now;
        });

        let _: () = msg_send![
            this,
            performSelectorOnMainThread: sel!(toggleVisibility),
            withObject: nil,
            waitUntilDone: NO
        ];
    }
}

// Hotkey keep-alive: periodically re-install (idempotent)
unsafe extern "C-unwind" fn hotkey_keepalive(this: &mut AnyObject, _cmd: Sel) {
    reinstall_hotkeys(this as *mut _ as id, hotkey_event_handler);
}

// ===== Settings actions (apply to ALL views) =====

unsafe extern "C-unwind" fn set_radius(this: &mut AnyObject, _cmd: Sel, sender: id) {
    unsafe {
        let mut v: f64 = msg_send![sender, doubleValue];
        // snap to 5 to match desired increments (no visual ticks)
        v = (v / 5.0).round() * 5.0;
        v = v.clamp(5.0, 200.0);

        // update non-interactive label immediately
        let field: id = *this.load_ivar("_fieldRadius");
        if field != nil {
            let _: () = msg_send![field, setStringValue: nsstring_id(&format!("{:.0}", v))];
        }

        prefs_set_double(PREF_RADIUS, v);
        apply_to_all_views(|vv| (*vv).store_ivar::<f64>("_radius", v));
        apply_to_all_views(|vv| {
            let _: () = msg_send![vv, setNeedsDisplay: YES];
        });
    }
}

// These are no-ops now (text fields became labels)
unsafe extern "C-unwind" fn set_radius_from_field(_this: &mut AnyObject, _cmd: Sel, _sender: id) {}

unsafe extern "C-unwind" fn set_border_width(this: &mut AnyObject, _cmd: Sel, sender: id) {
    unsafe {
        let mut v: f64 = msg_send![sender, doubleValue];
        v = v.round().clamp(1.0, 20.0); // integer steps

        let field: id = *this.load_ivar("_fieldBorder");
        if field != nil {
            let _: () = msg_send![field, setStringValue: nsstring_id(&format!("{:.0}", v))];
        }

        prefs_set_double(PREF_BORDER, v);
        apply_to_all_views(|vv| (*vv).store_ivar::<f64>("_borderWidth", v));
        apply_to_all_views(|vv| {
            let _: () = msg_send![vv, setNeedsDisplay: YES];
        });
    }
}

unsafe extern "C-unwind" fn set_border_from_field(_this: &mut AnyObject, _cmd: Sel, _sender: id) {}

unsafe extern "C-unwind" fn set_fill_transparency(this: &mut AnyObject, _cmd: Sel, sender: id) {
    unsafe {
        let mut v: f64 = msg_send![sender, doubleValue];
        v = (v / 5.0).round() * 5.0; // 5% steps
        v = v.clamp(0.0, 100.0);

        let field: id = *this.load_ivar("_fieldFillT");
        if field != nil {
            let _: () = msg_send![field, setStringValue: nsstring_id(&format!("{:.0}", v))];
        }

        prefs_set_double(PREF_FILL_TRANSPARENCY, v);
        apply_to_all_views(|vv| (*vv).store_ivar::<f64>("_fillTransparencyPct", v));
        apply_to_all_views(|vv| {
            let _: () = msg_send![vv, setNeedsDisplay: YES];
        });
    }
}

unsafe extern "C-unwind" fn set_fill_transparency_from_field(
    _this: &mut AnyObject,
    _cmd: Sel,
    _sender: id,
) {
}

unsafe extern "C-unwind" fn color_changed(this: &mut AnyObject, _cmd: Sel, sender: id) {
    unsafe {
        let color: id = msg_send![sender, color];
        let r: f64 = msg_send![color, redComponent];
        let g: f64 = msg_send![color, greenComponent];
        let b: f64 = msg_send![color, blueComponent];
        let a: f64 = msg_send![color, alphaComponent];

        prefs_set_double(PREF_STROKE_R, r);
        prefs_set_double(PREF_STROKE_G, g);
        prefs_set_double(PREF_STROKE_B, b);
        prefs_set_double(PREF_STROKE_A, a);

        apply_to_all_views(|vv| {
            (*vv).store_ivar::<f64>("_strokeR", r);
            (*vv).store_ivar::<f64>("_strokeG", g);
            (*vv).store_ivar::<f64>("_strokeB", b);
            (*vv).store_ivar::<f64>("_strokeA", a);
        });

        let hex_field: id = *this.load_ivar("_fieldHex");
        if hex_field != nil {
            let s = color_to_hex(r, g, b, a);
            let _: () = msg_send![hex_field, setStringValue: nsstring_id(&s)];
        }
        apply_to_all_views(|vv| {
            let _: () = msg_send![vv, setNeedsDisplay: YES];
        });
    }
}

unsafe extern "C-unwind" fn hex_changed(this: &mut AnyObject, _cmd: Sel, sender: id) {
    unsafe {
        let s: id = msg_send![sender, stringValue];
        let cstr_ptr: *const c_char = msg_send![s, UTF8String];
        if !cstr_ptr.is_null() {
            let txt = CStr::from_ptr(cstr_ptr).to_string_lossy();
            if let Some((r, g, b, a)) = parse_hex_color(&txt) {
                prefs_set_double(PREF_STROKE_R, r);
                prefs_set_double(PREF_STROKE_G, g);
                prefs_set_double(PREF_STROKE_B, b);
                prefs_set_double(PREF_STROKE_A, a);

                apply_to_all_views(|vv| {
                    (*vv).store_ivar::<f64>("_strokeR", r);
                    (*vv).store_ivar::<f64>("_strokeG", g);
                    (*vv).store_ivar::<f64>("_strokeB", b);
                    (*vv).store_ivar::<f64>("_strokeA", a);
                });

                let col: id = msg_send![
                    get_class("NSColor"),
                    colorWithCalibratedRed: r,
                    green: g,
                    blue: b,
                    alpha: a
                ];
                let well: id = *this.load_ivar("_colorWell");
                if well != nil {
                    let _: () = msg_send![well, setColor: col];
                }
                let norm = color_to_hex(r, g, b, a);
                let _: () = msg_send![sender, setStringValue: nsstring_id(&norm)];
                apply_to_all_views(|vv| {
                    let _: () = msg_send![vv, setNeedsDisplay: YES];
                });
            } else {
                let r = *this.load_ivar::<f64>("_strokeR");
                let g = *this.load_ivar::<f64>("_strokeG");
                let b = *this.load_ivar::<f64>("_strokeB");
                let a = *this.load_ivar::<f64>("_strokeA");
                let norm = color_to_hex(r, g, b, a);
                let _: () = msg_send![sender, setStringValue: nsstring_id(&norm)];
            }
        }
    }
}

unsafe extern "C-unwind" fn close_settings(this: &mut AnyObject, _cmd: Sel, _sender: id) {
    let view: id = this as *mut _ as id;
    close_settings_window(view);
}

// ===== Status bar menu actions =====

unsafe extern "C-unwind" fn status_bar_settings(_this: &mut AnyObject, _cmd: Sel, _sender: id) {
    // Publish OpenSettings event - dispatcher will handle it
    publish(AppEvent::OpenSettings);
}

unsafe extern "C-unwind" fn status_bar_about(_this: &mut AnyObject, _cmd: Sel, _sender: id) {
    // Publish ShowAbout event - dispatcher will handle it
    publish(AppEvent::ShowAbout);
}

unsafe extern "C-unwind" fn status_bar_help(_this: &mut AnyObject, _cmd: Sel, _sender: id) {
    // Publish ShowHelp event - dispatcher will handle it
    publish(AppEvent::ShowHelp);
}

unsafe extern "C-unwind" fn status_bar_quit(_this: &mut AnyObject, _cmd: Sel, _sender: id) {
    // Quit directly without confirmation dialog
    unsafe {
        let app: id = msg_send![get_class("NSApplication"), sharedApplication];
        let _: () = msg_send![app, terminate: nil];
    }
}

// Change language (0=en,1=es), update labels and Hex field layout
unsafe extern "C-unwind" fn lang_changed(this: &mut AnyObject, _cmd: Sel, sender: id) {
    unsafe {
        let idx: isize = msg_send![sender, indexOfSelectedItem];
        let new_lang = if idx == 1 { 1 } else { 0 };

        prefs_set_int(PREF_LANG, new_lang);
        apply_to_all_views(|v| (*v).store_ivar::<i32>("_lang", new_lang));

        let es = new_lang == 1;

        let settings: id = *this.load_ivar("_settingsWindow");
        if settings != nil {
            let _: () = msg_send![settings, setTitle: nsstring_id(tr_key("Settings", es).as_ref())];
        }

        let label_lang: id = *this.load_ivar("_labelLang");
        if label_lang != nil {
            let _: () = msg_send![
                label_lang,
                setStringValue: nsstring_id(tr_key("Language", es).as_ref())
            ];
        }

        let popup: id = *this.load_ivar("_popupLang");
        if popup != nil {
            let _: () = msg_send![popup, removeAllItems];
            let _: () = msg_send![
                popup,
                addItemWithTitle: nsstring_id(tr_key("English", es).as_ref())
            ];
            let _: () = msg_send![
                popup,
                addItemWithTitle: nsstring_id(tr_key("Spanish", es).as_ref())
            ];
            let _: () = msg_send![popup, selectItemAtIndex: if es { 1_isize } else { 0_isize }];
        }

        let lr: id = *this.load_ivar("_labelRadius");
        if lr != nil {
            let _: () =
                msg_send![lr, setStringValue: nsstring_id(tr_key("Radius (px)", es).as_ref())];
        }
        let lb: id = *this.load_ivar("_labelBorder");
        if lb != nil {
            let _: () =
                msg_send![lb, setStringValue: nsstring_id(tr_key("Border (px)", es).as_ref())];
        }
        let lc: id = *this.load_ivar("_labelColor");
        if lc != nil {
            let _: () = msg_send![lc, setStringValue: nsstring_id(tr_key("Color", es).as_ref())];
        }
        let lhex: id = *this.load_ivar("_labelHex");
        if lhex != nil {
            let _: () = msg_send![lhex, setStringValue: nsstring_id(tr_key("Hex", es).as_ref())];
            let _: () = msg_send![lhex, sizeToFit];

            let field_hex: id = *this.load_ivar("_fieldHex");
            if field_hex != nil && settings != nil {
                let wframe: NSRect = msg_send![settings, frame];
                let w = wframe.size.width;

                let label_hex_frame: NSRect = msg_send![lhex, frame];
                let padding: f64 = 8.0;
                let right_margin: f64 = 175.0;
                let field_x = label_hex_frame.origin.x + label_hex_frame.size.width + padding;
                let field_w = (w - right_margin) - field_x;

                let mut fh_frame: NSRect = msg_send![field_hex, frame];
                fh_frame.origin.x = field_x;
                fh_frame.size.width = field_w;
                let _: () = msg_send![field_hex, setFrame: fh_frame];
            }
        }
        let lfill: id = *this.load_ivar("_labelFillT");
        if lfill != nil {
            let _: () = msg_send![
                lfill,
                setStringValue: nsstring_id(tr_key("Fill Transparency (%)", es).as_ref())
            ];
        }

        let btn: id = *this.load_ivar("_btnClose");
        if btn != nil {
            let _: () = msg_send![btn, setTitle: nsstring_id(tr_key("Close", es).as_ref())];
        }

        // Update status bar menu language
        update_status_bar_language(this as *const _ as id);
    }
}

// Live delegate not required any more (value fields are labels)
unsafe extern "C-unwind" fn control_text_did_change(_this: &mut AnyObject, _cmd: Sel, _notif: id) {}

// ===== Drawing (circle or L/R letter) =====
unsafe extern "C-unwind" fn draw_rect(this: &AnyObject, _cmd: Sel, _rect: NSRect) {
    unsafe {
        let sx = *this.load_ivar::<f64>("_cursorXScreen");
        let sy = *this.load_ivar::<f64>("_cursorYScreen");
        let visible = get_bool_ivar(this as *const _ as id, "_visible");
        if !visible {
            return;
        }

        // Convert screen -> window -> view
        let screen_pt = NSPoint::new(sx, sy);
        let screen_rect = NSRect::new(screen_pt, NSSize::new(0.0, 0.0));
        let win: id = msg_send![this, window];
        let win_rect: NSRect = msg_send![win, convertRectFromScreen: screen_rect];
        let win_pt = win_rect.origin;
        let view_pt: NSPoint = msg_send![this, convertPoint: win_pt, fromView: nil];

        let mode = *this.load_ivar::<i32>("_displayMode");

        // Build drawing parameters from view ivars
        let params = DrawParams {
            center: view_pt,
            radius: *this.load_ivar::<f64>("_radius"),
            border_width: *this.load_ivar::<f64>("_borderWidth"),
            stroke_r: *this.load_ivar::<f64>("_strokeR"),
            stroke_g: *this.load_ivar::<f64>("_strokeG"),
            stroke_b: *this.load_ivar::<f64>("_strokeB"),
            stroke_a: *this.load_ivar::<f64>("_strokeA"),
            fill_transparency: *this.load_ivar::<f64>("_fillTransparencyPct"),
        };

        let es = *this.load_ivar::<i32>("_lang") == 1;
        match mode {
            0 => draw_circle(&params),
            1 => draw_letter(&params, ClickLetter::Left, es),
            _ => draw_letter(&params, ClickLetter::Right, es),
        }
    }
}
