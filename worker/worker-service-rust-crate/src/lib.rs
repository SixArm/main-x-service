//! Worker Service — a federated worker-identity index.
//!
//! This crate is one member of the **Main X Index** family of identity
//! registries (person, worker, place, thing, event, course). It maintains a
//! centralized registry of *worker* identities (doctors, nurses, carers,
//! staff, employees, …) gathered from many source systems, and exposes the
//! machinery needed to keep that registry clean: matching, deduplication,
//! merging, search, validation, privacy controls, and a full audit trail.
//!
//! # What this library provides
//!
//! - **Domain models** ([`models`]) — the [`Worker`](models::Worker) record
//!   and its sub-objects (names, identifiers, identity documents, emergency
//!   contacts, organizations, consent, merge/review-queue records).
//! - **Matching** ([`matching`]) — probabilistic (weighted-fuzzy) and
//!   deterministic (rule-based) scorers, plus an [`adapter`](matching::adapter)
//!   that bridges service records into the canonical `worker-matcher` crate.
//! - **Search** ([`search`]) — full-text, fuzzy, and phonetic search backed by
//!   the embedded [Tantivy](https://github.com/quickwit-oss/tantivy) index.
//! - **Persistence** ([`db`]) — SeaORM entities, repositories, and an audit
//!   log repository for PostgreSQL.
//! - **API surface** ([`api`]) — REST (Axum), FHIR R5, and a gRPC stub.
//! - **Cross-cutting concerns** — [`validation`], [`privacy`],
//!   [`streaming`] (event publishing), [`observability`] (OpenTelemetry),
//!   [`metrics`] (Prometheus), [`config`], and the crate-wide [`error`] types.
//!
//! # Crate layering
//!
//! The modules form a rough stack: the API layer ([`api`]) sits on top of the
//! business logic ([`matching`], [`search`], [`validation`], [`privacy`]),
//! which operates on the [`models`] and is persisted through [`db`]. Every
//! mutating operation also flows through [`streaming`] (events) and the audit
//! log. See `AGENTS/architecture.md` for the full diagram.
//!
//! # Example
//!
//! Construct a worker and read its derived full name:
//!
//! ```
//! use worker_service::models::{Worker, HumanName, Gender};
//!
//! let name = HumanName {
//!     use_type: None,
//!     family: "Smith".into(),
//!     given: vec!["John".into()],
//!     prefix: vec![],
//!     suffix: vec![],
//! };
//! let worker = Worker::new(name, Gender::Male);
//! assert_eq!(worker.full_name(), "John Smith");
//! assert!(worker.active);
//! ```

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// Module declarations.
//
// Every top-level module is `pub` so the crate doubles as a reusable library:
// downstream callers can pull in just the matching engine, just the models,
// etc., without standing up the full service.

/// HTTP / RPC API layer: REST (Axum), FHIR R5, and a gRPC stub.
pub mod api;
/// Loco application hooks (boot, routes, migrations).
pub mod app;
/// Configuration structs and environment loading.
pub mod config;
/// PostgreSQL persistence: SeaORM entities, repositories, audit log.
pub mod db;
/// Crate-wide error type and `Result` alias.
pub mod error;
/// Matching engine: probabilistic + deterministic scorers and the matcher adapter.
pub mod matching;
/// Prometheus metric definitions and text-exposition rendering.
pub mod metrics;
/// Domain models: the [`Worker`](models::Worker) record and its sub-objects.
pub mod models;
/// Observability: OpenTelemetry tracing and metrics setup.
pub mod observability;
/// Privacy controls: field masking, consent checking, GDPR export.
pub mod privacy;
/// Full-text search backed by Tantivy.
pub mod search;
/// Event streaming: producers and consumers for CRUD events.
pub mod streaming;
/// Data-quality validation and normalization.
pub mod validation;

// Re-exports: surface the most commonly used error types at the crate root so
// callers can write `worker_service::Result<T>` and `worker_service::Error`.
pub use error::{Error, Result};

#[cfg(test)]
mod tests {
    /// Smoke test: the crate's public [`Result`] alias is reachable and usable.
    #[test]
    fn test_module_imports() {
        // Verify key types are accessible
        let _: fn() -> crate::Result<()> = || Ok(());
    }
}
