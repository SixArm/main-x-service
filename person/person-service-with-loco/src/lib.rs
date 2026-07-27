//! Person Service
//!
//! A healthcare person identification and matching system built with Rust.
//!
//! This library provides:
//! - Person matcher algorithms (probabilistic and deterministic)
//! - Full-text search capabilities via Tantivy
//! - `RESTful` API via Axum
//! - HL7 FHIR R5 support
//! - gRPC API via Tonic
//! - `PostgreSQL` persistence via `SeaORM`
//! - Event streaming via Fluvio
//! - Data quality validation
//! - Privacy and data masking
//! - Record merging and deduplication
//! - Distributed tracing and observability via OpenTelemetry

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// Module declarations
/// REST, FHIR R5, and gRPC API layers plus shared response envelopes.
pub mod api;
/// Loco application hooks (boot, routes, migrations).
pub mod app;
/// Bulk import/export — async `bg_pg` jobs, JSONL codec, artifact store.
pub mod bulk;
/// Regulatory-compliance controls: the tamper-evident audit chain,
/// adopted from the care-pathway reference implementation
/// (`spec/compliance` §8.5 step 3).
pub mod compliance;
/// Configuration structs and environment/`.env` loading.
pub mod config;
/// PostgreSQL persistence: SeaORM entities, repositories, audit log.
pub mod db;
/// Crate-wide [`Error`] enum and [`Result`] alias.
pub mod error;
/// Probabilistic and deterministic person matching engines.
pub mod matching;
/// Prometheus metric definitions and the text-exposition surface.
pub mod metrics;
/// Domain models (Person, Identifier, Organization, Consent, …).
pub mod models;
/// OpenTelemetry/tracing initialization and custom metrics.
pub mod observability;
/// Data masking, GDPR export, and consent checking.
pub mod privacy;
/// Durable event bus **Phase 3**: the outbox relay loop + retention purge.
pub mod relay;
/// Tantivy full-text search engine and index.
pub mod search;
/// Event streaming (PersonEvent, producers, consumers).
pub mod streaming;
/// CLI tasks (loco extension point).
pub mod tasks;
/// Data-quality validation, normalization, and standardization.
pub mod validation;

// Re-exports
pub use error::{Error, Result};

/// Smoke tests for the crate's public surface.
#[cfg(test)]
mod tests {
    /// Verify key public types/aliases are reachable from the crate root.
    #[test]
    fn test_module_imports() {
        // Verify key types are accessible
        let _: fn() -> crate::Result<()> = || Ok(());
    }
}
