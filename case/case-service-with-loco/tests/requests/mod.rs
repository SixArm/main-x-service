//! Request-level test suites, grouped by controller: the `/api/cases`
//! suite in [`cases`] and the durable event-bus Phase-2 outbox atomicity
//! suite in [`event_outbox`].

mod cases;
mod entity_links;
mod event_outbox;
