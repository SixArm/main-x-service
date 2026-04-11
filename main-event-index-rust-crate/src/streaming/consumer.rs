//! Event consumer implementation

use super::{EventConsumer, EventEvent};
use crate::Result;

pub struct FluvioConsumer {
    // Fluvio consumer will be initialized here
}

impl EventConsumer for FluvioConsumer {
    fn subscribe(&mut self) -> Result<()> {
        // TODO: Implement Fluvio subscription
        todo!("Implement Fluvio subscription")
    }

    fn next_event(&mut self) -> Result<Option<EventEvent>> {
        // TODO: Implement event consumption
        todo!("Implement event consumption")
    }
}
