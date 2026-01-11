//! Global access to the application event bus.
//!
//! This module provides static access to the event bus.
//! The bus must be initialized once at application startup via `init_event_bus()`,
//! then any module can publish events via `publish()` or `publisher()`.
//!
//! # Design
//!
//! - `Sender` is stored in `OnceLock` - it's `Send + Sync`, perfect for static
//! - `Receiver` is stored in `Mutex` - only accessed from main thread, minimal overhead
//!
//! # Example
//!
//! ```ignore
//! // In main.rs at startup:
//! events::init_event_bus();
//!
//! // In any module:
//! events::publish(AppEvent::ToggleOverlay);
//!
//! // Or get a publisher for repeated use:
//! let pub = events::publisher();
//! pub.publish(AppEvent::OpenSettings);
//! ```

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

use super::bus::EventPublisher;
use super::types::AppEvent;

/// Global sender for publishing events.
///
/// `Sender` is `Send + Sync`, so it can be safely stored in a static.
static SENDER: OnceLock<Sender<AppEvent>> = OnceLock::new();

/// Global receiver for draining events.
///
/// Wrapped in `Mutex` for `Sync` requirement. Only accessed from main thread,
/// so contention is effectively zero.
static RECEIVER: OnceLock<Mutex<Receiver<AppEvent>>> = OnceLock::new();

/// Initialize the global event bus.
///
/// Must be called exactly once at application startup, before any events
/// are published. Panics if called more than once.
///
/// # Panics
///
/// Panics if the event bus has already been initialized.
pub fn init_event_bus() {
    let (sender, receiver) = mpsc::channel();

    SENDER
        .set(sender)
        .expect("Event bus already initialized (sender)");

    RECEIVER
        .set(Mutex::new(receiver))
        .expect("Event bus already initialized (receiver)");
}

/// Get a publisher handle for the global event bus.
///
/// Publishers are cheap to clone and thread-safe. Each module that needs
/// to emit events can call this once and store the publisher.
///
/// # Panics
///
/// Panics if `init_event_bus()` has not been called.
pub fn publisher() -> EventPublisher {
    let sender = SENDER
        .get()
        .expect("Event bus not initialized - call init_event_bus() first");

    EventPublisher::from_sender(sender.clone())
}

/// Publish an event to the global event bus.
///
/// Convenience function for one-off event publishing.
/// For repeated publishing from the same location, prefer storing
/// a publisher via `publisher()`.
///
/// # Panics
///
/// Panics if `init_event_bus()` has not been called.
pub fn publish(event: AppEvent) {
    let sender = SENDER
        .get()
        .expect("Event bus not initialized - call init_event_bus() first");

    // Ignore send errors - receiver dropped means app is shutting down
    let _ = sender.send(event);
}

/// Drain all pending events from the global event bus.
///
/// Convenience function for the main loop to collect all events
/// that have been published since the last drain.
///
/// # Panics
///
/// Panics if `init_event_bus()` has not been called.
/// May also panic if the receiver mutex is poisoned.
pub fn drain_events() -> Vec<AppEvent> {
    let receiver = RECEIVER
        .get()
        .expect("Event bus not initialized - call init_event_bus() first");

    let receiver = receiver.lock().expect("Event bus receiver mutex poisoned");

    let mut events = Vec::new();
    while let Ok(event) = receiver.try_recv() {
        events.push(event);
    }
    events
}

#[cfg(test)]
mod tests {
    // Note: These tests cannot use the global SENDER/RECEIVER directly because
    // OnceLock can only be set once per process. Instead, we test the
    // EventBus functionality directly in bus.rs tests.
    //
    // The global functions are thin wrappers that delegate to mpsc,
    // so testing EventBus provides sufficient coverage.
    //
    // Integration testing of the global access would require either:
    // 1. Running tests in separate processes
    // 2. Using a test-specific initialization mechanism
    //
    // For this application, manual testing of the integrated app
    // validates the global access pattern works correctly.

    #[test]
    fn test_module_compiles() {
        // Smoke test that the module compiles correctly
    }
}
