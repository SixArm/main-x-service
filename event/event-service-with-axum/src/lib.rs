//! Event Service — a registry of time-bounded events.
//!
//! This crate is the **Event** member of the Main X Index family. It
//! maintains a federated identity index of time-bounded occurrences —
//! conferences, performances, appointments, encounters, shifts,
//! sessions, screenings, sales, deliveries, incidents — with a domain
//! model aligned with [schema.org/Event](https://schema.org/Event).
//!
//! The crate's external name (for `use` statements in doctests and
//! downstream crates) is `event_service`; the package name in
//! `Cargo.toml` is `event-service`.
//!
//! # What this library provides
//!
//! - **Domain models** ([`models`]) — the [`Event`](models::Event)
//!   aggregate plus its supporting value objects (locations, parties,
//!   offers, identifiers, consent, merge / review-queue records).
//! - **Matching** ([`matching`]) — probabilistic (weighted fuzzy) and
//!   deterministic (rule-based) strategies, with a bridge
//!   ([`matching::adapter`]) to the canonical `event-matcher` crate.
//! - **Full-text search** ([`search`]) via Tantivy.
//! - **Validation** ([`validation`]) — data-quality rules,
//!   normalization, and standardization at the API boundary.
//! - **Privacy** ([`privacy`]) — field masking, GDPR export, consent.
//! - **Persistence** ([`db`]) — `PostgreSQL` via sqlx, plus a
//!   HIPAA-style audit log.
//! - **APIs** ([`api`]) — REST (Axum), FHIR stubs, gRPC stub.
//! - **Event streaming** ([`streaming`]) and **observability**
//!   ([`metrics`]; OTLP export comes from `axa::opentelemetry`).
//!
//! # Example
//!
//! Constructing the core [`Event`](models::Event) value object needs
//! no I/O, so it can run as a doctest:
//!
//! ```
//! use event_service::models::Event;
//!
//! use chrono::{TimeZone, Utc};
//! let start = Utc.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();
//! let event = Event::new("Annual Conference", start);
//! assert_eq!(event.name, "Annual Conference");
//! assert!(event.active);
//! ```

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// Module declarations.
pub mod api;
pub mod app;
pub mod client_limits;
pub mod compliance;
pub mod config;
pub mod controllers;
pub mod db;
pub mod error;
pub mod fhir;
pub mod matching;
pub mod metrics;
pub mod models;
pub mod privacy;
pub mod relay;
pub mod search;
pub mod streaming;
pub mod validation;

// Re-exports: surface the crate-wide error types at the root so callers
// can write `event_service::Error` / `event_service::Result`.
pub use error::{Error, Result};

#[cfg(test)]
mod tests {
    /// Smoke test: the crate-root [`Result`] alias is reachable and
    /// usable, proving the public module tree wires up.
    #[test]
    fn test_module_imports() {
        // Verify key types are accessible.
        let _: fn() -> crate::Result<()> = || Ok(());
    }
}
