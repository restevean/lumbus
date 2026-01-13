#![allow(unexpected_cfgs)] // Silence cfg warnings inside objc/cocoa macros

mod app;
mod ffi;
mod handlers;
mod input;
mod ui;

use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use objc::runtime::{Class, Object, Sel};
use objc::{class, declare::ClassDecl, msg_send, sel, sel_impl};
// Import helpers from the library crate (tests use the same code)
use lumbus::{clamp, color_to_hex, parse_hex_color, tr_key};
// Import event system
use lumbus::events::{init_event_bus, publish, AppEvent};
// Import model constants and preferences
use lumbus::model::{
    DEFAULT_DIAMETER, DEFAULT_BORDER_WIDTH, DEFAULT_COLOR, DEFAULT_FILL_TRANSPARENCY_PCT,
    PREF_RADIUS, PREF_BORDER, PREF_STROKE_R, PREF_STROKE_G, PREF_STROKE_B, PREF_STROKE_A,
    PREF_FILL_TRANSPARENCY, PREF_LANG,
    prefs_get_double, prefs_set_double, prefs_get_int, prefs_set_int,
};
use std::ffi::CStr;

// FFI bindings from local module
use crate::ffi::*;
// Global helpers (apply_to_all_views, etc.)
use crate::app::*;
// Input handling (hotkeys, observers, monitors)
use crate::input::{
    install_hotkeys, reinstall_hotkeys,
    install_termination_observer, start_hotkey_keepalive, install_wakeup_space_observers,
    install_local_ctrl_a_monitor, install_mouse_monitors,
};
// UI components
use crate::ui::{
    confirm_and_maybe_quit, open_settings_window, close_settings_window,
    install_status_bar, update_status_bar_language,
    DrawParams, ClickLetter, draw_circle, draw_letter,
};
// Event dispatcher
use crate::handlers::dispatch_events;

//
// ===================== App =====================
//

fn main() {
    // Initialize event bus before anything else
    init_event_bus();

    unsafe {
        let _pool = NSAutoreleasePool::new(nil);

        // Ask for Accessibility permission (shows a system prompt if needed)
        ensure_accessibility_prompt();

        let app = NSApp();
        app.setActivationPolicy_(
            NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
        );

        // Create one transparent overlay window per screen
        let screens: id = msg_send![class!(NSScreen), screens];
        let count: usize = msg_send![screens, count];
        if count == 0 {
            eprintln!("No screens available.");
            return;
        }

        let mut views: Vec<id> = Vec::with_capacity(count);
        for i in 0..count {
            let screen: id = msg_send![screens, objectAtIndex: i];
            let (win, view) = make_window_for_screen(screen);
            let _: () = msg_send![win, orderFrontRegardless];
            views.push(view);
        }

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

        app.run();
    }
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

    (*view).set_ivar::<f64>("_radius", radius);
    (*view).set_ivar::<f64>("_borderWidth", border);
    (*view).set_ivar::<f64>("_strokeR", r);
    (*view).set_ivar::<f64>("_strokeG", g);
    (*view).set_ivar::<f64>("_strokeB", b);
    (*view).set_ivar::<f64>("_strokeA", a);
    (*view).set_ivar::<f64>("_fillTransparencyPct", clamp(fill_t, 0.0, 100.0));
    (*view).set_ivar::<i32>("_lang", if lang == 1 { 1 } else { 0 });
}

//
// ===================== Multi-monitor view =====================
//

unsafe fn make_window_for_screen(screen: id) -> (id, id) {
    let frame: NSRect = msg_send![screen, frame];

    let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
        frame,
        NSWindowStyleMask::NSBorderlessWindowMask,
        NSBackingStoreType::NSBackingStoreBuffered,
        NO,
    );
    window.setOpaque_(NO);
    window.setBackgroundColor_(NSColor::clearColor(nil));
    window.setIgnoresMouseEvents_(YES);
    window.setAcceptsMouseMovedEvents_(YES); // allow mouse move events
    window.setLevel_(overlay_window_level());
    window.setCollectionBehavior_(
        NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary,
    );

    let view: id =
        register_custom_view_class_and_create_view(window, frame.size.width, frame.size.height);
    (*view).set_ivar::<id>("_ownScreen", screen);
    // Store stable DisplayID for this view's screen
    let did = display_id_for_screen(screen);
    (*view).set_ivar::<u32>("_ownDisplayID", did);

    (window, view)
}

unsafe fn register_custom_view_class_and_create_view(window: id, width: f64, height: f64) -> id {
    let class_name = "CustomViewMulti";
    let custom_view_class = if let Some(cls) = Class::get(class_name) {
        cls
    } else {
        let superclass = Class::get("NSView").unwrap();
        let mut decl = ClassDecl::new(class_name, superclass).unwrap();

        // Base state
        decl.add_ivar::<f64>("_cursorXScreen");
        decl.add_ivar::<f64>("_cursorYScreen");
        decl.add_ivar::<bool>("_visible"); // visible by screen selection
        decl.add_ivar::<bool>("_overlayEnabled"); // global toggle
        decl.add_ivar::<i32>("_displayMode"); // 0=circle, 1=L, 2=R
        decl.add_ivar::<id>("_ownScreen"); // owning NSScreen
        decl.add_ivar::<u32>("_ownDisplayID"); // stable DisplayID
        decl.add_ivar::<i32>("_lang"); // 0=en, 1=es

        // Visual parameters
        decl.add_ivar::<f64>("_radius");
        decl.add_ivar::<f64>("_borderWidth");
        decl.add_ivar::<f64>("_strokeR");
        decl.add_ivar::<f64>("_strokeG");
        decl.add_ivar::<f64>("_strokeB");
        decl.add_ivar::<f64>("_strokeA");
        decl.add_ivar::<f64>("_fillTransparencyPct"); // 0..100

        // Carbon refs
        decl.add_ivar::<*mut std::ffi::c_void>("_hkHandler");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkToggle");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkComma");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkSemi");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkHelp");
        decl.add_ivar::<*mut std::ffi::c_void>("_hkQuit");

        // Keep-alive timer for hotkeys
        decl.add_ivar::<id>("_hkKeepAliveTimer");

        // Global mouse monitors
        decl.add_ivar::<id>("_monLeftDown");
        decl.add_ivar::<id>("_monLeftUp");
        decl.add_ivar::<id>("_monRightDown");
        decl.add_ivar::<id>("_monRightUp");
        decl.add_ivar::<id>("_monMove");

        // Settings UI refs
        decl.add_ivar::<id>("_settingsWindow");
        decl.add_ivar::<id>("_labelLang");
        decl.add_ivar::<id>("_popupLang");

        decl.add_ivar::<id>("_labelRadius");
        decl.add_ivar::<id>("_fieldRadius"); // now a label
        decl.add_ivar::<id>("_sliderRadius");

        decl.add_ivar::<id>("_labelBorder");
        decl.add_ivar::<id>("_fieldBorder"); // now a label
        decl.add_ivar::<id>("_sliderBorder");

        decl.add_ivar::<id>("_labelColor");
        decl.add_ivar::<id>("_colorWell");

        decl.add_ivar::<id>("_labelHex");
        decl.add_ivar::<id>("_fieldHex"); // remains editable

        decl.add_ivar::<id>("_labelFillT");
        decl.add_ivar::<id>("_fieldFillT"); // now a label
        decl.add_ivar::<id>("_sliderFillT");

        decl.add_ivar::<id>("_btnClose");
        decl.add_ivar::<id>("_previousApp"); // App to restore focus to when closing settings

        // Local key monitor (for Ctrl+A redundancy)
        decl.add_ivar::<id>("_localKeyMonitor");

        // Debounce toggles
        decl.add_ivar::<f64>("_lastToggleTs");

        // Menu installed flag
        decl.add_ivar::<bool>("_menuInstalled");

        // Refresh timer
        decl.add_ivar::<id>("_updateTimer");

        // Monitors used while settings window is open (ESC/Enter)
        decl.add_ivar::<id>("_settingsEscMonitor");
        decl.add_ivar::<id>("_settingsGlobalMonitor");

        // ====== Methods ======

        extern "C" fn update_cursor_multi(this: &mut Object, _cmd: Sel) {
            unsafe {
                // Process any pending events from the event bus
                process_pending_events(this as *mut _ as id);

                let (x, y) = get_mouse_position_cocoa();
                let screens: id = msg_send![class!(NSScreen), screens];
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

                let enabled = *this.get_ivar::<bool>("_overlayEnabled");

                apply_to_all_views(|v| {
                    *(*v).get_mut_ivar::<f64>("_cursorXScreen") = x;
                    *(*v).get_mut_ivar::<f64>("_cursorYScreen") = y;
                    let own_id = *(*v).get_ivar::<u32>("_ownDisplayID");
                    let vis = enabled && own_id == target_id && target_id != 0;
                    *(*v).get_mut_ivar::<bool>("_visible") = vis;
                    let _: () = msg_send![v, setNeedsDisplay: YES];
                    let win: id = msg_send![v, window];
                    let _: () = msg_send![win, displayIfNeeded];
                });
            }
        }

        extern "C" fn toggle_visibility(this: &mut Object, _cmd: Sel) {
            unsafe {
                let enabled = *this.get_ivar::<bool>("_overlayEnabled");
                let new_enabled = !enabled;

                apply_to_all_views(|v| {
                    *(*v).get_mut_ivar::<bool>("_overlayEnabled") = new_enabled;
                });

                if new_enabled {
                    let _: () = msg_send![
                        this,
                        performSelectorOnMainThread: sel!(update_cursor_multi)
                        withObject: nil
                        waitUntilDone: NO
                    ];
                } else {
                    apply_to_all_views(|v| {
                        *(*v).get_mut_ivar::<bool>("_visible") = false;
                        let _: () = msg_send![v, setNeedsDisplay: YES];
                        let win: id = msg_send![v, window];
                        let _: () = msg_send![win, displayIfNeeded];
                    });
                }
            }
        }

        // Debounced toggle used by the temporary menu / local key equivalents
        extern "C" fn request_toggle(this: &mut Object, _cmd: Sel) {
            unsafe {
                let now = CFAbsoluteTimeGetCurrent();
                let last = *this.get_ivar::<f64>("_lastToggleTs");
                if now - last < 0.15 {
                    return;
                }
                apply_to_all_views(|v| {
                    *(*v).get_mut_ivar::<f64>("_lastToggleTs") = now;
                });

                let _: () = msg_send![
                    this,
                    performSelectorOnMainThread: sel!(toggleVisibility)
                    withObject: nil
                    waitUntilDone: NO
                ];
            }
        }

        // Hotkey keep-alive: periodically re-install (idempotent)
        extern "C" fn hotkey_keepalive(this: &mut Object, _cmd: Sel) {
            unsafe {
                reinstall_hotkeys(this as *mut _ as id, hotkey_event_handler);
            }
        }

        // ===== Settings actions (apply to ALL views) =====
        extern "C" fn set_radius(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let mut v: f64 = msg_send![sender, doubleValue];
                // snap to 5 to match desired increments (no visual ticks)
                v = (v / 5.0).round() * 5.0;
                v = clamp(v, 5.0, 200.0);

                // update non-interactive label immediately
                let field: id = *this.get_ivar("_fieldRadius");
                if field != nil {
                    let _: () = msg_send![field, setStringValue: nsstring(&format!("{:.0}", v))];
                }

                prefs_set_double(PREF_RADIUS, v);
                apply_to_all_views(|vv| { (*vv).set_ivar::<f64>("_radius", v) });
                apply_to_all_views(|vv| { let _: () = msg_send![vv, setNeedsDisplay: YES]; });
            }
        }
        // These are no-ops now (text fields became labels)
        extern "C" fn set_radius_from_field(_this: &mut Object, _cmd: Sel, _sender: id) {}
        extern "C" fn set_border_width(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let mut v: f64 = msg_send![sender, doubleValue];
                v = clamp(v.round(), 1.0, 20.0); // integer steps

                let field: id = *this.get_ivar("_fieldBorder");
                if field != nil {
                    let _: () = msg_send![field, setStringValue: nsstring(&format!("{:.0}", v))];
                }

                prefs_set_double(PREF_BORDER, v);
                apply_to_all_views(|vv| { (*vv).set_ivar::<f64>("_borderWidth", v) });
                apply_to_all_views(|vv| { let _: () = msg_send![vv, setNeedsDisplay: YES]; });
            }
        }
        extern "C" fn set_border_from_field(_this: &mut Object, _cmd: Sel, _sender: id) {}
        extern "C" fn set_fill_transparency(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let mut v: f64 = msg_send![sender, doubleValue];
                v = (v / 5.0).round() * 5.0; // 5% steps
                v = clamp(v, 0.0, 100.0);

                let field: id = *this.get_ivar("_fieldFillT");
                if field != nil {
                    let _: () = msg_send![field, setStringValue: nsstring(&format!("{:.0}", v))];
                }

                prefs_set_double(PREF_FILL_TRANSPARENCY, v);
                apply_to_all_views(|vv| { (*vv).set_ivar::<f64>("_fillTransparencyPct", v) });
                apply_to_all_views(|vv| { let _: () = msg_send![vv, setNeedsDisplay: YES]; });
            }
        }
        extern "C" fn set_fill_transparency_from_field(_this: &mut Object, _cmd: Sel, _sender: id) {}
        extern "C" fn color_changed(this: &mut Object, _cmd: Sel, sender: id) {
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
                    (*vv).set_ivar::<f64>("_strokeR", r);
                    (*vv).set_ivar::<f64>("_strokeG", g);
                    (*vv).set_ivar::<f64>("_strokeB", b);
                    (*vv).set_ivar::<f64>("_strokeA", a);
                });

                let hex_field: id = *this.get_ivar("_fieldHex");
                if hex_field != nil {
                    let s = color_to_hex(r, g, b, a);
                    let _: () = msg_send![hex_field, setStringValue: nsstring(&s)];
                }
                apply_to_all_views(|vv| { let _: () = msg_send![vv, setNeedsDisplay: YES]; });
            }
        }
        extern "C" fn hex_changed(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let s: id = msg_send![sender, stringValue];
                let cstr_ptr: *const std::os::raw::c_char = msg_send![s, UTF8String];
                if !cstr_ptr.is_null() {
                    let txt = CStr::from_ptr(cstr_ptr).to_string_lossy();
                    if let Some((r, g, b, a)) = parse_hex_color(&txt) {
                        prefs_set_double(PREF_STROKE_R, r);
                        prefs_set_double(PREF_STROKE_G, g);
                        prefs_set_double(PREF_STROKE_B, b);
                        prefs_set_double(PREF_STROKE_A, a);

                        apply_to_all_views(|vv| {
                            (*vv).set_ivar::<f64>("_strokeR", r);
                            (*vv).set_ivar::<f64>("_strokeG", g);
                            (*vv).set_ivar::<f64>("_strokeB", b);
                            (*vv).set_ivar::<f64>("_strokeA", a);
                        });

                        let ns_color = Class::get("NSColor").unwrap();
                        let col: id = msg_send![
                            ns_color,
                            colorWithCalibratedRed: r
                            green: g
                            blue: b
                            alpha: a
                        ];
                        let well: id = *this.get_ivar("_colorWell");
                        if well != nil {
                            let _: () = msg_send![well, setColor: col];
                        }
                        let norm = color_to_hex(r, g, b, a);
                        let _: () = msg_send![sender, setStringValue: nsstring(&norm)];
                        apply_to_all_views(|vv| { let _: () = msg_send![vv, setNeedsDisplay: YES]; });
                    } else {
                        let r = *this.get_ivar::<f64>("_strokeR");
                        let g = *this.get_ivar::<f64>("_strokeG");
                        let b = *this.get_ivar::<f64>("_strokeB");
                        let a = *this.get_ivar::<f64>("_strokeA");
                        let norm = color_to_hex(r, g, b, a);
                        let _: () = msg_send![sender, setStringValue: nsstring(&norm)];
                    }
                }
            }
        }
        extern "C" fn close_settings(this: &mut Object, _cmd: Sel, _sender: id) {
            let view: id = this as *mut _ as id;
            close_settings_window(view);
        }

        // ===== Status bar menu actions =====

        extern "C" fn status_bar_settings(_this: &mut Object, _cmd: Sel, _sender: id) {
            // Publish OpenSettings event - dispatcher will handle it
            publish(AppEvent::OpenSettings);
        }

        extern "C" fn status_bar_about(_this: &mut Object, _cmd: Sel, _sender: id) {
            // Publish ShowAbout event - dispatcher will handle it
            publish(AppEvent::ShowAbout);
        }

        extern "C" fn status_bar_help(_this: &mut Object, _cmd: Sel, _sender: id) {
            // Publish ShowHelp event - dispatcher will handle it
            publish(AppEvent::ShowHelp);
        }

        extern "C" fn status_bar_quit(_this: &mut Object, _cmd: Sel, _sender: id) {
            // Quit directly without confirmation dialog
            unsafe {
                let app: id = msg_send![class!(NSApplication), sharedApplication];
                let _: () = msg_send![app, terminate: nil];
            }
        }

        // Change language (0=en,1=es), update labels and Hex field layout
        extern "C" fn lang_changed(this: &mut Object, _cmd: Sel, sender: id) {
            unsafe {
                let idx: i32 = msg_send![sender, indexOfSelectedItem];
                let new_lang = if idx == 1 { 1 } else { 0 };

                prefs_set_int(PREF_LANG, new_lang);
                apply_to_all_views(|v| { (*v).set_ivar::<i32>("_lang", new_lang) });

                let es = new_lang == 1;

                let settings: id = *this.get_ivar("_settingsWindow");
                if settings != nil {
                    let _: () =
                        msg_send![settings, setTitle: nsstring(tr_key("Settings", es).as_ref())];
                }

                let label_lang: id = *this.get_ivar("_labelLang");
                if label_lang != nil {
                    let _: () = msg_send![
                        label_lang,
                        setStringValue: nsstring(tr_key("Language", es).as_ref())
                    ];
                }

                let popup: id = *this.get_ivar("_popupLang");
                if popup != nil {
                    let _: () = msg_send![popup, removeAllItems];
                    let _: () = msg_send![
                        popup,
                        addItemWithTitle: nsstring(tr_key("English", es).as_ref())
                    ];
                    let _: () = msg_send![
                        popup,
                        addItemWithTitle: nsstring(tr_key("Spanish", es).as_ref())
                    ];
                    let _: () = msg_send![popup, selectItemAtIndex: (if es { 1 } else { 0 })];
                }

                let lr: id = *this.get_ivar("_labelRadius");
                if lr != nil {
                    let _: () =
                        msg_send![lr, setStringValue: nsstring(tr_key("Radius (px)", es).as_ref())];
                }
                let lb: id = *this.get_ivar("_labelBorder");
                if lb != nil {
                    let _: () =
                        msg_send![lb, setStringValue: nsstring(tr_key("Border (px)", es).as_ref())];
                }
                let lc: id = *this.get_ivar("_labelColor");
                if lc != nil {
                    let _: () =
                        msg_send![lc, setStringValue: nsstring(tr_key("Color", es).as_ref())];
                }
                let lhex: id = *this.get_ivar("_labelHex");
                if lhex != nil {
                    let _: () =
                        msg_send![lhex, setStringValue: nsstring(tr_key("Hex", es).as_ref())];
                    let _: () = msg_send![lhex, sizeToFit];

                    let field_hex: id = *this.get_ivar("_fieldHex");
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
                let lfill: id = *this.get_ivar("_labelFillT");
                if lfill != nil {
                    let _: () = msg_send![
                        lfill,
                        setStringValue: nsstring(tr_key("Fill Transparency (%)", es).as_ref())
                    ];
                }

                let btn: id = *this.get_ivar("_btnClose");
                if btn != nil {
                    let _: () = msg_send![btn, setTitle: nsstring(tr_key("Close", es).as_ref())];
                }

                // Update status bar menu language
                update_status_bar_language(this as *const _ as id);
            }
        }

        // Live delegate not required any more (value fields are labels)
        extern "C" fn control_text_did_change(_this: &mut Object, _cmd: Sel, _notif: id) {}

        // ===== Drawing (circle or L/R letter) =====
        extern "C" fn draw_rect(this: &Object, _cmd: Sel, _rect: NSRect) {
            unsafe {
                let sx = *this.get_ivar::<f64>("_cursorXScreen");
                let sy = *this.get_ivar::<f64>("_cursorYScreen");
                let visible = *this.get_ivar::<bool>("_visible");
                if !visible {
                    return;
                }

                // Convert screen → window → view
                let screen_pt = NSPoint::new(sx, sy);
                let screen_rect = NSRect::new(screen_pt, NSSize::new(0.0, 0.0));
                let win: id = msg_send![this, window];
                let win_rect: NSRect = msg_send![win, convertRectFromScreen: screen_rect];
                let win_pt = win_rect.origin;
                let view_pt: NSPoint = msg_send![this, convertPoint: win_pt fromView: nil];

                let mode = *this.get_ivar::<i32>("_displayMode");

                // Build drawing parameters from view ivars
                let params = DrawParams {
                    center: view_pt,
                    radius: *this.get_ivar::<f64>("_radius"),
                    border_width: *this.get_ivar::<f64>("_borderWidth"),
                    stroke_r: *this.get_ivar::<f64>("_strokeR"),
                    stroke_g: *this.get_ivar::<f64>("_strokeG"),
                    stroke_b: *this.get_ivar::<f64>("_strokeB"),
                    stroke_a: *this.get_ivar::<f64>("_strokeA"),
                    fill_transparency: *this.get_ivar::<f64>("_fillTransparencyPct"),
                };

                match mode {
                    0 => draw_circle(&params),
                    1 => draw_letter(&params, ClickLetter::Left),
                    _ => draw_letter(&params, ClickLetter::Right),
                }
            }
        }

        // Register methods (including no-op live delegate)
        decl.add_method(
            sel!(update_cursor_multi),
            update_cursor_multi as extern "C" fn(&mut Object, Sel),
        );
        decl.add_method(
            sel!(toggleVisibility),
            toggle_visibility as extern "C" fn(&mut Object, Sel),
        );
        decl.add_method(
            sel!(requestToggle),
            request_toggle as extern "C" fn(&mut Object, Sel),
        );
        decl.add_method(
            sel!(hotkeyKeepAlive),
            hotkey_keepalive as extern "C" fn(&mut Object, Sel),
        );

        decl.add_method(sel!(setRadius:), set_radius as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(
            sel!(setRadiusFromField:),
            set_radius_from_field as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(setBorderWidth:),
            set_border_width as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(setBorderFromField:),
            set_border_from_field as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(setFillTransparency:),
            set_fill_transparency as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(setFillTransparencyFromField:),
            set_fill_transparency_from_field as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(sel!(colorChanged:), color_changed as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(sel!(hexChanged:), hex_changed as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(
            sel!(closeSettings:),
            close_settings as extern "C" fn(&mut Object, Sel, id),
        );

        // Status bar menu actions
        decl.add_method(
            sel!(statusBarSettings:),
            status_bar_settings as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(statusBarHelp:),
            status_bar_help as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(statusBarAbout:),
            status_bar_about as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(
            sel!(statusBarQuit:),
            status_bar_quit as extern "C" fn(&mut Object, Sel, id),
        );

        decl.add_method(sel!(langChanged:), lang_changed as extern "C" fn(&mut Object, Sel, id));
        decl.add_method(
            sel!(controlTextDidChange:),
            control_text_did_change as extern "C" fn(&mut Object, Sel, id),
        );
        decl.add_method(sel!(drawRect:), draw_rect as extern "C" fn(&Object, Sel, NSRect));

        decl.register()
    };

    let view: id = msg_send![custom_view_class, alloc];
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
    let view: id = msg_send![view, initWithFrame: frame];

    // Initial state
    (*view).set_ivar::<f64>("_cursorXScreen", 0.0);
    (*view).set_ivar::<f64>("_cursorYScreen", 0.0);
    (*view).set_ivar::<bool>("_visible", false);
    (*view).set_ivar::<bool>("_overlayEnabled", true); // Circle visible on app start
    (*view).set_ivar::<i32>("_displayMode", 0);
    (*view).set_ivar::<i32>("_lang", 0);
    (*view).set_ivar::<u32>("_ownDisplayID", 0);

    // Visual defaults (overridden by prefs + sync)
    (*view).set_ivar::<f64>("_radius", DEFAULT_DIAMETER / 2.0);
    (*view).set_ivar::<f64>("_borderWidth", DEFAULT_BORDER_WIDTH);
    (*view).set_ivar::<f64>("_strokeR", DEFAULT_COLOR.0);
    (*view).set_ivar::<f64>("_strokeG", DEFAULT_COLOR.1);
    (*view).set_ivar::<f64>("_strokeB", DEFAULT_COLOR.2);
    (*view).set_ivar::<f64>("_strokeA", DEFAULT_COLOR.3);
    (*view).set_ivar::<f64>("_fillTransparencyPct", DEFAULT_FILL_TRANSPARENCY_PCT);

    // Carbon refs
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkHandler", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkToggle", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkComma", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkSemi", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkHelp", std::ptr::null_mut());
    (*view).set_ivar::<*mut std::ffi::c_void>("_hkQuit", std::ptr::null_mut());

    // Keep-alive timer ref
    (*view).set_ivar::<id>("_hkKeepAliveTimer", nil);

    // Mouse monitors
    (*view).set_ivar::<id>("_monLeftDown", nil);
    (*view).set_ivar::<id>("_monLeftUp", nil);
    (*view).set_ivar::<id>("_monRightDown", nil);
    (*view).set_ivar::<id>("_monRightUp", nil);
    (*view).set_ivar::<id>("_monMove", nil);

    // Settings UI refs
    (*view).set_ivar::<id>("_settingsWindow", nil);
    (*view).set_ivar::<id>("_labelLang", nil);
    (*view).set_ivar::<id>("_popupLang", nil);

    (*view).set_ivar::<id>("_labelRadius", nil);
    (*view).set_ivar::<id>("_fieldRadius", nil);
    (*view).set_ivar::<id>("_sliderRadius", nil);

    (*view).set_ivar::<id>("_labelBorder", nil);
    (*view).set_ivar::<id>("_fieldBorder", nil);
    (*view).set_ivar::<id>("_sliderBorder", nil);

    (*view).set_ivar::<id>("_labelColor", nil);
    (*view).set_ivar::<id>("_colorWell", nil);

    (*view).set_ivar::<id>("_labelHex", nil);
    (*view).set_ivar::<id>("_fieldHex", nil);

    (*view).set_ivar::<id>("_labelFillT", nil);
    (*view).set_ivar::<id>("_fieldFillT", nil);
    (*view).set_ivar::<id>("_sliderFillT", nil);

    (*view).set_ivar::<id>("_btnClose", nil);

    (*view).set_ivar::<id>("_localKeyMonitor", nil);
    (*view).set_ivar::<f64>("_lastToggleTs", 0.0);
    (*view).set_ivar::<bool>("_menuInstalled", false);

    // Refresh timer
    (*view).set_ivar::<id>("_updateTimer", nil);

    // ESC monitor (settings)
    (*view).set_ivar::<id>("_settingsEscMonitor", nil);

    let _: () = msg_send![window, setContentView: view];
    view
}

// AppKit timer (reliable even after panels / UI mode changes)
// Uses NSRunLoopCommonModes so the timer keeps firing during modal menus
unsafe fn create_timer(target: id, selector: Sel, interval: f64) -> id {
    let prev: id = *(*target).get_ivar::<id>("_updateTimer");
    if prev != nil {
        let _: () = msg_send![prev, invalidate];
        (*target).set_ivar::<id>("_updateTimer", nil);
    }
    let timer_class = Class::get("NSTimer").unwrap();
    // Create timer without auto-scheduling
    let timer: id = msg_send![
        timer_class,
        timerWithTimeInterval: interval
        target: target
        selector: selector
        userInfo: nil
        repeats: YES
    ];
    // Add to run loop with CommonModes (keeps running during menus)
    let run_loop: id = msg_send![class!(NSRunLoop), currentRunLoop];
    let common_modes = NSString::alloc(nil).init_str("kCFRunLoopCommonModes");
    let _: () = msg_send![run_loop, addTimer: timer forMode: common_modes];

    (*target).set_ivar::<id>("_updateTimer", timer);
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
            let mut hot_id = EventHotKeyID { signature: 0, id: 0 };
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
                    HKID_SETTINGS_COMMA | HKID_SETTINGS_SEMI => {
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


