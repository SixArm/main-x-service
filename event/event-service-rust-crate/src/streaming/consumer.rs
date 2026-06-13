//! [`EventConsumer`](crate::streaming::EventConsumer) implementation.
//!
//! [`FluvioConsumer`](crate::streaming::consumer::FluvioConsumer) is the
//! production placeholder for reading
//! [`EventEvent`](crate::streaming::EventEvent)s back off the stream;
//! both methods are currently unimplemented.

use super::{EventConsumer, EventEvent};
use crate::Result;

/// Production [`EventConsumer`] backed by Fluvio (not yet implemented).
pub struct FluvioConsumer {
    // Fluvio consumer will be initialized here
}

impl EventConsumer for FluvioConsumer {
    /// Begin a Fluvio subscription. Currently unimplemented (`todo!`).
    fn subscribe(&mut self) -> Result<()> {
        // TODO: Implement Fluvio subscription
        todo!("Implement Fluvio subscription")
    }

    /// Pull the next event from Fluvio. Currently unimplemented (`todo!`).
    fn next_event(&mut self) -> Result<Option<EventEvent>> {
        // TODO: Implement event consumption
        todo!("Implement event consumption")
    }
}
