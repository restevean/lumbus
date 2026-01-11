#![allow(unexpected_cfgs)] // Silence cfg warnings inside objc/cocoa macros

mod app;
mod ffi;
mod input;
mod ui;

use block::ConcreteBlock;
use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSColor, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask,
};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize};
use objc::runtime::{Class, Object, Sel};
use objc::{class, declare::ClassDecl, msg_send, sel, sel_impl};
// Import helpers from the library crate (tests use the same code)
use mouse_highlighter::{clamp, color_to_hex, parse_hex_color, tr_key};
// Import model constants and preferences
use mouse_highlighter::model::{
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
// Input handling (hotkeys)
use crate::input::{install_hotkeys, uninstall_hotkeys, reinstall_hotkeys};

//
// ===================== App =====================
//

fn main() {
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
        install_termination_observer(host_view);
        install_local_ctrl_a_monitor(host_view);

        // Defensive re-install of hotkeys on system events
        start_hotkey_keepalive(host_view);
        install_wakeup_space_observers(host_view);

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
// ===================== Settings window =====================
//

/// Install a minimal main menu so that local key equivalents work while Regular
fn ensure_hotkey_menu(view: id) {
    unsafe {
        let installed = *(*view).get_ivar::<bool>("_menuInstalled");
        if installed {
            return;
        }

        let main_menu: id = msg_send![class!(NSMenu), new];
        let app_item: id = msg_send![class!(NSMenuItem), new];
        let _: () = msg_send![main_menu, addItem: app_item];

        let app_menu: id = msg_send![class!(NSMenu), new];
        let _: () = msg_send![app_item, setSubmenu: app_menu];

        let mi_toggle: id = msg_send![class!(NSMenuItem), alloc];
        let mi_toggle: id = msg_send![
            mi_toggle,
            initWithTitle: nsstring("Toggle Overlay")
            action: sel!(requestToggle)
            keyEquivalent: nsstring("a")
        ];
        let ctrl_mask: u64 = 1 << 18; // Control
        let _: () = msg_send![mi_toggle, setKeyEquivalentModifierMask: ctrl_mask];
        let _: () = msg_send![mi_toggle, setTarget: view];
        let _: () = msg_send![app_menu, addItem: mi_toggle];

        let app = NSApp();
        let _: () = msg_send![app, setMainMenu: main_menu];

        (*view).set_ivar::<bool>("_menuInstalled", true);
    }
}

fn open_settings_window(view: id) {
    unsafe {
        let existing: id = *(*view).get_ivar::<id>("_settingsWindow");
        if existing != nil {
            let _: () = msg_send![existing, makeKeyAndOrderFront: nil];
            return;
        }

        // Save current overlay state and hide circle during settings
        let was_enabled = *(*view).get_ivar::<bool>("_overlayEnabled");
        apply_to_all_views(|v| {
            *(*v).get_mut_ivar::<bool>("_overlayEnabled") = false;
            *(*v).get_mut_ivar::<bool>("_visible") = false;
            let _: () = msg_send![v, setNeedsDisplay: YES];
        });

        let es = lang_is_es(view);

        let style = NSWindowStyleMask::NSTitledWindowMask
            | NSWindowStyleMask::NSClosableWindowMask;
        let w = 520.0;
        let h = 330.0;
        let settings = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(w, h)),
            style,
            NSBackingStoreType::NSBackingStoreBuffered,
            NO,
        );
        let _: () = msg_send![settings, setTitle: nsstring(tr_key("Settings", es).as_ref())];

        // High level and collection behavior to appear over fullscreen apps
        let _: () = msg_send![settings, setLevel: overlay_window_level()];
        // CanJoinAllSpaces (1) + FullScreenAuxiliary (256) = 257
        let _: () = msg_send![settings, setCollectionBehavior: 257u64];

        // Center on the screen where the cursor is (not just main screen)
        let mouse_loc: NSPoint = msg_send![class!(NSEvent), mouseLocation];
        let screens: id = msg_send![class!(NSScreen), screens];
        let screen_count: usize = msg_send![screens, count];
        let mut target_screen: id = msg_send![class!(NSScreen), mainScreen];

        for i in 0..screen_count {
            let scr: id = msg_send![screens, objectAtIndex: i];
            let frame: NSRect = msg_send![scr, frame];
            if mouse_loc.x >= frame.origin.x
                && mouse_loc.x < frame.origin.x + frame.size.width
                && mouse_loc.y >= frame.origin.y
                && mouse_loc.y < frame.origin.y + frame.size.height
            {
                target_screen = scr;
                break;
            }
        }

        let screen_frame: NSRect = msg_send![target_screen, frame];
        let settings_frame: NSRect = msg_send![settings, frame];
        let centered_x = screen_frame.origin.x + (screen_frame.size.width - settings_frame.size.width) / 2.0;
        let centered_y = screen_frame.origin.y + (screen_frame.size.height - settings_frame.size.height) / 2.0;
        let centered_origin = NSPoint { x: centered_x, y: centered_y };
        let _: () = msg_send![settings, setFrameOrigin: centered_origin];

        let content: id = msg_send![settings, contentView];

        let radius: f64 = *(*view).get_ivar::<f64>("_radius");
        let border: f64 = *(*view).get_ivar::<f64>("_borderWidth");
        let r: f64 = *(*view).get_ivar::<f64>("_strokeR");
        let g: f64 = *(*view).get_ivar::<f64>("_strokeG");
        let b: f64 = *(*view).get_ivar::<f64>("_strokeB");
        let a: f64 = *(*view).get_ivar::<f64>("_strokeA");
        let fill_t: f64 = *(*view).get_ivar::<f64>("_fillTransparencyPct");
        let cur_lang: i32 = *(*view).get_ivar::<i32>("_lang");

        // Helper: static label
        let mk_label = |x, y, text: &str| -> id {
            let lbl: id = msg_send![class!(NSTextField), alloc];
            let lbl: id = msg_send![
                lbl,
                initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(180.0, 20.0))
            ];
            let _: () = msg_send![lbl, setBezeled: NO];
            let _: () = msg_send![lbl, setDrawsBackground: NO];
            let _: () = msg_send![lbl, setEditable: NO];
            let _: () = msg_send![lbl, setSelectable: NO];
            let _: () = msg_send![lbl, setStringValue: nsstring(text)];
            lbl
        };

        // Helper: value as a non-interactive label
        let mk_value_label = |x, y, w, h, val: &str| -> id {
            let tf: id = msg_send![class!(NSTextField), alloc];
            let tf: id =
                msg_send![tf, initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))];
            let _: () = msg_send![tf, setBezeled: NO];
            let _: () = msg_send![tf, setDrawsBackground: NO];
            let _: () = msg_send![tf, setEditable: NO];
            let _: () = msg_send![tf, setSelectable: NO];
            let _: () = msg_send![tf, setStringValue: nsstring(val)];
            tf
        };

        // Language selector
        let label_lang = mk_label(20.0, h - 40.0, tr_key("Language", es).as_ref());
        let popup_lang: id = msg_send![class!(NSPopUpButton), alloc];
        let popup_lang: id = msg_send![
            popup_lang,
            initWithFrame: NSRect::new(NSPoint::new(160.0, h - 44.0), NSSize::new(160.0, 24.0))
        ];
        let _: () = msg_send![popup_lang, addItemWithTitle: nsstring(tr_key("English", es).as_ref())];
        let _: () = msg_send![popup_lang, addItemWithTitle: nsstring(tr_key("Spanish", es).as_ref())];
        let _: () = msg_send![popup_lang, selectItemAtIndex: (if cur_lang == 1 { 1 } else { 0 })];
        let _: () = msg_send![popup_lang, setTarget: view];
        let _: () = msg_send![popup_lang, setAction: sel!(langChanged:)];

        // Static labels
        let label_radius = mk_label(20.0, h - 80.0, tr_key("Radius (px)", es).as_ref());
        let label_border = mk_label(20.0, h - 130.0, tr_key("Border (px)", es).as_ref());
        let label_color = mk_label(20.0, h - 180.0, tr_key("Color", es).as_ref());
        let label_hex = mk_label(220.0, h - 180.0, tr_key("Hex", es).as_ref());
        let _: () = msg_send![label_hex, sizeToFit];
        let label_fill_t = mk_label(20.0, h - 230.0, tr_key("Fill Transparency (%)", es).as_ref());

        // Value labels (non-interactive) + sliders (NO tick marks, still snapping in code)
        // Radius
        let field_radius = mk_value_label(160.0, h - 84.0, 60.0, 24.0, &format!("{:.0}", radius));
        let slider_radius: id = msg_send![class!(NSSlider), alloc];
        let slider_radius: id = msg_send![
            slider_radius,
            initWithFrame: NSRect::new(NSPoint::new(230.0, h - 85.0), NSSize::new(260.0, 24.0))
        ];
        let _: () = msg_send![slider_radius, setMinValue: 5.0f64];
        let _: () = msg_send![slider_radius, setMaxValue: 200.0f64];
        let _: () = msg_send![slider_radius, setDoubleValue: radius];
        let _: () = msg_send![slider_radius, setTarget: view];
        let _: () = msg_send![slider_radius, setAction: sel!(setRadius:)];
        let _: () = msg_send![slider_radius, setContinuous: YES];

        // Border
        let field_border = mk_value_label(160.0, h - 134.0, 60.0, 24.0, &format!("{:.0}", border));
        let slider_border: id = msg_send![class!(NSSlider), alloc];
        let slider_border: id = msg_send![
            slider_border,
            initWithFrame: NSRect::new(NSPoint::new(230.0, h - 135.0), NSSize::new(260.0, 24.0))
        ];
        let _: () = msg_send![slider_border, setMinValue: 1.0f64];
        let _: () = msg_send![slider_border, setMaxValue: 20.0f64];
        let _: () = msg_send![slider_border, setDoubleValue: border];
        let _: () = msg_send![slider_border, setTarget: view];
        let _: () = msg_send![slider_border, setAction: sel!(setBorderWidth:)];
        let _: () = msg_send![slider_border, setContinuous: YES];

        // Colour + Hex (Hex remains editable)
        let color_well: id = msg_send![class!(NSColorWell), alloc];
        let color_well: id = msg_send![
            color_well,
            initWithFrame: NSRect::new(NSPoint::new(160.0, h - 185.0), NSSize::new(50.0, 25.0))
        ];
        let ns_color = Class::get("NSColor").unwrap();
        let current_color: id =
            msg_send![ns_color, colorWithCalibratedRed: r green: g blue: b alpha: a];
        let _: () = msg_send![color_well, setColor: current_color];
        let _: () = msg_send![color_well, setTarget: view];
        let _: () = msg_send![color_well, setAction: sel!(colorChanged:)];

        let hex_str = color_to_hex(r, g, b, a);
        // Place Hex field right after "Hex" label
        let label_hex_frame: NSRect = msg_send![label_hex, frame];
        let padding: f64 = 8.0;
        let right_margin: f64 = 175.0;
        let field_x = label_hex_frame.origin.x + label_hex_frame.size.width + padding;
        let field_w = (w - right_margin) - field_x;

        let field_hex: id = msg_send![class!(NSTextField), alloc];
        let field_hex: id = msg_send![
            field_hex,
            initWithFrame: NSRect::new(NSPoint::new(field_x, h - 185.0), NSSize::new(field_w, 24.0))
        ];
        let _: () = msg_send![field_hex, setStringValue: nsstring(&hex_str)];
        the_hex_field_config(view, field_hex);

        // Fill transparency
        let field_fill_t = mk_value_label(160.0, h - 234.0, 60.0, 24.0, &format!("{:.0}", fill_t));
        let slider_fill_t: id = msg_send![class!(NSSlider), alloc];
        let slider_fill_t: id = msg_send![
            slider_fill_t,
            initWithFrame: NSRect::new(NSPoint::new(230.0, h - 235.0), NSSize::new(260.0, 24.0))
        ];
        let _: () = msg_send![slider_fill_t, setMinValue: 0.0f64];
        let _: () = msg_send![slider_fill_t, setMaxValue: 100.0f64];
        let _: () = msg_send![slider_fill_t, setDoubleValue: fill_t];
        let _: () = msg_send![slider_fill_t, setTarget: view];
        let _: () = msg_send![slider_fill_t, setAction: sel!(setFillTransparency:)];
        let _: () = msg_send![slider_fill_t, setContinuous: YES];

        // Close button
        let btn_close: id = msg_send![class!(NSButton), alloc];
        let btn_close: id = msg_send![
            btn_close,
            initWithFrame: NSRect::new(NSPoint::new(w - 100.0, 15.0), NSSize::new(80.0, 28.0))
        ];
        let _: () = msg_send![btn_close, setTitle: nsstring(tr_key("Close", es).as_ref())];
        let _: () = msg_send![btn_close, setTarget: view];
        let _: () = msg_send![btn_close, setAction: sel!(closeSettings:)];

        // Enter/Return activates "Close"
        let _: () = msg_send![btn_close, setKeyEquivalent: nsstring("\r")];
        let cell: id = msg_send![btn_close, cell];
        let _: () = msg_send![settings, setDefaultButtonCell: cell];

        // Add subviews
        let _: () = msg_send![content, addSubview: label_lang];
        let _: () = msg_send![content, addSubview: popup_lang];

        let _: () = msg_send![content, addSubview: label_radius];
        let _: () = msg_send![content, addSubview: field_radius];
        let _: () = msg_send![content, addSubview: slider_radius];

        let _: () = msg_send![content, addSubview: label_border];
        let _: () = msg_send![content, addSubview: field_border];
        let _: () = msg_send![content, addSubview: slider_border];

        let _: () = msg_send![content, addSubview: label_color];
        let _: () = msg_send![content, addSubview: color_well];
        let _: () = msg_send![content, addSubview: label_hex];
        let _: () = msg_send![content, addSubview: field_hex];

        let _: () = msg_send![content, addSubview: label_fill_t];
        let _: () = msg_send![content, addSubview: field_fill_t];
        let _: () = msg_send![content, addSubview: slider_fill_t];

        let _: () = msg_send![content, addSubview: btn_close];

        // Save refs for later sync
        (*view).set_ivar::<id>("_settingsWindow", settings);
        (*view).set_ivar::<id>("_labelLang", label_lang);
        (*view).set_ivar::<id>("_popupLang", popup_lang);

        (*view).set_ivar::<id>("_labelRadius", label_radius);
        (*view).set_ivar::<id>("_fieldRadius", field_radius); // label
        (*view).set_ivar::<id>("_sliderRadius", slider_radius);

        (*view).set_ivar::<id>("_labelBorder", label_border);
        (*view).set_ivar::<id>("_fieldBorder", field_border); // label
        (*view).set_ivar::<id>("_sliderBorder", slider_border);

        (*view).set_ivar::<id>("_labelColor", label_color);
        (*view).set_ivar::<id>("_colorWell", color_well);

        (*view).set_ivar::<id>("_labelHex", label_hex);
        (*view).set_ivar::<id>("_fieldHex", field_hex); // editable

        (*view).set_ivar::<id>("_labelFillT", label_fill_t);
        (*view).set_ivar::<id>("_fieldFillT", field_fill_t); // label
        (*view).set_ivar::<id>("_sliderFillT", slider_fill_t);

        (*view).set_ivar::<id>("_btnClose", btn_close);

        // Local monitor for ESC/Enter to close modal
        const KEY_DOWN_MASK: u64 = 1 << 10;
        let key_block = ConcreteBlock::new(move |event: id| -> id {
            let keycode: u16 = msg_send![event, keyCode];
            if keycode == 53 || keycode == 36 {
                // 53 = Escape, 36 = Return/Enter - stop modal
                let app = NSApp();
                let _: () = msg_send![app, stopModal];
                return nil;
            }
            event
        })
        .copy();
        let key_mon: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: KEY_DOWN_MASK
            handler: &*key_block
        ];

        // Force activation and show window, then run modal
        let app = NSApp();
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];
        let _: () = msg_send![settings, makeKeyAndOrderFront: nil];
        let _: () = msg_send![app, runModalForWindow: settings];

        // Modal ended - clean up
        let _: () = msg_send![class!(NSEvent), removeMonitor: key_mon];
        let _: () = msg_send![settings, orderOut: nil];

        // Clear stored references
        (*view).set_ivar::<id>("_settingsWindow", nil);
        (*view).set_ivar::<id>("_settingsEscMonitor", nil);
        (*view).set_ivar::<id>("_settingsGlobalMonitor", nil);

        // Restore overlay if it was enabled
        if was_enabled {
            apply_to_all_views(|v| {
                *(*v).get_mut_ivar::<bool>("_overlayEnabled") = true;
            });
        }

        // Ensure overlays are in front and refreshed
        apply_to_all_views(|v| {
            let overlay_win: id = msg_send![v, window];
            let _: () = msg_send![overlay_win, setLevel: overlay_window_level()];
            let _: () = msg_send![overlay_win, orderFrontRegardless];
            let _: () = msg_send![
                v,
                performSelectorOnMainThread: sel!(update_cursor_multi)
                withObject: nil
                waitUntilDone: NO
            ];
        });

        reinstall_hotkeys(view, hotkey_event_handler);
    }
}

unsafe fn the_hex_field_config(view: id, field_hex: id) {
    let _: () = msg_send![field_hex, setBezeled: YES];
    let _: () = msg_send![field_hex, setDrawsBackground: YES];
    let _: () = msg_send![field_hex, setEditable: YES];
    let _: () = msg_send![field_hex, setSelectable: YES];
    let _: () = msg_send![field_hex, setTarget: view];
    let _: () = msg_send![field_hex, setAction: sel!(hexChanged:)];
}

fn close_settings_window(_view: id) {
    unsafe {
        // Just stop the modal - cleanup happens in open_settings_window after runModalForWindow returns
        let app = NSApp();
        let _: () = msg_send![app, stopModal];
    }
}

//
// ===================== Quit confirmation (Ctrl+Shift+X) =====================
//

/// Custom borderless quit dialog that can appear over fullscreen apps
fn confirm_and_maybe_quit(view: id) {
    unsafe {
        // Save current overlay state and hide circle during dialog
        let was_enabled = *(*view).get_ivar::<bool>("_overlayEnabled");
        apply_to_all_views(|v| {
            *(*v).get_mut_ivar::<bool>("_overlayEnabled") = false;
            *(*v).get_mut_ivar::<bool>("_visible") = false;
            let _: () = msg_send![v, setNeedsDisplay: YES];
        });

        let es = lang_is_es(view);

        // Dialog dimensions
        let dialog_w: f64 = 320.0;
        let dialog_h: f64 = 140.0;

        // Create BORDERLESS window (key to appearing over fullscreen apps)
        let dialog: id = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(dialog_w, dialog_h)),
            NSWindowStyleMask::NSBorderlessWindowMask,
            NSBackingStoreType::NSBackingStoreBuffered,
            NO,
        );

        // Configure window like overlay windows
        let _: () = msg_send![dialog, setOpaque: NO];
        let _: () = msg_send![dialog, setBackgroundColor: NSColor::clearColor(nil)];
        let _: () = msg_send![dialog, setLevel: overlay_window_level()];
        // CanJoinAllSpaces (1) + Stationary (16) + FullScreenAuxiliary (256) = 273
        let _: () = msg_send![dialog, setCollectionBehavior: 273u64];
        // Allow the window to receive mouse events (unlike overlays)
        let _: () = msg_send![dialog, setIgnoresMouseEvents: NO];

        // Find screen where cursor is
        let mouse_loc: NSPoint = msg_send![class!(NSEvent), mouseLocation];
        let screens: id = msg_send![class!(NSScreen), screens];
        let screen_count: usize = msg_send![screens, count];
        let mut target_screen: id = msg_send![class!(NSScreen), mainScreen];

        for i in 0..screen_count {
            let scr: id = msg_send![screens, objectAtIndex: i];
            let frame: NSRect = msg_send![scr, frame];
            if mouse_loc.x >= frame.origin.x
                && mouse_loc.x < frame.origin.x + frame.size.width
                && mouse_loc.y >= frame.origin.y
                && mouse_loc.y < frame.origin.y + frame.size.height
            {
                target_screen = scr;
                break;
            }
        }

        // Center dialog on target screen
        let screen_frame: NSRect = msg_send![target_screen, frame];
        let centered_x = screen_frame.origin.x + (screen_frame.size.width - dialog_w) / 2.0;
        let centered_y = screen_frame.origin.y + (screen_frame.size.height - dialog_h) / 2.0;
        let _: () = msg_send![dialog, setFrameOrigin: NSPoint { x: centered_x, y: centered_y }];

        // Create content view with rounded background
        let content: id = msg_send![dialog, contentView];

        // Background box (dark semi-transparent with rounded corners)
        let bg_box: id = msg_send![class!(NSBox), alloc];
        let bg_box: id = msg_send![bg_box, initWithFrame: NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(dialog_w, dialog_h)
        )];
        let _: () = msg_send![bg_box, setBoxType: 4i64]; // NSBoxCustom
        let _: () = msg_send![bg_box, setBorderType: 0i64]; // NSNoBorder
        let _: () = msg_send![bg_box, setCornerRadius: 12.0f64];
        let _: () = msg_send![bg_box, setFillColor: NSColor::colorWithSRGBRed_green_blue_alpha_(
            nil, 0.15, 0.15, 0.15, 0.95
        )];
        let _: () = msg_send![content, addSubview: bg_box];

        // Title label
        let title_text = if es { "¿Salir de la aplicación?" } else { "Quit the app?" };
        let title_label: id = msg_send![class!(NSTextField), alloc];
        let title_label: id = msg_send![title_label, initWithFrame: NSRect::new(
            NSPoint::new(20.0, dialog_h - 45.0),
            NSSize::new(dialog_w - 40.0, 30.0)
        )];
        let _: () = msg_send![title_label, setStringValue: nsstring(title_text)];
        let _: () = msg_send![title_label, setBezeled: NO];
        let _: () = msg_send![title_label, setDrawsBackground: NO];
        let _: () = msg_send![title_label, setEditable: NO];
        let _: () = msg_send![title_label, setSelectable: NO];
        let white_color: id = msg_send![class!(NSColor), whiteColor];
        let _: () = msg_send![title_label, setTextColor: white_color];
        let _: () = msg_send![title_label, setAlignment: 1i64]; // NSTextAlignmentCenter
        let bold_font: id = msg_send![class!(NSFont), boldSystemFontOfSize: 16.0f64];
        let _: () = msg_send![title_label, setFont: bold_font];
        let _: () = msg_send![content, addSubview: title_label];

        // Message label
        let msg_text = if es { "Se cerrará la app" } else { "The app will close" };
        let msg_label: id = msg_send![class!(NSTextField), alloc];
        let msg_label: id = msg_send![msg_label, initWithFrame: NSRect::new(
            NSPoint::new(20.0, dialog_h - 75.0),
            NSSize::new(dialog_w - 40.0, 20.0)
        )];
        let _: () = msg_send![msg_label, setStringValue: nsstring(msg_text)];
        let _: () = msg_send![msg_label, setBezeled: NO];
        let _: () = msg_send![msg_label, setDrawsBackground: NO];
        let _: () = msg_send![msg_label, setEditable: NO];
        let _: () = msg_send![msg_label, setSelectable: NO];
        let _: () = msg_send![msg_label, setTextColor: NSColor::colorWithSRGBRed_green_blue_alpha_(
            nil, 0.8, 0.8, 0.8, 1.0
        )];
        let _: () = msg_send![msg_label, setAlignment: 1i64]; // NSTextAlignmentCenter
        let _: () = msg_send![content, addSubview: msg_label];

        // Buttons
        let btn_w: f64 = 100.0;
        let btn_h: f64 = 32.0;
        let btn_y: f64 = 20.0;
        let btn_spacing: f64 = 20.0;
        let total_btn_w = btn_w * 2.0 + btn_spacing;
        let btn_start_x = (dialog_w - total_btn_w) / 2.0;

        // Button titles
        let cancel_title = tr_key("Cancel", es);
        let quit_title = tr_key("Quit", es);

        // Cancel button (left) - not focused initially (gray)
        let cancel_btn: id = msg_send![class!(NSButton), alloc];
        let cancel_btn: id = msg_send![cancel_btn, initWithFrame: NSRect::new(
            NSPoint::new(btn_start_x, btn_y),
            NSSize::new(btn_w, btn_h)
        )];
        let _: () = msg_send![cancel_btn, setTitle: nsstring(&cancel_title)];
        let _: () = msg_send![cancel_btn, setBordered: NO]; // Remove border
        let _: () = msg_send![cancel_btn, setKeyEquivalent: nsstring("\x1b")]; // Escape
        let _: () = msg_send![cancel_btn, setWantsLayer: YES];
        let cancel_layer: id = msg_send![cancel_btn, layer];
        // Darker gray for unfocused button
        let dark_gray: id = NSColor::colorWithSRGBRed_green_blue_alpha_(nil, 0.35, 0.35, 0.35, 1.0);
        let dark_gray_cg: *const std::ffi::c_void = msg_send![dark_gray, CGColor];
        let _: () = msg_send![cancel_layer, setBackgroundColor: dark_gray_cg];
        let _: () = msg_send![cancel_layer, setCornerRadius: 6.0f64];
        let _: () = msg_send![content, addSubview: cancel_btn];

        // Quit button (right) - focused initially (blue)
        let quit_btn: id = msg_send![class!(NSButton), alloc];
        let quit_btn: id = msg_send![quit_btn, initWithFrame: NSRect::new(
            NSPoint::new(btn_start_x + btn_w + btn_spacing, btn_y),
            NSSize::new(btn_w, btn_h)
        )];
        let _: () = msg_send![quit_btn, setTitle: nsstring(&quit_title)];
        let _: () = msg_send![quit_btn, setBordered: NO]; // Remove border
        let _: () = msg_send![quit_btn, setWantsLayer: YES];
        let quit_layer: id = msg_send![quit_btn, layer];
        let blue_cg: id = msg_send![class!(NSColor), systemBlueColor];
        let blue_cgcolor: *const std::ffi::c_void = msg_send![blue_cg, CGColor];
        let _: () = msg_send![quit_layer, setBackgroundColor: blue_cgcolor];
        let _: () = msg_send![quit_layer, setCornerRadius: 6.0f64];
        let _: () = msg_send![content, addSubview: quit_btn];

        // Response flag: 0=pending, 1=quit, 2=cancel
        let response_flag = Box::into_raw(Box::new(0i32));
        // Focus state: 0=quit button (default), 1=cancel button
        let focus_state = Box::into_raw(Box::new(0i32));

        // Button and layer references for closures
        let quit_btn_copy = quit_btn;
        let cancel_btn_copy = cancel_btn;
        let quit_layer_ref = quit_layer;
        let cancel_layer_ref = cancel_layer;

        // Local keyboard monitor for Enter, Escape, and Tab
        const KEY_DOWN_MASK: u64 = 1 << 10;
        let flag_ptr = response_flag;
        let focus_ptr = focus_state;
        let key_block = ConcreteBlock::new(move |event: id| -> id {
            let keycode: u16 = msg_send![event, keyCode];
            match keycode {
                36 => { // Enter/Return - activate focused button
                    let current_focus = *focus_ptr;
                    if current_focus == 0 {
                        *flag_ptr = 1; // Quit
                    } else {
                        *flag_ptr = 2; // Cancel
                    }
                    let app = NSApp();
                    let _: () = msg_send![app, stopModal];
                    return nil;
                }
                53 => { // Escape - always cancel
                    *flag_ptr = 2; // Cancel
                    let app = NSApp();
                    let _: () = msg_send![app, stopModal];
                    return nil;
                }
                48 | 123 | 124 => { // Tab, Left arrow, Right arrow - change focus
                    let current_focus = *focus_ptr;
                    let new_focus = match keycode {
                        123 => 1, // Left arrow -> Cancel (left button)
                        124 => 0, // Right arrow -> Quit (right button)
                        _ => if current_focus == 0 { 1 } else { 0 }, // Tab toggles
                    };

                    // Only update if focus actually changed
                    if new_focus != current_focus {
                        *focus_ptr = new_focus;

                        // Get colors for focus switching
                        let blue: id = msg_send![class!(NSColor), systemBlueColor];
                        let dark_gray: id = NSColor::colorWithSRGBRed_green_blue_alpha_(nil, 0.35, 0.35, 0.35, 1.0);
                        let blue_cg: *const std::ffi::c_void = msg_send![blue, CGColor];
                        let gray_cg: *const std::ffi::c_void = msg_send![dark_gray, CGColor];

                        // Update layer background colors
                        if new_focus == 0 {
                            // Focus on Quit (blue), Cancel gray
                            let _: () = msg_send![quit_layer_ref, setBackgroundColor: blue_cg];
                            let _: () = msg_send![cancel_layer_ref, setBackgroundColor: gray_cg];
                        } else {
                            // Focus on Cancel (blue), Quit gray
                            let _: () = msg_send![quit_layer_ref, setBackgroundColor: gray_cg];
                            let _: () = msg_send![cancel_layer_ref, setBackgroundColor: blue_cg];
                        }
                    }
                    return nil;
                }
                _ => {}
            }
            event
        }).copy();
        let key_mon: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: KEY_DOWN_MASK
            handler: &*key_block
        ];

        // Mouse click handler for buttons
        const LEFT_DOWN_MASK: u64 = 1 << 1;
        let flag_ptr2 = response_flag;
        let click_block = ConcreteBlock::new(move |event: id| -> id {
            let loc: NSPoint = msg_send![event, locationInWindow];
            // Check if click is on quit button
            let quit_frame: NSRect = msg_send![quit_btn_copy, frame];
            if loc.x >= quit_frame.origin.x && loc.x <= quit_frame.origin.x + quit_frame.size.width
                && loc.y >= quit_frame.origin.y && loc.y <= quit_frame.origin.y + quit_frame.size.height
            {
                *flag_ptr2 = 1; // Quit
                let app = NSApp();
                let _: () = msg_send![app, stopModal];
                return nil;
            }
            // Check if click is on cancel button
            let cancel_frame: NSRect = msg_send![cancel_btn_copy, frame];
            if loc.x >= cancel_frame.origin.x && loc.x <= cancel_frame.origin.x + cancel_frame.size.width
                && loc.y >= cancel_frame.origin.y && loc.y <= cancel_frame.origin.y + cancel_frame.size.height
            {
                *flag_ptr2 = 2; // Cancel
                let app = NSApp();
                let _: () = msg_send![app, stopModal];
                return nil;
            }
            event
        }).copy();
        let click_mon: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: LEFT_DOWN_MASK
            handler: &*click_block
        ];

        // Show dialog and run modal
        let _: () = msg_send![dialog, makeKeyAndOrderFront: nil];
        let app = NSApp();
        let _: () = msg_send![app, runModalForWindow: dialog];

        // Clean up monitors
        let _: () = msg_send![class!(NSEvent), removeMonitor: key_mon];
        let _: () = msg_send![class!(NSEvent), removeMonitor: click_mon];

        // Read response and clean up
        let response = *response_flag;
        let _ = Box::from_raw(response_flag); // Deallocate
        let _ = Box::from_raw(focus_state); // Deallocate

        // Close dialog
        let _: () = msg_send![dialog, orderOut: nil];

        // Handle response
        if response == 1 {
            // Quit selected
            let _: () = msg_send![app, terminate: nil];
            return;
        }

        // Cancel: restore overlay if it was enabled
        if was_enabled {
            apply_to_all_views(|v| {
                *(*v).get_mut_ivar::<bool>("_overlayEnabled") = true;
            });
            let _: () = msg_send![
                view,
                performSelectorOnMainThread: sel!(update_cursor_multi)
                withObject: nil
                waitUntilDone: NO
            ];
        }

        // Ensure overlays are back on top and hotkeys are solid
        apply_to_all_views(|v| {
            let overlay_win: id = msg_send![v, window];
            let _: () = msg_send![overlay_win, setLevel: overlay_window_level()];
            let _: () = msg_send![overlay_win, orderFrontRegardless];
        });
        reinstall_hotkeys(view, hotkey_event_handler);
    }
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
    window.setLevel_(overlay_window_level().into());
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

                let radius = *this.get_ivar::<f64>("_radius");
                let border_width = *this.get_ivar::<f64>("_borderWidth");
                let r = *this.get_ivar::<f64>("_strokeR");
                let g = *this.get_ivar::<f64>("_strokeG");
                let b = *this.get_ivar::<f64>("_strokeB");
                let a = *this.get_ivar::<f64>("_strokeA");
                let fill_t = *this.get_ivar::<f64>("_fillTransparencyPct");
                let mode = *this.get_ivar::<i32>("_displayMode");

                let ns_color = Class::get("NSColor").unwrap();

                if mode == 0 {
                    // Circle
                    let rect = NSRect::new(
                        NSPoint::new(view_pt.x - radius, view_pt.y - radius),
                        NSSize::new(radius * 2.0, radius * 2.0),
                    );
                    let ns_bezier = Class::get("NSBezierPath").unwrap();
                    let circle: id = msg_send![ns_bezier, bezierPathWithOvalInRect: rect];

                    // Fill
                    let fill_alpha = a * (1.0 - clamp(fill_t, 0.0, 100.0) / 100.0);
                    if fill_alpha > 0.0 {
                        let fill: id = msg_send![
                            ns_color,
                            colorWithCalibratedRed: r
                            green: g
                            blue: b
                            alpha: fill_alpha
                        ];
                        let _: () = msg_send![fill, set];
                        let _: () = msg_send![circle, fill];
                    }
                    // Stroke
                    let stroke: id =
                        msg_send![ns_color, colorWithCalibratedRed: r green: g blue: b alpha: a];
                    let _: () = msg_send![stroke, set];
                    let _: () = msg_send![circle, setLineWidth: border_width];
                    let _: () = msg_send![circle, stroke];
                    return;
                }

                // Letter L/R
                let target_letter_height = 3.0 * radius; // 1.5 × diameter
                let font_class = Class::get("NSFont").unwrap();
                let font: id = msg_send![font_class, boldSystemFontOfSize: target_letter_height];

                let font_name: id = msg_send![font, fontName];
                let ct_font: CTFontRef =
                    CTFontCreateWithName(font_name as *const _, target_letter_height, std::ptr::null());

                let ch_u16: u16 = if mode == 1 { 'L' as u16 } else { 'R' as u16 };
                let mut glyph: u16 = 0;
                let mapped = CTFontGetGlyphsForCharacters(
                    ct_font,
                    &ch_u16 as *const u16,
                    &mut glyph as *mut u16,
                    1,
                );
                if !mapped || glyph == 0 {
                    CFRelease(ct_font as *const _);
                    return;
                }
                let cg_path: CGPathRef = CTFontCreatePathForGlyph(ct_font, glyph, std::ptr::null());
                if cg_path.is_null() {
                    CFRelease(ct_font as *const _);
                    return;
                }

                let ns_bezier = Class::get("NSBezierPath").unwrap();
                let path: id = msg_send![ns_bezier, bezierPathWithCGPath: cg_path];

                let pbounds: NSRect = msg_send![path, bounds];
                let mid_x = pbounds.origin.x + pbounds.size.width / 2.0;
                let mid_y = pbounds.origin.y + pbounds.size.height / 2.0;

                let ns_affine = Class::get("NSAffineTransform").unwrap();
                let transform: id = msg_send![ns_affine, transform];
                let dx = view_pt.x - mid_x;
                let dy = view_pt.y - mid_y;
                let _: () = msg_send![transform, translateXBy: dx yBy: dy];
                let _: () = msg_send![path, transformUsingAffineTransform: transform];

                let _: () = msg_send![path, setLineJoinStyle: 1u64 /* round */];

                // Fill exactly like the circle
                let fill_alpha = a * (1.0 - clamp(fill_t, 0.0, 100.0) / 100.0);
                if fill_alpha > 0.0 {
                    let fill: id = msg_send![
                        ns_color,
                        colorWithCalibratedRed: r
                        green: g
                        blue: b
                        alpha: fill_alpha
                    ];
                    let _: () = msg_send![fill, set];
                    let _: () = msg_send![path, fill];
                }
                // Stroke
                let stroke: id =
                    msg_send![ns_color, colorWithCalibratedRed: r green: g blue: b alpha: a];
                let _: () = msg_send![stroke, set];
                let _: () = msg_send![path, setLineWidth: border_width];
                let _: () = msg_send![path, stroke];

                CGPathRelease(cg_path);
                CFRelease(ct_font as *const _);
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
unsafe fn create_timer(target: id, selector: Sel, interval: f64) -> id {
    let prev: id = *(*target).get_ivar::<id>("_updateTimer");
    if prev != nil {
        let _: () = msg_send![prev, invalidate];
        (*target).set_ivar::<id>("_updateTimer", nil);
    }
    let timer_class = Class::get("NSTimer").unwrap();
    let timer: id = msg_send![
        timer_class,
        scheduledTimerWithTimeInterval: interval
        target: target
        selector: selector
        userInfo: nil
        repeats: YES
    ];
    (*target).set_ivar::<id>("_updateTimer", timer);
    timer
}

//
// ===================== Hotkeys (Carbon) =====================
//

extern "C" fn hotkey_event_handler(
    _call_ref: EventHandlerCallRef,
    event: EventRef,
    user_data: *mut std::ffi::c_void,
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
                let view = user_data as id;
                match hot_id.id {
                    HKID_TOGGLE => {
                        // Toggle overlay (main thread)
                        let _: () = msg_send![
                            view,
                            performSelectorOnMainThread: sel!(requestToggle)
                            withObject: nil
                            waitUntilDone: NO
                        ];
                    }
                    HKID_SETTINGS_COMMA | HKID_SETTINGS_SEMI => {
                        let block = ConcreteBlock::new(move || {
                            open_settings_window(view);
                        })
                            .copy();
                        let main_queue: id = msg_send![class!(NSOperationQueue), mainQueue];
                        let _: () = msg_send![main_queue, addOperationWithBlock: &*block];
                    }
                    HKID_QUIT => {
                        let block = ConcreteBlock::new(move || {
                            confirm_and_maybe_quit(view);
                        })
                            .copy();
                        let main_queue: id = msg_send![class!(NSOperationQueue), mainQueue];
                        let _: () = msg_send![main_queue, addOperationWithBlock: &*block];
                    }
                    _ => {}
                }
            }
        }
        NO_ERR
    }
}

unsafe fn install_termination_observer(view: id) {
    // On app termination: clean Carbon resources
    let center: id = msg_send![class!(NSNotificationCenter), defaultCenter];
    let queue: id = nil; // main thread

    let block = ConcreteBlock::new(move |_note: id| {
        unsafe {
            uninstall_hotkeys(view);
        }
    })
        .copy();

    let name: id = msg_send![
        class!(NSString),
        stringWithUTF8String: b"NSApplicationWillTerminateNotification\0".as_ptr() as *const _
    ];
    let _: id =
        msg_send![center, addObserverForName: name object: nil queue: queue usingBlock: &*block];
}

//
// ===================== Hotkey keep-alive & wake/space observers =====================
//

/// Start a repeating NSTimer to periodically re-install hotkeys (defensive)
unsafe fn start_hotkey_keepalive(view: id) {
    // Clear previous timer if any
    let prev: id = *(*view).get_ivar::<id>("_hkKeepAliveTimer");
    if prev != nil {
        let _: () = msg_send![prev, invalidate];
        (*view).set_ivar::<id>("_hkKeepAliveTimer", nil);
    }

    // 60s interval; cheap operation
    let timer_class = Class::get("NSTimer").unwrap();
    let timer: id = msg_send![
        timer_class,
        scheduledTimerWithTimeInterval: 60.0f64
        target: view
        selector: sel!(hotkeyKeepAlive)
        userInfo: nil
        repeats: YES
    ];
    (*view).set_ivar::<id>("_hkKeepAliveTimer", timer);
}

/// Observe system events that may disrupt Carbon hotkeys and re-install on demand
unsafe fn install_wakeup_space_observers(view: id) {
    let ws: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let nc: id = msg_send![ws, notificationCenter];

    // Helper to add an observer for a given notification name (C string)
    let add_obs = |name_cstr: &'static [u8]| {
        let name: id =
            msg_send![class!(NSString), stringWithUTF8String: name_cstr.as_ptr() as *const _];
        let block = ConcreteBlock::new(move |_note: id| unsafe {
            reinstall_hotkeys(view, hotkey_event_handler);
        })
            .copy();
        let _: id = msg_send![nc, addObserverForName: name object: nil queue: nil usingBlock: &*block];
    };

    // Wake from sleep
    add_obs(b"NSWorkspaceDidWakeNotification\0");
    // Session became active (unlock/login)
    add_obs(b"NSWorkspaceSessionDidBecomeActiveNotification\0");
    // Active Space changed (Mission Control / Spaces)
    add_obs(b"NSWorkspaceActiveSpaceDidChangeNotification\0");
}

//
// ===================== Local Ctrl+A monitor =====================
//

unsafe fn install_local_ctrl_a_monitor(view: id) {
    let existing: id = *(*view).get_ivar::<id>("_localKeyMonitor");
    if existing != nil {
        return;
    }

    const KEY_DOWN_MASK: u64 = 1 << 10;
    const CTRL_FLAG: u64 = 1 << 18;
    const KEYCODE_A: u16 = 0;

    let host = view;
    let block = ConcreteBlock::new(move |event: id| {
        unsafe {
            let keycode: u16 = msg_send![event, keyCode];
            let flags: u64 = msg_send![event, modifierFlags];
            if keycode == KEYCODE_A && (flags & CTRL_FLAG) != 0 {
                let _: () = msg_send![
                    host,
                    performSelectorOnMainThread: sel!(requestToggle)
                    withObject: nil
                    waitUntilDone: NO
                ];
            }
        }
        event
    })
        .copy();

    let mon: id = msg_send![
        class!(NSEvent),
        addLocalMonitorForEventsMatchingMask: KEY_DOWN_MASK
        handler: &*block
    ];
    (*view).set_ivar::<id>("_localKeyMonitor", mon);
}

//
// ===================== Global mouse monitors =====================
//

unsafe fn install_mouse_monitors(view: id) {
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
                withObject: nil
                waitUntilDone: NO
            ];
        }
    })
        .copy();
    let mon_move: id =
        msg_send![cls, addGlobalMonitorForEventsMatchingMask: MOUSE_MOVED_MASK handler: &*hmove];
    (*view).set_ivar::<id>("_monMove", mon_move);
}

//
// ===================== TCC: Accessibility prompt =====================
//

unsafe fn ensure_accessibility_prompt() {
    // Create CFDictionary with kAXTrustedCheckOptionPrompt = true
    let keys = [kAXTrustedCheckOptionPrompt];
    let values = [kCFBooleanTrue];

    let dict = CFDictionaryCreate(
        std::ptr::null(),  // default allocator
        keys.as_ptr(),
        values.as_ptr(),
        1,  // one key-value pair
        kCFTypeDictionaryKeyCallBacks,
        kCFTypeDictionaryValueCallBacks,
    );

    let _trusted: bool = AXIsProcessTrustedWithOptions(dict);

    // Clean up
    if !dict.is_null() {
        CFRelease(dict);
    }
    // We ignore the boolean: if not trusted this triggers the system prompt.
}
