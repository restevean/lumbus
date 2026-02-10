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

use std::sync::atomic::{AtomicBool, Ordering};

use crate::platform::macos::ffi::bridge::{id, msg_send, nil, NSApp, YES};

use crate::platform::macos::ui::show_help_overlay;
use crate::events::{take_event, AppEvent};

/// Guard to prevent concurrent dispatch_events calls from racing.
///
/// This is necessary because macOS run loop continues processing timers
/// while a modal dialog is open via `runModalForWindow:`. Without this guard,
/// pressing Ctrl+, multiple times quickly would open multiple settings windows.
///
/// The guard is acquired at the START of dispatch_events and released
/// when the modal closes, preventing race conditions where two timer
/// callbacks both take events before either reaches the modal.
static DISPATCH_GUARD: AtomicBool = AtomicBool::new(false);

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
    // CRITICAL: Acquire exclusive access to event processing.
    // This prevents race conditions where two timer callbacks both
    // enter dispatch_events and take events before either can block.
    if DISPATCH_GUARD
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        // Another dispatch_events call is already running - skip this tick
        return;
    }

    // Process all pending events
    while let Some(event) = take_event() {
        let was_modal = dispatch_single_event(
            view,
            &event,
            open_settings_fn,
            confirm_quit_fn,
            reinstall_hotkeys_fn,
        );

        // After a modal closes, drain any duplicate modal events that accumulated
        // while the user was spamming the hotkey. This prevents the second window
        // from opening after the first one closes.
        if was_modal {
            drain_duplicate_modal_events();
            break;
        }
    }

    // Release the guard so next timer tick can process events
    DISPATCH_GUARD.store(false, Ordering::SeqCst);
}

/// Drain and discard any pending modal events from the queue.
///
/// Called after a modal closes to prevent duplicate windows from opening.
fn drain_duplicate_modal_events() {
    while let Some(event) = take_event() {
        match event {
            // Discard duplicate modal-opening events
            AppEvent::OpenSettings
            | AppEvent::RequestQuit
            | AppEvent::ShowHelp
            | AppEvent::ShowAbout => {
                // Silently discard
            }
            // Re-queue non-modal events? No, we can't easily re-queue.
            // These events (SettingsClosed, etc.) are posted AFTER the modal,
            // so they shouldn't be in the queue at this point anyway.
            _ => {
                // This shouldn't happen, but log if it does
                #[cfg(debug_assertions)]
                eprintln!("[DISPATCH] Unexpected event after modal: {:?}", event);
            }
        }
    }
}

/// Dispatch a single event.
///
/// Returns `true` if the event was a modal (blocking) event.
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
) -> bool {
    match event {
        AppEvent::ToggleOverlay => {
            // Toggle overlay visibility via the view's method
            let _: () = msg_send![view, requestToggle];
            false
        }

        AppEvent::OpenSettings => {
            // Open settings window - blocks until closed
            open_settings_fn(view);
            true
        }

        AppEvent::RequestQuit => {
            // Show quit dialog - blocks until closed
            confirm_quit_fn(view);
            true
        }

        AppEvent::ShowAbout => {
            // Activate app and show the standard macOS About panel
            let app: id = NSApp();
            let _: () = msg_send![app, activateIgnoringOtherApps: YES];
            let _: () = msg_send![app, orderFrontStandardAboutPanel: nil];
            // About panel is non-blocking
            false
        }

        AppEvent::ShowHelp => {
            // Show help overlay - blocks until closed
            show_help_overlay(view);
            true
        }

        AppEvent::SettingsClosed
        | AppEvent::QuitCancelled
        | AppEvent::HelpClosed
        | AppEvent::ReinstallHotkeys => {
            // These events require hotkey reinstallation
            reinstall_hotkeys_fn(view);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::events::AppEvent;

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
        assert!(AppEvent::HelpClosed.requires_hotkey_reinstall());
        assert!(AppEvent::ReinstallHotkeys.requires_hotkey_reinstall());
        assert!(!AppEvent::ToggleOverlay.requires_hotkey_reinstall());
        assert!(!AppEvent::OpenSettings.requires_hotkey_reinstall());
        assert!(!AppEvent::RequestQuit.requires_hotkey_reinstall());
        assert!(!AppEvent::ShowAbout.requires_hotkey_reinstall());
        assert!(!AppEvent::ShowHelp.requires_hotkey_reinstall());
    }
}
