//! macOS-specific entry point and application logic.
//!
//! This module contains the main application loop for macOS.
//! The CustomView implementation is in platform/macos/ui/overlay/view.rs.

use lumbus::model::constants::*;
use lumbus::platform::macos::app::sync_visual_prefs_to_all_views;
use lumbus::platform::macos::ffi::bridge::{
    autoreleasepool, get_class, id, msg_send, nil, nsstring_id, NSApp, ObjectExt, NO, YES,
};
use lumbus::platform::macos::ffi::{
    display_id_for_screen, ensure_accessibility_prompt, overlay_window_level,
};
use lumbus::platform::macos::input::{
    hotkey_event_handler, install_hotkeys, install_local_ctrl_a_monitor, install_mouse_monitors,
    install_termination_observer, install_wakeup_space_observers, start_hotkey_keepalive,
};
use lumbus::platform::macos::storage::{prefs_get_double, prefs_get_int};
use lumbus::platform::macos::ui::{install_status_bar, register_and_create_view};

use objc2::sel;
use objc2_foundation::NSRect;

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

/// Load preferences into a view.
///
/// # Safety
/// The view must be a valid CustomView instance.
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
    (*view).store_ivar::<f64>("_fillTransparencyPct", fill_t.clamp(0.0, 100.0));
    (*view).store_ivar::<i32>("_lang", if lang == 1 { 1 } else { 0 });
}

/// Create a transparent overlay window for a given screen.
///
/// # Safety
/// The screen must be a valid NSScreen instance.
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

    let view: id = register_and_create_view(window, frame.size.width, frame.size.height);
    (*view).store_ivar::<id>("_ownScreen", screen);
    // Store stable DisplayID for this view's screen
    let did = display_id_for_screen(screen);
    (*view).store_ivar::<u32>("_ownDisplayID", did);

    (window, view)
}

/// Create an AppKit timer that fires even during modal menus.
///
/// # Safety
/// The target must be a valid NSObject that responds to the selector.
unsafe fn create_timer(target: id, selector: objc2::runtime::Sel, interval: f64) -> id {
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
