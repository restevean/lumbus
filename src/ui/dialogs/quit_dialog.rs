//! Quit confirmation dialog.
//!
//! Custom borderless dialog that can appear over fullscreen apps.

use block::ConcreteBlock;
use cocoa::appkit::{NSApp, NSBackingStoreType, NSColor, NSWindow, NSWindowStyleMask};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSPoint, NSRect, NSSize};
use objc::{class, msg_send, sel, sel_impl};

use crate::app::{apply_to_all_views, lang_is_es};
use crate::ffi::{nsstring, overlay_window_level};
use lumbus::events::{publish, AppEvent};
use lumbus::tr_key;

/// Show a quit confirmation dialog.
///
/// If user cancels, publishes `AppEvent::QuitCancelled` to the event bus.
/// The dispatcher handles hotkey reinstallation.
/// If user confirms quit, terminates the application.
///
/// # Arguments
/// * `view` - The host view (for accessing ivars and triggering updates)
///
/// # Safety
/// Must be called from main thread.
pub fn confirm_and_maybe_quit(view: id) {
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
        let app = NSApp();
        let _: () = msg_send![app, activateIgnoringOtherApps: YES];
        let _: () = msg_send![dialog, makeKeyAndOrderFront: nil];
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

        // Publish event - dispatcher will handle hotkey reinstallation
        publish(AppEvent::QuitCancelled);
    }
}
