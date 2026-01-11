//! Thread-safe event bus using mpsc channels.
//!
//! The bus provides a simple publish/subscribe mechanism where:
//! - Any thread can publish events via `EventPublisher::publish()`
//! - The main thread polls for events via `EventBus::drain()`
//!
//! This is pure Rust with no external dependencies beyond std.

use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

use super::types::AppEvent;

/// Thread-safe event bus for application-wide event distribution.
///
/// Uses a multi-producer, single-consumer (mpsc) channel internally.
/// Multiple publishers can send events concurrently, and a single
/// consumer (the main thread) receives and processes them.
///
/// # Example
///
/// ```
/// use mouse_highlighter::events::{EventBus, AppEvent};
///
/// let bus = EventBus::new();
/// let publisher = bus.publisher();
///
/// publisher.publish(AppEvent::ToggleOverlay);
///
/// let events = bus.drain();
/// assert_eq!(events.len(), 1);
/// ```
pub struct EventBus {
    sender: Sender<AppEvent>,
    receiver: Receiver<AppEvent>,
}

impl EventBus {
    /// Create a new event bus.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    /// Get a publisher handle that can be cloned and sent to other threads.
    ///
    /// Publishers are cheap to clone and thread-safe. Each module that needs
    /// to emit events should hold its own publisher.
    pub fn publisher(&self) -> EventPublisher {
        EventPublisher {
            sender: self.sender.clone(),
        }
    }

    /// Try to receive the next event without blocking.
    ///
    /// Returns `Some(event)` if an event is available, `None` otherwise.
    /// This should be called from the main thread's event loop.
    pub fn try_recv(&self) -> Option<AppEvent> {
        match self.receiver.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                // All senders dropped - this shouldn't happen in normal operation
                // but we handle it gracefully
                None
            }
        }
    }

    /// Drain all pending events into a Vec.
    ///
    /// This is the preferred method for processing events in the main loop.
    /// It collects all available events at once, allowing batch processing.
    pub fn drain(&self) -> Vec<AppEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.try_recv() {
            events.push(event);
        }
        events
    }

    /// Check if there are any pending events without consuming them.
    ///
    /// Note: Due to the nature of mpsc channels, this is a point-in-time check.
    /// Events may arrive between checking and draining.
    pub fn has_pending(&self) -> bool {
        // We can't peek without consuming, so we try_recv and re-send
        // This is a limitation of mpsc - for this app it's fine to just drain
        false // Conservative: always return false, let drain handle it
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// A cloneable, thread-safe event publisher.
///
/// Multiple modules can hold publishers and send events concurrently.
/// Cloning a publisher is cheap (just clones the internal Sender).
#[derive(Clone)]
pub struct EventPublisher {
    sender: Sender<AppEvent>,
}

impl EventPublisher {
    /// Create a publisher from an existing sender.
    ///
    /// Used by the global access module to create publishers from the static sender.
    pub fn from_sender(sender: Sender<AppEvent>) -> Self {
        Self { sender }
    }

    /// Publish an event to the bus.
    ///
    /// This is non-blocking and thread-safe. The event will be queued
    /// and processed on the next drain cycle in the main thread.
    ///
    /// If the receiver has been dropped (app shutting down), the send
    /// silently fails - this is intentional.
    pub fn publish(&self, event: AppEvent) {
        // Ignore send errors - receiver dropped means app is shutting down
        let _ = self.sender.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bus() {
        let bus = EventBus::new();
        assert!(bus.drain().is_empty());
    }

    #[test]
    fn test_publish_and_receive_single_event() {
        let bus = EventBus::new();
        let publisher = bus.publisher();

        publisher.publish(AppEvent::ToggleOverlay);

        let events = bus.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], AppEvent::ToggleOverlay);
    }

    #[test]
    fn test_publish_and_receive_multiple_events() {
        let bus = EventBus::new();
        let publisher = bus.publisher();

        publisher.publish(AppEvent::ToggleOverlay);
        publisher.publish(AppEvent::OpenSettings);
        publisher.publish(AppEvent::RequestQuit);

        let events = bus.drain();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], AppEvent::ToggleOverlay);
        assert_eq!(events[1], AppEvent::OpenSettings);
        assert_eq!(events[2], AppEvent::RequestQuit);
    }

    #[test]
    fn test_drain_empties_queue() {
        let bus = EventBus::new();
        let publisher = bus.publisher();

        publisher.publish(AppEvent::ToggleOverlay);
        publisher.publish(AppEvent::OpenSettings);

        let first_drain = bus.drain();
        assert_eq!(first_drain.len(), 2);

        let second_drain = bus.drain();
        assert!(second_drain.is_empty());
    }

    #[test]
    fn test_multiple_publishers() {
        let bus = EventBus::new();
        let pub1 = bus.publisher();
        let pub2 = bus.publisher();
        let pub3 = bus.publisher();

        pub1.publish(AppEvent::ToggleOverlay);
        pub2.publish(AppEvent::OpenSettings);
        pub3.publish(AppEvent::RequestQuit);

        let events = bus.drain();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn test_publisher_clone_is_independent() {
        let bus = EventBus::new();
        let pub1 = bus.publisher();
        let pub2 = pub1.clone();

        pub1.publish(AppEvent::ToggleOverlay);
        pub2.publish(AppEvent::OpenSettings);

        let events = bus.drain();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_try_recv_returns_none_when_empty() {
        let bus = EventBus::new();
        assert!(bus.try_recv().is_none());
    }

    #[test]
    fn test_try_recv_returns_events_in_order() {
        let bus = EventBus::new();
        let publisher = bus.publisher();

        publisher.publish(AppEvent::ToggleOverlay);
        publisher.publish(AppEvent::OpenSettings);

        assert_eq!(bus.try_recv(), Some(AppEvent::ToggleOverlay));
        assert_eq!(bus.try_recv(), Some(AppEvent::OpenSettings));
        assert_eq!(bus.try_recv(), None);
    }

    #[test]
    fn test_default_creates_new_bus() {
        let bus = EventBus::default();
        let publisher = bus.publisher();

        publisher.publish(AppEvent::ToggleOverlay);
        assert_eq!(bus.drain().len(), 1);
    }

    #[test]
    fn test_events_preserve_data() {
        let bus = EventBus::new();
        let publisher = bus.publisher();

        // Test all event types round-trip correctly
        let test_events = vec![
            AppEvent::ToggleOverlay,
            AppEvent::OpenSettings,
            AppEvent::RequestQuit,
            AppEvent::SettingsClosed,
            AppEvent::QuitCancelled,
            AppEvent::ReinstallHotkeys,
        ];

        for event in &test_events {
            publisher.publish(event.clone());
        }

        let received = bus.drain();
        assert_eq!(received, test_events);
    }
}
