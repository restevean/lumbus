//! Event handlers and dispatching.
//!
//! This module contains the event dispatcher that processes events
//! from the event bus and executes the corresponding actions.

pub mod dispatcher;

pub use dispatcher::dispatch_events;
