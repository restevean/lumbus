//! Event system for decoupled inter-module communication.
//!
//! This module provides a simple publish/subscribe mechanism using Rust's
//! standard library `mpsc` channels. It enables:
//!
//! - **Decoupled architecture**: Modules publish events without knowing who handles them
//! - **Thread safety**: Multiple publishers can send events concurrently
//! - **Testability**: Event types are pure Rust enums, easily testable without FFI
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   Hotkeys   │     │  Settings   │     │   Dialogs   │
//! │  (Carbon)   │     │   Window    │     │             │
//! └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
//!        │                   │                   │
//!        │ publish()         │ publish()         │ publish()
//!        ▼                   ▼                   ▼
//! ┌─────────────────────────────────────────────────────┐
//! │                     EventBus                        │
//! │                   (mpsc channel)                    │
//! └─────────────────────────┬───────────────────────────┘
//!                           │ drain()
//!                           ▼
//! ┌─────────────────────────────────────────────────────┐
//! │                    Dispatcher                       │
//! │              (main loop, 60fps timer)               │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use lumbus::events::{self, AppEvent};
//!
//! // Initialize at app startup (once only)
//! events::init_event_bus();
//!
//! // Publish from any module
//! events::publish(AppEvent::ToggleOverlay);
//!
//! // Or get a reusable publisher
//! let publisher = events::publisher();
//! publisher.publish(AppEvent::OpenSettings);
//!
//! // Drain events in main loop
//! for event in events::drain_events() {
//!     // Handle event...
//! }
//! ```
//!
//! # Module Structure
//!
//! - [`types`]: Event definitions (`AppEvent` enum)
//! - [`bus`]: `EventBus` and `EventPublisher` types
//! - [`global`]: Static access functions

pub mod bus;
pub mod global;
pub mod types;

// Re-export main types for convenient access
pub use bus::{EventBus, EventPublisher};
pub use global::{drain_events, init_event_bus, publish, publisher};
pub use types::AppEvent;
