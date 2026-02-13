//! Settings window management.
//!
//! This module contains functions for opening and managing the settings window.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::platform::macos::ffi::bridge::{
    get_bool_ivar, get_class, id, msg_send, nil, nsstring_id, sel, set_bool_ivar, NSApp, NSPoint,
    NSRect, NSSize, ObjectExt, NO, YES,
};
use block2::RcBlock;

use crate::events::{publish, AppEvent};
use crate::platform::macos::app::{apply_to_all_views, lang_is_es};
use crate::platform::macos::ffi::overlay_window_level;
use crate::{color_to_hex, tr_key};

/// Guard to prevent multiple settings windows
static SETTINGS_OPENING: AtomicBool = AtomicBool::new(false);

/// Configure a hex color text field.
unsafe fn configure_hex_field(view: id, field_hex: id) {
    let _: () = msg_send![field_hex, setBezeled: YES];
    let _: () = msg_send![field_hex, setDrawsBackground: YES];
    let _: () = msg_send![field_hex, setEditable: YES];
    let _: () = msg_send![field_hex, setSelectable: YES];
    let _: () = msg_send![field_hex, setTarget: view];
    let _: () = msg_send![field_hex, setAction: sel!(hexChanged:)];
}

/// Open the settings window.
///
/// When the window closes, publishes `AppEvent::SettingsClosed` to the event bus.
/// The dispatcher handles hotkey reinstallation.
///
/// # Safety
/// - `view` must be a valid, non-null pointer to a CustomViewMulti.
/// - Must be called from main thread with valid autorelease pool.
pub unsafe fn open_settings_window(view: id) {
    // Atomic guard: only one settings window can be opening at a time
    let was_false = SETTINGS_OPENING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok();

    if !was_false {
        return;
    }

    // Save current overlay state and hide circle during settings
    let was_enabled = get_bool_ivar(view, "_overlayEnabled");
    apply_to_all_views(|v| {
        set_bool_ivar(v, "_overlayEnabled", false);
        set_bool_ivar(v, "_visible", false);
        let _: () = msg_send![v, setNeedsDisplay: YES];
    });

    let es = lang_is_es(view);

    // NSTitledWindowMask (1) | NSClosableWindowMask (2) = 3
    let style: u64 = 1 | 2;
    let w = 520.0;
    let h = 330.0;

    let settings: id = msg_send![get_class("NSWindow"), alloc];
    let settings: id = msg_send![
        settings,
        initWithContentRect: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(w, h)),
        styleMask: style,
        backing: 2u64,  // NSBackingStoreBuffered
        defer: NO
    ];
    let _: () = msg_send![settings, setTitle: nsstring_id(tr_key("Settings", es).as_ref())];

    // High level and collection behavior to appear over fullscreen apps
    let _: () = msg_send![settings, setLevel: overlay_window_level()];
    // CanJoinAllSpaces (1) + FullScreenAuxiliary (256) = 257
    let _: () = msg_send![settings, setCollectionBehavior: 257u64];

    // Center on the screen where the cursor is (not just main screen)
    let mouse_loc: NSPoint = msg_send![get_class("NSEvent"), mouseLocation];
    let screens: id = msg_send![get_class("NSScreen"), screens];
    let screen_count: usize = msg_send![screens, count];
    let mut target_screen: id = msg_send![get_class("NSScreen"), mainScreen];

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
    let centered_x =
        screen_frame.origin.x + (screen_frame.size.width - settings_frame.size.width) / 2.0;
    let centered_y =
        screen_frame.origin.y + (screen_frame.size.height - settings_frame.size.height) / 2.0;
    let centered_origin = NSPoint {
        x: centered_x,
        y: centered_y,
    };
    let _: () = msg_send![settings, setFrameOrigin: centered_origin];

    let content: id = msg_send![settings, contentView];

    let radius: f64 = *(*view).load_ivar::<f64>("_radius");
    let border: f64 = *(*view).load_ivar::<f64>("_borderWidth");
    let r: f64 = *(*view).load_ivar::<f64>("_strokeR");
    let g: f64 = *(*view).load_ivar::<f64>("_strokeG");
    let b: f64 = *(*view).load_ivar::<f64>("_strokeB");
    let a: f64 = *(*view).load_ivar::<f64>("_strokeA");
    let fill_t: f64 = *(*view).load_ivar::<f64>("_fillTransparencyPct");
    let cur_lang: i32 = *(*view).load_ivar::<i32>("_lang");

    // Helper: static label
    let mk_label = |x, y, text: &str| -> id {
        let lbl: id = msg_send![get_class("NSTextField"), alloc];
        let lbl: id = msg_send![
            lbl,
            initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(180.0, 20.0))
        ];
        let _: () = msg_send![lbl, setBezeled: NO];
        let _: () = msg_send![lbl, setDrawsBackground: NO];
        let _: () = msg_send![lbl, setEditable: NO];
        let _: () = msg_send![lbl, setSelectable: NO];
        let _: () = msg_send![lbl, setStringValue: nsstring_id(text)];
        lbl
    };

    // Helper: value as a non-interactive label
    let mk_value_label = |x, y, w_lbl, h_lbl, val: &str| -> id {
        let tf: id = msg_send![get_class("NSTextField"), alloc];
        let tf: id = msg_send![tf, initWithFrame: NSRect::new(NSPoint::new(x, y), NSSize::new(w_lbl, h_lbl))];
        let _: () = msg_send![tf, setBezeled: NO];
        let _: () = msg_send![tf, setDrawsBackground: NO];
        let _: () = msg_send![tf, setEditable: NO];
        let _: () = msg_send![tf, setSelectable: NO];
        let _: () = msg_send![tf, setStringValue: nsstring_id(val)];
        tf
    };

    // Language selector
    let label_lang = mk_label(20.0, h - 40.0, tr_key("Language", es).as_ref());
    let popup_lang: id = msg_send![get_class("NSPopUpButton"), alloc];
    let popup_lang: id = msg_send![
        popup_lang,
        initWithFrame: NSRect::new(NSPoint::new(160.0, h - 44.0), NSSize::new(160.0, 24.0))
    ];
    let _: () =
        msg_send![popup_lang, addItemWithTitle: nsstring_id(tr_key("English", es).as_ref())];
    let _: () =
        msg_send![popup_lang, addItemWithTitle: nsstring_id(tr_key("Spanish", es).as_ref())];
    let _: () = msg_send![popup_lang, selectItemAtIndex: (if cur_lang == 1 { 1i64 } else { 0i64 })];
    let _: () = msg_send![popup_lang, setTarget: view];
    let _: () = msg_send![popup_lang, setAction: sel!(langChanged:)];

    // Static labels
    let label_radius = mk_label(20.0, h - 80.0, tr_key("Radius (px)", es).as_ref());
    let label_border = mk_label(20.0, h - 130.0, tr_key("Border (px)", es).as_ref());
    let label_color = mk_label(20.0, h - 180.0, tr_key("Color", es).as_ref());
    let label_hex = mk_label(220.0, h - 180.0, tr_key("Hex", es).as_ref());
    let _: () = msg_send![label_hex, sizeToFit];
    let label_fill_t = mk_label(
        20.0,
        h - 230.0,
        tr_key("Fill Transparency (%)", es).as_ref(),
    );

    // Value labels (non-interactive) + sliders (NO tick marks, still snapping in code)
    // Radius
    let field_radius = mk_value_label(160.0, h - 84.0, 60.0, 24.0, &format!("{:.0}", radius));
    let slider_radius: id = msg_send![get_class("NSSlider"), alloc];
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
    let slider_border: id = msg_send![get_class("NSSlider"), alloc];
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
    let color_well: id = msg_send![get_class("NSColorWell"), alloc];
    let color_well: id = msg_send![
        color_well,
        initWithFrame: NSRect::new(NSPoint::new(160.0, h - 185.0), NSSize::new(50.0, 25.0))
    ];
    let ns_color = get_class("NSColor");
    let current_color: id =
        msg_send![ns_color, colorWithCalibratedRed: r, green: g, blue: b, alpha: a];
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

    let field_hex: id = msg_send![get_class("NSTextField"), alloc];
    let field_hex: id = msg_send![
        field_hex,
        initWithFrame: NSRect::new(NSPoint::new(field_x, h - 185.0), NSSize::new(field_w, 24.0))
    ];
    let _: () = msg_send![field_hex, setStringValue: nsstring_id(&hex_str)];
    configure_hex_field(view, field_hex);

    // Fill transparency
    let field_fill_t = mk_value_label(160.0, h - 234.0, 60.0, 24.0, &format!("{:.0}", fill_t));
    let slider_fill_t: id = msg_send![get_class("NSSlider"), alloc];
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
    let btn_close: id = msg_send![get_class("NSButton"), alloc];
    let btn_close: id = msg_send![
        btn_close,
        initWithFrame: NSRect::new(NSPoint::new(w - 100.0, 15.0), NSSize::new(80.0, 28.0))
    ];
    let _: () = msg_send![btn_close, setTitle: nsstring_id(tr_key("Close", es).as_ref())];
    let _: () = msg_send![btn_close, setTarget: view];
    let _: () = msg_send![btn_close, setAction: sel!(closeSettings:)];

    // Enter/Return activates "Close"
    let _: () = msg_send![btn_close, setKeyEquivalent: nsstring_id("\r")];
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
    (*view).store_ivar::<id>("_settingsWindow", settings);
    (*view).store_ivar::<id>("_labelLang", label_lang);
    (*view).store_ivar::<id>("_popupLang", popup_lang);

    (*view).store_ivar::<id>("_labelRadius", label_radius);
    (*view).store_ivar::<id>("_fieldRadius", field_radius); // label
    (*view).store_ivar::<id>("_sliderRadius", slider_radius);

    (*view).store_ivar::<id>("_labelBorder", label_border);
    (*view).store_ivar::<id>("_fieldBorder", field_border); // label
    (*view).store_ivar::<id>("_sliderBorder", slider_border);

    (*view).store_ivar::<id>("_labelColor", label_color);
    (*view).store_ivar::<id>("_colorWell", color_well);

    (*view).store_ivar::<id>("_labelHex", label_hex);
    (*view).store_ivar::<id>("_fieldHex", field_hex); // editable

    (*view).store_ivar::<id>("_labelFillT", label_fill_t);
    (*view).store_ivar::<id>("_fieldFillT", field_fill_t); // label
    (*view).store_ivar::<id>("_sliderFillT", slider_fill_t);

    (*view).store_ivar::<id>("_btnClose", btn_close);

    // Local monitor for ESC/Enter to close modal
    const KEY_DOWN_MASK: u64 = 1 << 10;
    let key_block = RcBlock::new(move |event: id| -> id {
        unsafe {
            let keycode: u16 = msg_send![event, keyCode];
            if keycode == 53 || keycode == 36 {
                // 53 = Escape, 36 = Return/Enter - stop modal
                let app: id = NSApp();
                let _: () = msg_send![app, stopModal];
                return nil;
            }
        }
        event
    });
    let key_mon: id = msg_send![
        get_class("NSEvent"),
        addLocalMonitorForEventsMatchingMask: KEY_DOWN_MASK,
        handler: &*key_block
    ];

    // Force activation and show window, then run modal
    let app: id = NSApp();
    let _: () = msg_send![app, activateIgnoringOtherApps: YES];
    let _: () = msg_send![settings, makeKeyAndOrderFront: nil];

    let _modal_result: i64 = msg_send![app, runModalForWindow: settings];

    // Modal ended - clean up
    let _: () = msg_send![get_class("NSEvent"), removeMonitor: key_mon];
    let _: () = msg_send![settings, orderOut: nil];

    // Clear stored references
    (*view).store_ivar::<id>("_settingsWindow", nil);
    (*view).store_ivar::<id>("_settingsEscMonitor", nil);
    (*view).store_ivar::<id>("_settingsGlobalMonitor", nil);

    // Restore overlay if it was enabled
    if was_enabled {
        apply_to_all_views(|v| {
            set_bool_ivar(v, "_overlayEnabled", true);
        });
    }

    // Ensure overlays are in front and refreshed
    apply_to_all_views(|v| {
        let overlay_win: id = msg_send![v, window];
        let _: () = msg_send![overlay_win, setLevel: overlay_window_level()];
        let _: () = msg_send![overlay_win, orderFrontRegardless];
        let _: () = msg_send![
            v,
            performSelectorOnMainThread: sel!(update_cursor_multi),
            withObject: nil,
            waitUntilDone: NO
        ];
    });

    // Reset atomic guard
    SETTINGS_OPENING.store(false, Ordering::SeqCst);

    // Publish event - dispatcher will handle hotkey reinstallation
    publish(AppEvent::SettingsClosed);
}

/// Close the settings window by stopping the modal.
///
/// # Safety
/// Must be called from main thread with valid autorelease pool.
pub unsafe fn close_settings_window(_view: id) {
    // Just stop the modal - cleanup happens in open_settings_window after runModalForWindow returns
    let app: id = NSApp();
    let _: () = msg_send![app, stopModal];
}
