//! Consuming side of the event stream (stub).
//!
//! [`FluvioConsumer`] is the intended production reader of the worker event
//! topic; its [`EventConsumer`] methods are not yet implemented.

use super::{EventConsumer, WorkerEvent};
use crate::Result;

/// Production [`EventConsumer`] backed by Fluvio (not yet implemented).
pub struct FluvioConsumer {
    // Fluvio consumer handle will be initialized here once wired up.
}

impl EventConsumer for FluvioConsumer {
    /// Not yet implemented.
    ///
    /// # Errors
    ///
    /// Will surface subscription failures once implemented.
    ///
    /// # Panics
    ///
    /// Always panics via `todo!` — this consumer is a placeholder.
    fn subscribe(&mut self) -> Result<()> {
        // TODO: Implement Fluvio subscription
        todo!("Implement Fluvio subscription")
    }

    /// Not yet implemented.
    ///
    /// # Errors
    ///
    /// Will surface poll failures once implemented.
    ///
    /// # Panics
    ///
    /// Always panics via `todo!` — this consumer is a placeholder.
    fn next_event(&mut self) -> Result<Option<WorkerEvent>> {
        // TODO: Implement event consumption
        todo!("Implement event consumption")
    }
}
