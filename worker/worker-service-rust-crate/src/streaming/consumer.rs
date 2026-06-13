//! Consuming side of the event stream (stub).
//!
//! [`FluvioConsumer`] is the intended production reader of the worker event
//! topic; its [`EventConsumer`] methods are not yet implemented.

use super::{EventConsumer, WorkerEvent};
use crate::Result;

/// Production [`EventConsumer`] backed by Fluvio (not yet implemented).
pub struct FluvioConsumer {
    // Fluvio consumer will be initialized here
}

impl EventConsumer for FluvioConsumer {
    /// Not yet implemented — panics via `todo!`.
    fn subscribe(&mut self) -> Result<()> {
        // TODO: Implement Fluvio subscription
        todo!("Implement Fluvio subscription")
    }

    /// Not yet implemented — panics via `todo!`.
    fn next_event(&mut self) -> Result<Option<WorkerEvent>> {
        // TODO: Implement event consumption
        todo!("Implement event consumption")
    }
}
