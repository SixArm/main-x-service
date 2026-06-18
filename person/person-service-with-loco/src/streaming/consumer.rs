//! Event consumer implementation (stub).
//!
//! Defines [`FluvioConsumer`](crate::streaming::consumer::FluvioConsumer), the production [`EventConsumer`](crate::streaming::EventConsumer) backed by
//! Fluvio. It is not yet implemented; the in-memory test path inspects
//! the publisher's buffer directly rather than going through a consumer.

use super::{EventConsumer, PersonEvent};
use crate::Result;

/// Production [`EventConsumer`] backed by Fluvio (not yet implemented).
///
/// The Fluvio consumer/stream handle will live in this struct once wired
/// up.
pub struct FluvioConsumer {
    // Fluvio consumer will be initialized here
}

impl EventConsumer for FluvioConsumer {
    /// Unimplemented; panics via `todo!` until Fluvio is wired up.
    fn subscribe(&mut self) -> Result<()> {
        // TODO: Implement Fluvio subscription
        todo!("Implement Fluvio subscription")
    }

    /// Unimplemented; panics via `todo!` until Fluvio is wired up.
    fn next_event(&mut self) -> Result<Option<PersonEvent>> {
        // TODO: Implement event consumption
        todo!("Implement event consumption")
    }
}
