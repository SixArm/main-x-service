//! Main Worker Index (MPI)
//!
//! A healthcare worker identification and matching system built with Rust.
//!
//! This library provides:
//! - Worker matching algorithms (probabilistic and deterministic)
//! - Full-text search capabilities via Tantivy
//! - RESTful API via Axum
//! - HL7 FHIR R5 support
//! - gRPC API via Tonic
//! - PostgreSQL persistence via SeaORM
//! - Event streaming via Fluvio
//! - Data quality validation
//! - Privacy and data masking
//! - Record merging and deduplication
//! - Distributed tracing and observability via OpenTelemetry

// Module declarations
pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod matching;
pub mod models;
pub mod observability;
pub mod privacy;
pub mod search;
pub mod streaming;
pub mod validation;
pub mod web;

// Re-exports
pub use error::{Error, Result};

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_imports() {
        // Verify key types are accessible
        let _: fn() -> crate::Result<()> = || Ok(());
    }
}
