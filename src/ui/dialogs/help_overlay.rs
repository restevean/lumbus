//! Help overlay showing keyboard shortcuts.
//!
//! Displays a semi-transparent overlay with all available hotkeys.
//! Dismisses on any key press.

use block::ConcreteBlock;
use cocoa::appkit::{NSApp, NSBackingStoreType, NSColor, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSPoint, NSRect, NSSize};
use objc::{class, msg_send, sel, sel_impl};

use crate::app::{apply_to_all_views, lang_is_es};
use crate::ffi::{nsstring, overlay_window_level};
use lumbus::events::{publish, AppEvent};
use lumbus::tr_key;

/// Data structure for a single hotkey entry to display.
struct HotkeyEntry {
    /// The key combination to display (e.g., "Ctrl + A")
    keys: &'static str,
    /// Translation key for the description
    description_key: &'static str,
}

/// All hotkeys to display in the help overlay.
const HOTKEYS: &[HotkeyEntry] = &[
    HotkeyEntry { keys: "Ctrl + A", description_key: "Toggle overlay" },
    HotkeyEntry { keys: "\u{2318} + ,", description_key: "Open settings" },
    HotkeyEntry { keys: "\u{2318} + Shift + H", description_key: "Show help" },
    HotkeyEntry { keys: "Ctrl + Shift + X", description_key: "Quit app" },
];

/// Show the help overlay with keyboard shortcuts.
///
/// The overlay appears centered on the screen where the cursor is.
/// It dismisses when the user presses any key.
///
/// Publishes `AppEvent::HelpClosed` when dismissed.
///
/// # Arguments
/// * `view` - The host view (for accessing ivars and triggering updates)
///
/// # Safety
/// Must be called from main thread.
pub fn show_help_overlay(view: id) {
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
        let dialog_w: f64 = 380.0;
        let dialog_h: f64 = 260.0;

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
        // Allow the window to receive mouse events
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
        let title_text = tr_key("Keyboard Shortcuts", es);
        let title_label: id = msg_send![class!(NSTextField), alloc];
        let title_label: id = msg_send![title_label, initWithFrame: NSRect::new(
            NSPoint::new(20.0, dialog_h - 50.0),
            NSSize::new(dialog_w - 40.0, 30.0)
        )];
        let _: () = msg_send![title_label, setStringValue: nsstring(&title_text)];
        let _: () = msg_send![title_label, setBezeled: NO];
        let _: () = msg_send![title_label, setDrawsBackground: NO];
        let _: () = msg_send![title_label, setEditable: NO];
        let _: () = msg_send![title_label, setSelectable: NO];
        let white_color: id = msg_send![class!(NSColor), whiteColor];
        let _: () = msg_send![title_label, setTextColor: white_color];
        let _: () = msg_send![title_label, setAlignment: 1i64]; // NSTextAlignmentCenter
        let bold_font: id = msg_send![class!(NSFont), boldSystemFontOfSize: 18.0f64];
        let _: () = msg_send![title_label, setFont: bold_font];
        let _: () = msg_send![content, addSubview: title_label];

        // Hotkey entries
        let row_height: f64 = 32.0;
        let start_y = dialog_h - 90.0;
        let key_x: f64 = 30.0;
        let key_w: f64 = 140.0;
        let desc_x: f64 = 180.0;
        let desc_w: f64 = 170.0;

        let regular_font: id = msg_send![class!(NSFont), systemFontOfSize: 14.0f64];
        let light_gray: id = NSColor::colorWithSRGBRed_green_blue_alpha_(nil, 0.75, 0.75, 0.75, 1.0);

        for (i, entry) in HOTKEYS.iter().enumerate() {
            let y = start_y - (i as f64 * row_height);

            // Key combination label (left)
            let key_label: id = msg_send![class!(NSTextField), alloc];
            let key_label: id = msg_send![key_label, initWithFrame: NSRect::new(
                NSPoint::new(key_x, y),
                NSSize::new(key_w, 24.0)
            )];
            let _: () = msg_send![key_label, setStringValue: nsstring(entry.keys)];
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
            let desc_label: id = msg_send![class!(NSTextField), alloc];
            let desc_label: id = msg_send![desc_label, initWithFrame: NSRect::new(
                NSPoint::new(desc_x, y),
                NSSize::new(desc_w, 24.0)
            )];
            let _: () = msg_send![desc_label, setStringValue: nsstring(&desc_text)];
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
        let footer_label: id = msg_send![class!(NSTextField), alloc];
        let footer_label: id = msg_send![footer_label, initWithFrame: NSRect::new(
            NSPoint::new(20.0, 20.0),
            NSSize::new(dialog_w - 40.0, 20.0)
        )];
        let _: () = msg_send![footer_label, setStringValue: nsstring(&footer_text)];
        let _: () = msg_send![footer_label, setBezeled: NO];
        let _: () = msg_send![footer_label, setDrawsBackground: NO];
        let _: () = msg_send![footer_label, setEditable: NO];
        let _: () = msg_send![footer_label, setSelectable: NO];
        let _: () = msg_send![footer_label, setTextColor: NSColor::colorWithSRGBRed_green_blue_alpha_(
            nil, 0.5, 0.5, 0.5, 1.0
        )];
        let _: () = msg_send![footer_label, setAlignment: 1i64]; // NSTextAlignmentCenter
        let italic_font: id = msg_send![class!(NSFontManager), sharedFontManager];
        let italic_font: id = msg_send![italic_font, convertFont: regular_font toHaveTrait: 1u64]; // NSItalicFontMask
        let _: () = msg_send![footer_label, setFont: italic_font];
        let _: () = msg_send![content, addSubview: footer_label];

        // Response flag: false=waiting, true=close
        let should_close = Box::into_raw(Box::new(false));

        // Local keyboard monitor - close on any key
        const KEY_DOWN_MASK: u64 = 1 << 10;
        let flag_ptr = should_close;
        let key_block = ConcreteBlock::new(move |_event: id| -> id {
            *flag_ptr = true;
            let app = NSApp();
            let _: () = msg_send![app, stopModal];
            nil
        }).copy();
        let key_mon: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: KEY_DOWN_MASK
            handler: &*key_block
        ];

        // Also close on mouse click anywhere
        const LEFT_DOWN_MASK: u64 = 1 << 1;
        let flag_ptr2 = should_close;
        let click_block = ConcreteBlock::new(move |_event: id| -> id {
            *flag_ptr2 = true;
            let app = NSApp();
            let _: () = msg_send![app, stopModal];
            nil
        }).copy();
        let click_mon: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: LEFT_DOWN_MASK
            handler: &*click_block
        ];

        // Show dialog and run modal
        let app = NSApp();
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];
        let _: () = msg_send![dialog, makeKeyAndOrderFront: nil];
        let _: () = msg_send![app, runModalForWindow: dialog];

        // Clean up monitors
        let _: () = msg_send![class!(NSEvent), removeMonitor: key_mon];
        let _: () = msg_send![class!(NSEvent), removeMonitor: click_mon];

        // Clean up flag
        let _ = Box::from_raw(should_close);

        // Close dialog
        let _: () = msg_send![dialog, orderOut: nil];

        // Restore overlay if it was enabled
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

        // Ensure overlays are back on top
        apply_to_all_views(|v| {
            let overlay_win: id = msg_send![v, window];
            let _: () = msg_send![overlay_win, setLevel: overlay_window_level()];
            let _: () = msg_send![overlay_win, orderFrontRegardless];
        });

        // Publish event - dispatcher will handle hotkey reinstallation
        publish(AppEvent::HelpClosed);
    }
}
