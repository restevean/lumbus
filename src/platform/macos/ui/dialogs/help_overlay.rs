//! Help overlay showing keyboard shortcuts.
//!
//! Displays a semi-transparent overlay with all available hotkeys.
//! Dismisses on any key press.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::platform::macos::ffi::bridge::{
    get_bool_ivar, get_class, id, msg_send, nil, nsstring_id, sel, set_bool_ivar, NSApp, NSPoint,
    NSRect, NSSize, NO, YES,
};
use block2::RcBlock;

use crate::events::{publish, AppEvent};
use crate::platform::macos::app::{apply_to_all_views, lang_is_es};
use crate::platform::macos::ffi::overlay_window_level;
use crate::tr_key;

/// Guard to prevent multiple help overlays
static HELP_OPENING: AtomicBool = AtomicBool::new(false);

/// Data structure for a single hotkey entry to display.
struct HotkeyEntry {
    /// The key combination to display (e.g., "Ctrl + A")
    keys: &'static str,
    /// Translation key for the description
    description_key: &'static str,
}

/// All hotkeys to display in the help overlay.
const HOTKEYS: &[HotkeyEntry] = &[
    HotkeyEntry {
        keys: "Ctrl + A",
        description_key: "Toggle overlay",
    },
    HotkeyEntry {
        keys: "Ctrl + ,",
        description_key: "Open settings",
    },
    HotkeyEntry {
        keys: "\u{2318} + Shift + H",
        description_key: "Show help",
    },
    HotkeyEntry {
        keys: "Ctrl + Shift + X",
        description_key: "Quit app",
    },
];

/// Show the help overlay with keyboard shortcuts.
///
/// The overlay appears centered on the screen where the cursor is.
/// It dismisses when the user presses any key.
///
/// Publishes `AppEvent::HelpClosed` when dismissed.
///
/// # Safety
/// - `view` must be a valid, non-null pointer to a CustomViewMulti.
/// - Must be called from main thread with valid autorelease pool.
pub unsafe fn show_help_overlay(view: id) {
    // Atomic guard: only one help overlay can be opening at a time
    if HELP_OPENING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    // Save current overlay state and hide circle during dialog
    let was_enabled = get_bool_ivar(view, "_overlayEnabled");
    apply_to_all_views(|v| {
        set_bool_ivar(v, "_overlayEnabled", false);
        set_bool_ivar(v, "_visible", false);
        let _: () = msg_send![v, setNeedsDisplay: YES];
    });

    let es = lang_is_es(view);

    // Dialog dimensions
    let dialog_w: f64 = 380.0;
    let dialog_h: f64 = 260.0;

    // Create BORDERLESS window (key to appearing over fullscreen apps)
    // NSBorderlessWindowMask = 0
    let dialog: id = msg_send![get_class("NSWindow"), alloc];
    let dialog: id = msg_send![
        dialog,
        initWithContentRect: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(dialog_w, dialog_h)),
        styleMask: 0u64,
        backing: 2u64,  // NSBackingStoreBuffered
        defer: NO
    ];

    // Configure window like overlay windows
    let clear_color: id = msg_send![get_class("NSColor"), clearColor];
    let _: () = msg_send![dialog, setOpaque: NO];
    let _: () = msg_send![dialog, setBackgroundColor: clear_color];
    let _: () = msg_send![dialog, setLevel: overlay_window_level()];
    // CanJoinAllSpaces (1) + Stationary (16) + FullScreenAuxiliary (256) = 273
    let _: () = msg_send![dialog, setCollectionBehavior: 273u64];
    // Allow the window to receive mouse events
    let _: () = msg_send![dialog, setIgnoresMouseEvents: NO];

    // Find screen where cursor is
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

    // Center dialog on target screen
    let screen_frame: NSRect = msg_send![target_screen, frame];
    let centered_x = screen_frame.origin.x + (screen_frame.size.width - dialog_w) / 2.0;
    let centered_y = screen_frame.origin.y + (screen_frame.size.height - dialog_h) / 2.0;
    let _: () = msg_send![dialog, setFrameOrigin: NSPoint { x: centered_x, y: centered_y }];

    // Create content view with rounded background
    let content: id = msg_send![dialog, contentView];

    // Background box (dark semi-transparent with rounded corners)
    let bg_box: id = msg_send![get_class("NSBox"), alloc];
    let bg_box: id = msg_send![bg_box, initWithFrame: NSRect::new(
        NSPoint::new(0.0, 0.0),
        NSSize::new(dialog_w, dialog_h)
    )];
    let _: () = msg_send![bg_box, setBoxType: 4i64]; // NSBoxCustom
    let _: () = msg_send![bg_box, setBorderType: 0i64]; // NSNoBorder
    let _: () = msg_send![bg_box, setCornerRadius: 12.0f64];
    let bg_color: id = msg_send![
        get_class("NSColor"),
        colorWithSRGBRed: 0.15f64,
        green: 0.15f64,
        blue: 0.15f64,
        alpha: 0.95f64
    ];
    let _: () = msg_send![bg_box, setFillColor: bg_color];
    let _: () = msg_send![content, addSubview: bg_box];

    // Title label
    let title_text = tr_key("Keyboard Shortcuts", es);
    let title_label: id = msg_send![get_class("NSTextField"), alloc];
    let title_label: id = msg_send![title_label, initWithFrame: NSRect::new(
        NSPoint::new(20.0, dialog_h - 50.0),
        NSSize::new(dialog_w - 40.0, 30.0)
    )];
    let _: () = msg_send![title_label, setStringValue: nsstring_id(&title_text)];
    let _: () = msg_send![title_label, setBezeled: NO];
    let _: () = msg_send![title_label, setDrawsBackground: NO];
    let _: () = msg_send![title_label, setEditable: NO];
    let _: () = msg_send![title_label, setSelectable: NO];
    let white_color: id = msg_send![get_class("NSColor"), whiteColor];
    let _: () = msg_send![title_label, setTextColor: white_color];
    let _: () = msg_send![title_label, setAlignment: 1i64]; // NSTextAlignmentCenter
    let bold_font: id = msg_send![get_class("NSFont"), boldSystemFontOfSize: 18.0f64];
    let _: () = msg_send![title_label, setFont: bold_font];
    let _: () = msg_send![content, addSubview: title_label];

    // Hotkey entries
    let row_height: f64 = 32.0;
    let start_y = dialog_h - 90.0;
    let key_x: f64 = 30.0;
    let key_w: f64 = 140.0;
    let desc_x: f64 = 180.0;
    let desc_w: f64 = 170.0;

    let regular_font: id = msg_send![get_class("NSFont"), systemFontOfSize: 14.0f64];
    let light_gray: id = msg_send![
        get_class("NSColor"),
        colorWithSRGBRed: 0.75f64,
        green: 0.75f64,
        blue: 0.75f64,
        alpha: 1.0f64
    ];

    for (i, entry) in HOTKEYS.iter().enumerate() {
        let y = start_y - (i as f64 * row_height);

        // Key combination label (left)
        let key_label: id = msg_send![get_class("NSTextField"), alloc];
        let key_label: id = msg_send![key_label, initWithFrame: NSRect::new(
            NSPoint::new(key_x, y),
            NSSize::new(key_w, 24.0)
        )];
        let _: () = msg_send![key_label, setStringValue: nsstring_id(entry.keys)];
        let _: () = msg_send![key_label, setBezeled: NO];
        let _: () = msg_send![key_label, setDrawsBackground: NO];
        let _: () = msg_send![key_label, setEditable: NO];
        let _: () = msg_send![key_label, setSelectable: NO];
        let _: () = msg_send![key_label, setTextColor: white_color];
        let _: () = msg_send![key_label, setFont: regular_font];
        let _: () = msg_send![key_label, setAlignment: 2i64]; // NSTextAlignmentRight
        let _: () = msg_send![content, addSubview: key_label];

        // Description label (right)
        let desc_text = tr_key(entry.description_key, es);
        let desc_label: id = msg_send![get_class("NSTextField"), alloc];
        let desc_label: id = msg_send![desc_label, initWithFrame: NSRect::new(
            NSPoint::new(desc_x, y),
            NSSize::new(desc_w, 24.0)
        )];
        let _: () = msg_send![desc_label, setStringValue: nsstring_id(&desc_text)];
        let _: () = msg_send![desc_label, setBezeled: NO];
        let _: () = msg_send![desc_label, setDrawsBackground: NO];
        let _: () = msg_send![desc_label, setEditable: NO];
        let _: () = msg_send![desc_label, setSelectable: NO];
        let _: () = msg_send![desc_label, setTextColor: light_gray];
        let _: () = msg_send![desc_label, setFont: regular_font];
        let _: () = msg_send![desc_label, setAlignment: 0i64]; // NSTextAlignmentLeft
        let _: () = msg_send![content, addSubview: desc_label];
    }

    // Footer label
    let footer_text = tr_key("Press any key to close", es);
    let footer_label: id = msg_send![get_class("NSTextField"), alloc];
    let footer_label: id = msg_send![footer_label, initWithFrame: NSRect::new(
        NSPoint::new(20.0, 20.0),
        NSSize::new(dialog_w - 40.0, 20.0)
    )];
    let _: () = msg_send![footer_label, setStringValue: nsstring_id(&footer_text)];
    let _: () = msg_send![footer_label, setBezeled: NO];
    let _: () = msg_send![footer_label, setDrawsBackground: NO];
    let _: () = msg_send![footer_label, setEditable: NO];
    let _: () = msg_send![footer_label, setSelectable: NO];
    let footer_color: id = msg_send![
        get_class("NSColor"),
        colorWithSRGBRed: 0.5f64,
        green: 0.5f64,
        blue: 0.5f64,
        alpha: 1.0f64
    ];
    let _: () = msg_send![footer_label, setTextColor: footer_color];
    let _: () = msg_send![footer_label, setAlignment: 1i64]; // NSTextAlignmentCenter
    let font_mgr: id = msg_send![get_class("NSFontManager"), sharedFontManager];
    let italic_font: id = msg_send![font_mgr, convertFont: regular_font, toHaveTrait: 1u64]; // NSItalicFontMask
    let _: () = msg_send![footer_label, setFont: italic_font];
    let _: () = msg_send![content, addSubview: footer_label];

    // Response flag: false=waiting, true=close
    let should_close = Box::into_raw(Box::new(false));

    // Local keyboard monitor - close on any key
    const KEY_DOWN_MASK: u64 = 1 << 10;
    let flag_ptr = should_close;
    let key_block = RcBlock::new(move |_event: id| -> id {
        unsafe {
            *flag_ptr = true;
            let app: id = NSApp();
            let _: () = msg_send![app, stopModal];
        }
        nil
    });
    let key_mon: id = msg_send![
        get_class("NSEvent"),
        addLocalMonitorForEventsMatchingMask: KEY_DOWN_MASK,
        handler: &*key_block
    ];

    // Also close on mouse click anywhere
    const LEFT_DOWN_MASK: u64 = 1 << 1;
    let flag_ptr2 = should_close;
    let click_block = RcBlock::new(move |_event: id| -> id {
        unsafe {
            *flag_ptr2 = true;
            let app: id = NSApp();
            let _: () = msg_send![app, stopModal];
        }
        nil
    });
    let click_mon: id = msg_send![
        get_class("NSEvent"),
        addLocalMonitorForEventsMatchingMask: LEFT_DOWN_MASK,
        handler: &*click_block
    ];

    // Show dialog and run modal
    let app: id = NSApp();
    let _: () = msg_send![app, activateIgnoringOtherApps: YES];
    let _: () = msg_send![dialog, makeKeyAndOrderFront: nil];
    let _: i64 = msg_send![app, runModalForWindow: dialog];

    // Clean up monitors
    let _: () = msg_send![get_class("NSEvent"), removeMonitor: key_mon];
    let _: () = msg_send![get_class("NSEvent"), removeMonitor: click_mon];

    // Clean up flag
    let _ = Box::from_raw(should_close);

    // Close dialog
    let _: () = msg_send![dialog, orderOut: nil];

    // Restore overlay if it was enabled
    if was_enabled {
        apply_to_all_views(|v| {
            set_bool_ivar(v, "_overlayEnabled", true);
        });
        let _: () = msg_send![
            view,
            performSelectorOnMainThread: sel!(update_cursor_multi),
            withObject: nil,
            waitUntilDone: NO
        ];
    }

    // Ensure overlays are back on top
    apply_to_all_views(|v| {
        let overlay_win: id = msg_send![v, window];
        let _: () = msg_send![overlay_win, setLevel: overlay_window_level()];
        let _: () = msg_send![overlay_win, orderFrontRegardless];
    });

    // Reset atomic guard
    HELP_OPENING.store(false, Ordering::SeqCst);

    // Publish event - dispatcher will handle hotkey reinstallation
    publish(AppEvent::HelpClosed);
}
