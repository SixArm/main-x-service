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
//! - **Persistence** ([`db`]) — PostgreSQL via SeaORM, plus a
//!   HIPAA-style audit log.
//! - **APIs** ([`api`]) — REST (Axum), FHIR stubs, gRPC stub.
//! - **Event streaming** ([`streaming`]) and **observability**
//!   ([`observability`], [`metrics`]).
//!
//! # Example
//!
//! Constructing the core [`Event`](models::Event) value object needs
//! no I/O, so it can run as a doctest:
//!
//! ```
//! use event_service::models::Event;
//!
//! let start = jiff::civil::datetime(2026, 6, 1, 9, 0, 0, 0).in_tz("UTC").unwrap().timestamp();
//! let event = Event::new("Annual Conference", start);
//! assert_eq!(event.name, "Annual Conference");
//! assert!(event.active);
//! ```

// Module declarations.
/// HTTP / RPC surface: REST (Axum), FHIR stubs, gRPC stub.
pub mod api;
/// Runtime configuration (server, database, search, matching).
pub mod config;
/// PostgreSQL persistence (SeaORM entities, repositories, audit log).
pub mod db;
/// Crate-wide error type and [`Result`] alias.
pub mod error;
/// Matching strategies, scoring, and the canonical-matcher bridge.
pub mod matching;
/// Prometheus metric registry and text exposition.
pub mod metrics;
/// Domain models (the [`Event`](models::Event) aggregate and friends).
pub mod models;
/// OpenTelemetry tracing and metrics setup.
pub mod observability;
/// Privacy controls: masking, GDPR export, consent checks.
pub mod privacy;
/// Tantivy full-text search index and query layer.
pub mod search;
/// Event-stream publishing and consumption.
pub mod streaming;
/// Data-quality validation, normalization, and standardization.
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
