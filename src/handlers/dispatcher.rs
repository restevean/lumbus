//! Event dispatcher for handling application events.
//!
//! The dispatcher receives events from the event bus and executes
//! the corresponding actions. It's called from the main loop timer
//! and processes all pending events in batch.
//!
//! # Architecture
//!
//! ```text
//! EventBus::drain() → dispatch_events() → action handlers
//! ```
//!
//! The dispatcher acts as the central coordinator, translating
//! high-level events into concrete macOS actions.

use cocoa::appkit::NSApp;
use cocoa::base::{id, nil};
use objc::{msg_send, sel, sel_impl};

use lumbus::events::{drain_events, AppEvent};

/// Callback type for reinstalling hotkeys.
///
/// After certain UI operations (settings closed, quit cancelled),
/// Carbon hotkeys need to be reinstalled.
pub type ReinstallHotkeysCallback = unsafe fn(id);

/// Dispatch all pending events from the global event bus.
///
/// This should be called from the main loop timer (60fps).
/// It drains all pending events and executes the appropriate actions.
///
/// # Arguments
///
/// * `view` - The host CustomView for UI operations
/// * `open_settings_fn` - Function to open settings window
/// * `confirm_quit_fn` - Function to show quit confirmation dialog
/// * `reinstall_hotkeys_fn` - Function to reinstall Carbon hotkeys
///
/// # Safety
///
/// Must be called from the main thread. The view pointer must be valid.
pub unsafe fn dispatch_events(
    view: id,
    open_settings_fn: unsafe fn(id),
    confirm_quit_fn: unsafe fn(id),
    reinstall_hotkeys_fn: ReinstallHotkeysCallback,
) {
    let events = drain_events();

    for event in events {
        dispatch_single_event(
            view,
            &event,
            open_settings_fn,
            confirm_quit_fn,
            reinstall_hotkeys_fn,
        );
    }
}

/// Dispatch a single event.
///
/// # Safety
///
/// Must be called from the main thread. The view pointer must be valid.
unsafe fn dispatch_single_event(
    view: id,
    event: &AppEvent,
    open_settings_fn: unsafe fn(id),
    confirm_quit_fn: unsafe fn(id),
    reinstall_hotkeys_fn: ReinstallHotkeysCallback,
) {
    match event {
        AppEvent::ToggleOverlay => {
            // Toggle overlay visibility via the view's method
            let _: () = objc::msg_send![view, requestToggle];
        }

        AppEvent::OpenSettings => {
            // Open settings window - it will publish SettingsClosed when done
            open_settings_fn(view);
        }

        AppEvent::RequestQuit => {
            // Show quit dialog - it will publish QuitCancelled if user cancels
            confirm_quit_fn(view);
        }

        AppEvent::ShowAbout => {
            // Show the standard macOS About panel
            let app: id = NSApp();
            let _: () = msg_send![app, orderFrontStandardAboutPanel: nil];
        }

        AppEvent::SettingsClosed | AppEvent::QuitCancelled | AppEvent::ReinstallHotkeys => {
            // These events require hotkey reinstallation
            reinstall_hotkeys_fn(view);
        }
    }
}

#[cfg(test)]
mod tests {
    use lumbus::events::AppEvent;

    // Note: Full integration testing requires macOS runtime.
    // These tests verify the dispatch logic at a structural level.

    #[test]
    fn test_module_compiles() {
        // Smoke test that the module compiles correctly
    }

    #[test]
    fn test_reinstall_required_events() {
        // Verify that the events that should trigger reinstall are correct
        assert!(AppEvent::SettingsClosed.requires_hotkey_reinstall());
        assert!(AppEvent::QuitCancelled.requires_hotkey_reinstall());
        assert!(AppEvent::ReinstallHotkeys.requires_hotkey_reinstall());
        assert!(!AppEvent::ToggleOverlay.requires_hotkey_reinstall());
        assert!(!AppEvent::OpenSettings.requires_hotkey_reinstall());
        assert!(!AppEvent::RequestQuit.requires_hotkey_reinstall());
        assert!(!AppEvent::ShowAbout.requires_hotkey_reinstall());
    }
}
