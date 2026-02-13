//! macOS-specific entry point and application logic.
//!
//! This module contains the main application loop for macOS,
//! including CustomView registration and overlay window creation.

use std::ffi::{c_char, CStr};

use lumbus::clamp;
use lumbus::color_to_hex;
use lumbus::events::{publish, AppEvent};
use lumbus::model::constants::*;
use lumbus::parse_hex_color;
use lumbus::platform::macos::app::{apply_to_all_views, sync_visual_prefs_to_all_views};
use lumbus::platform::macos::ffi::bridge::{
    autoreleasepool, get_class, id, msg_send, nil, nsstring_id, NSApp, ObjectExt, NO, YES,
};
use lumbus::platform::macos::ffi::{
    display_id_for_screen, ensure_accessibility_prompt, get_mouse_position_cocoa,
    overlay_window_level, CFAbsoluteTimeGetCurrent, EventHandlerCallRef, EventHotKeyID, EventRef,
    GetEventClass, GetEventKind, GetEventParameter, HKID_HELP, HKID_QUIT, HKID_SETTINGS_COMMA,
    HKID_TOGGLE, K_EVENT_CLASS_KEYBOARD, K_EVENT_HOTKEY_PRESSED, K_EVENT_PARAM_DIRECT_OBJECT,
    NO_ERR, SIG_MHLT, TYPE_EVENT_HOTKEY_ID,
};
use lumbus::platform::macos::handlers::dispatch_events;
use lumbus::platform::macos::input::{
    install_hotkeys, install_local_ctrl_a_monitor, install_mouse_monitors,
    install_termination_observer, install_wakeup_space_observers, reinstall_hotkeys,
    start_hotkey_keepalive,
};
use lumbus::platform::macos::storage::{
    prefs_get_double, prefs_get_int, prefs_set_double, prefs_set_int,
};
use lumbus::platform::macos::ui::{
    close_settings_window, confirm_and_maybe_quit, draw_circle, draw_letter, install_status_bar,
    open_settings_window, update_status_bar_language, ClickLetter, DrawParams,
};
use lumbus::tr_key;

use objc2::runtime::{AnyClass, AnyObject, ClassBuilder, Sel};
use objc2::sel;
use objc2_foundation::{NSPoint, NSRect, NSSize};

// Helper to store/load boolean as u8
unsafe fn get_bool_ivar(obj: id, name: &str) -> bool {
    let val = *(*obj).load_ivar::<u8>(name);
    val != 0
}

unsafe fn set_bool_ivar(obj: id, name: &str, val: bool) {
    (*obj).store_ivar::<u8>(name, if val { 1 } else { 0 });
}

/// Main entry point for macOS.
pub fn run() {
    // Event bus is already initialized by main()

    autoreleasepool(|| {
        unsafe {
            // Ask for Accessibility permission (shows a system prompt if needed)
            ensure_accessibility_prompt();

            let app = NSApp();
            // NSApplicationActivationPolicyAccessory = 1
            let _: bool = msg_send![app, setActivationPolicy: 1i64];

            // Create one transparent overlay window per screen
            let screens: id = msg_send![get_class("NSScreen"), screens];
            let count: usize = msg_send![screens, count];
            if count == 0 {
                eprintln!("No screens available.");
                return;
            }

            let mut views: Vec<id> = Vec::with_capacity(count);
            let mut windows: Vec<id> = Vec::with_capacity(count); // Keep windows alive
            for i in 0..count {
                let screen: id = msg_send![screens, objectAtIndex: i];
                let (win, view) = make_window_for_screen(screen);
                // CRITICAL: Retain window and view to prevent autorelease pool from deallocating them
                let _: id = msg_send![win, retain];
                let _: id = msg_send![view, retain];
                let _: () = msg_send![win, orderFrontRegardless];
                windows.push(win);
                views.push(view);
            }
            // Keep windows vector alive for the duration of the app
            std::mem::forget(windows);

            // Host view
            let host_view = *views.first().unwrap();

            // Load and broadcast preferences
            load_preferences_into_view(host_view);
            sync_visual_prefs_to_all_views(host_view);

            // ~60 FPS timer: updates cursor and visibility per screen
            let _ = create_timer(host_view, sel!(update_cursor_multi), 0.016);

            // Carbon hotkeys + global mouse monitors + termination observer
            install_hotkeys(host_view, hotkey_event_handler);
            install_mouse_monitors(host_view);
            install_termination_observer(host_view, hotkey_event_handler);
            install_local_ctrl_a_monitor(host_view);

            // Defensive re-install of hotkeys on system events
            start_hotkey_keepalive(host_view);
            install_wakeup_space_observers(host_view, hotkey_event_handler);

            // Status bar item in menu bar
            install_status_bar(host_view);

            let _: () = msg_send![app, run];
        }
    });
}

/// Load preferences into a view
unsafe fn load_preferences_into_view(view: id) {
    let radius = prefs_get_double(PREF_RADIUS, DEFAULT_DIAMETER / 2.0);
    let border = prefs_get_double(PREF_BORDER, DEFAULT_BORDER_WIDTH);
    let r = prefs_get_double(PREF_STROKE_R, DEFAULT_COLOR.0);
    let g = prefs_get_double(PREF_STROKE_G, DEFAULT_COLOR.1);
    let b = prefs_get_double(PREF_STROKE_B, DEFAULT_COLOR.2);
    let a = prefs_get_double(PREF_STROKE_A, DEFAULT_COLOR.3);
    let fill_t = prefs_get_double(PREF_FILL_TRANSPARENCY, DEFAULT_FILL_TRANSPARENCY_PCT);
    let lang = prefs_get_int(PREF_LANG, 0); // 0 en, 1 es

    (*view).store_ivar::<f64>("_radius", radius);
    (*view).store_ivar::<f64>("_borderWidth", border);
    (*view).store_ivar::<f64>("_strokeR", r);
    (*view).store_ivar::<f64>("_strokeG", g);
    (*view).store_ivar::<f64>("_strokeB", b);
    (*view).store_ivar::<f64>("_strokeA", a);
    (*view).store_ivar::<f64>("_fillTransparencyPct", clamp(fill_t, 0.0, 100.0));
    (*view).store_ivar::<i32>("_lang", if lang == 1 { 1 } else { 0 });
}

//
// ===================== Multi-monitor view =====================
//

unsafe fn make_window_for_screen(screen: id) -> (id, id) {
    let frame: NSRect = msg_send![screen, frame];

    // NSBorderlessWindowMask = 0
    let style_mask: u64 = 0;
    // NSBackingStoreBuffered = 2
    let backing: u64 = 2;

    let window: id = msg_send![get_class("NSWindow"), alloc];
    let window: id = msg_send![
        window,
        initWithContentRect: frame,
        styleMask: style_mask,
        backing: backing,
        defer: NO
    ];

    let _: () = msg_send![window, setOpaque: NO];

    // Get clear color
    let clear_color: id = msg_send![get_class("NSColor"), clearColor];
    let _: () = msg_send![window, setBackgroundColor: clear_color];

    let _: () = msg_send![window, setIgnoresMouseEvents: YES];
    let _: () = msg_send![window, setAcceptsMouseMovedEvents: YES];
    let _: () = msg_send![window, setLevel: overlay_window_level()];

    // NSWindowCollectionBehaviorCanJoinAllSpaces = 1 << 0 = 1
    // NSWindowCollectionBehaviorFullScreenAuxiliary = 1 << 8 = 256
    // NSWindowCollectionBehaviorStationary = 1 << 4 = 16
    let collection_behavior: u64 = 1 | 256 | 16;
    let _: () = msg_send![window, setCollectionBehavior: collection_behavior];

    let view: id =
        register_custom_view_class_and_create_view(window, frame.size.width, frame.size.height);
    (*view).store_ivar::<id>("_ownScreen", screen);
    // Store stable DisplayID for this view's screen
    let did = display_id_for_screen(screen);
    (*view).store_ivar::<u32>("_ownDisplayID", did);

    (window, view)
}

unsafe fn register_custom_view_class_and_create_view(window: id, width: f64, height: f64) -> id {
    let class_name = c"CustomViewMulti";
    let custom_view_class = if let Some(cls) = AnyClass::get(class_name) {
        cls
    } else {
        let superclass = AnyClass::get(c"NSView").unwrap();
        let mut builder = ClassBuilder::new(class_name, superclass).unwrap();

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

        // ====== Methods ======

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
                v = clamp(v, 5.0, 200.0);

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
        unsafe extern "C-unwind" fn set_radius_from_field(
            _this: &mut AnyObject,
            _cmd: Sel,
            _sender: id,
        ) {
        }
        unsafe extern "C-unwind" fn set_border_width(this: &mut AnyObject, _cmd: Sel, sender: id) {
            unsafe {
                let mut v: f64 = msg_send![sender, doubleValue];
                v = clamp(v.round(), 1.0, 20.0); // integer steps

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
        unsafe extern "C-unwind" fn set_border_from_field(
            _this: &mut AnyObject,
            _cmd: Sel,
            _sender: id,
        ) {
        }
        unsafe extern "C-unwind" fn set_fill_transparency(
            this: &mut AnyObject,
            _cmd: Sel,
            sender: id,
        ) {
            unsafe {
                let mut v: f64 = msg_send![sender, doubleValue];
                v = (v / 5.0).round() * 5.0; // 5% steps
                v = clamp(v, 0.0, 100.0);

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

        unsafe extern "C-unwind" fn status_bar_settings(
            _this: &mut AnyObject,
            _cmd: Sel,
            _sender: id,
        ) {
            // Publish OpenSettings event - dispatcher will handle it
            publish(AppEvent::OpenSettings);
        }

        unsafe extern "C-unwind" fn status_bar_about(
            _this: &mut AnyObject,
            _cmd: Sel,
            _sender: id,
        ) {
            // Publish ShowAbout event - dispatcher will handle it
            publish(AppEvent::ShowAbout);
        }

        unsafe extern "C-unwind" fn status_bar_help(_this: &mut AnyObject, _cmd: Sel, _sender: id) {
            // Publish ShowHelp event - dispatcher will handle it
            publish(AppEvent::ShowHelp);
        }

        unsafe extern "C-unwind" fn status_bar_quit(_this: &mut AnyObject, _cmd: Sel, _sender: id) {
            // Quit directly without confirmation dialog
            let app: id = msg_send![get_class("NSApplication"), sharedApplication];
            let _: () = msg_send![app, terminate: nil];
        }

        // Change language (0=en,1=es), update labels and Hex field layout
        unsafe extern "C-unwind" fn lang_changed(this: &mut AnyObject, _cmd: Sel, sender: id) {
            unsafe {
                let idx: i32 = msg_send![sender, indexOfSelectedItem];
                let new_lang = if idx == 1 { 1 } else { 0 };

                prefs_set_int(PREF_LANG, new_lang);
                apply_to_all_views(|v| (*v).store_ivar::<i32>("_lang", new_lang));

                let es = new_lang == 1;

                let settings: id = *this.load_ivar("_settingsWindow");
                if settings != nil {
                    let _: () =
                        msg_send![settings, setTitle: nsstring_id(tr_key("Settings", es).as_ref())];
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
                    let _: () = msg_send![popup, selectItemAtIndex: (if es { 1 } else { 0 })];
                }

                let lr: id = *this.load_ivar("_labelRadius");
                if lr != nil {
                    let _: () = msg_send![lr, setStringValue: nsstring_id(tr_key("Radius (px)", es).as_ref())];
                }
                let lb: id = *this.load_ivar("_labelBorder");
                if lb != nil {
                    let _: () = msg_send![lb, setStringValue: nsstring_id(tr_key("Border (px)", es).as_ref())];
                }
                let lc: id = *this.load_ivar("_labelColor");
                if lc != nil {
                    let _: () =
                        msg_send![lc, setStringValue: nsstring_id(tr_key("Color", es).as_ref())];
                }
                let lhex: id = *this.load_ivar("_labelHex");
                if lhex != nil {
                    let _: () =
                        msg_send![lhex, setStringValue: nsstring_id(tr_key("Hex", es).as_ref())];
                    let _: () = msg_send![lhex, sizeToFit];

                    let field_hex: id = *this.load_ivar("_fieldHex");
                    if field_hex != nil && settings != nil {
                        let wframe: NSRect = msg_send![settings, frame];
                        let w = wframe.size.width;

                        let label_hex_frame: NSRect = msg_send![lhex, frame];
                        let padding: f64 = 8.0;
                        let right_margin: f64 = 175.0;
                        let field_x =
                            label_hex_frame.origin.x + label_hex_frame.size.width + padding;
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
        unsafe extern "C-unwind" fn control_text_did_change(
            _this: &mut AnyObject,
            _cmd: Sel,
            _notif: id,
        ) {
        }

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

        // Register methods (including no-op live delegate)
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

        builder.add_method(
            sel!(langChanged:),
            lang_changed as unsafe extern "C-unwind" fn(_, _, _),
        );
        builder.add_method(
            sel!(controlTextDidChange:),
            control_text_did_change as unsafe extern "C-unwind" fn(_, _, _),
        );
        builder.add_method(
            sel!(drawRect:),
            draw_rect as unsafe extern "C-unwind" fn(_, _, _),
        );

        builder.register()
    };

    let view: id = msg_send![custom_view_class, alloc];
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
    let view: id = msg_send![view, initWithFrame: frame];

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

    let _: () = msg_send![window, setContentView: view];
    view
}

// AppKit timer (reliable even after panels / UI mode changes)
// Uses NSRunLoopCommonModes so the timer keeps firing during modal menus
unsafe fn create_timer(target: id, selector: Sel, interval: f64) -> id {
    let prev: id = *(*target).load_ivar::<id>("_updateTimer");
    if prev != nil {
        let _: () = msg_send![prev, invalidate];
        (*target).store_ivar::<id>("_updateTimer", nil);
    }
    let timer_class = get_class("NSTimer");
    // Create timer without auto-scheduling
    let timer: id = msg_send![
        timer_class,
        timerWithTimeInterval: interval,
        target: target,
        selector: selector,
        userInfo: nil,
        repeats: YES
    ];
    // Add to run loop with CommonModes (keeps running during menus)
    let run_loop: id = msg_send![get_class("NSRunLoop"), currentRunLoop];
    let common_modes = nsstring_id("kCFRunLoopCommonModes");
    let _: () = msg_send![run_loop, addTimer: timer, forMode: common_modes];

    (*target).store_ivar::<id>("_updateTimer", timer);
    timer
}

//
// ===================== Hotkeys (Carbon) =====================
//

/// Wrapper callback for reinstalling hotkeys after dialogs close.
/// This is passed to UI functions that need to reinstall hotkeys on close.
unsafe fn reinstall_hotkeys_callback(view: id) {
    reinstall_hotkeys(view, hotkey_event_handler);
}

/// Process all pending events from the event bus.
///
/// This is called from the timer (60fps) to dispatch queued events.
/// It's a global function so it can be called from the CustomView method.
unsafe fn process_pending_events(view: id) {
    dispatch_events(
        view,
        open_settings_window,
        confirm_and_maybe_quit,
        reinstall_hotkeys_callback,
    );
}

extern "C" fn hotkey_event_handler(
    _call_ref: EventHandlerCallRef,
    event: EventRef,
    _user_data: *mut std::ffi::c_void,
) -> i32 {
    unsafe {
        if GetEventClass(event) == K_EVENT_CLASS_KEYBOARD
            && GetEventKind(event) == K_EVENT_HOTKEY_PRESSED
        {
            let mut hot_id = EventHotKeyID {
                signature: 0,
                id: 0,
            };
            let status = GetEventParameter(
                event,
                K_EVENT_PARAM_DIRECT_OBJECT,
                TYPE_EVENT_HOTKEY_ID,
                std::ptr::null_mut(),
                std::mem::size_of::<EventHotKeyID>() as u32,
                std::ptr::null_mut(),
                &mut hot_id as *mut _ as *mut std::ffi::c_void,
            );
            if status == NO_ERR && hot_id.signature == SIG_MHLT {
                // Publish events to the bus - they'll be processed in the main loop
                match hot_id.id {
                    HKID_TOGGLE => {
                        publish(AppEvent::ToggleOverlay);
                    }
                    HKID_SETTINGS_COMMA => {
                        publish(AppEvent::OpenSettings);
                    }
                    HKID_HELP => {
                        publish(AppEvent::ShowHelp);
                    }
                    HKID_QUIT => {
                        publish(AppEvent::RequestQuit);
                    }
                    _ => {}
                }
            }
        }
        NO_ERR
    }
}
