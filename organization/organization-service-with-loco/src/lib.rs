//! Organization Service — schema.org/Organization-aligned identity registry.
//!
//! Library entry-point for a loco.rs service that stores and matches
//! organization records. The API DTO **is**
//! [`organization_matcher::Organization`]: the service persists it
//! verbatim as JSONB and matches with the same type, so there is no
//! separate model to drift.
//!
//! Re-exports the loco [`app::App`] hooks, the REST [`controllers`], the
//! `SeaORM` [`models`], the hand-written [`openapi`] document, and the
//! in-memory [`streaming`] event buffer. For the canonical behaviour
//! reference see [`../spec/index.md`](../spec/index.md); for per-area
//! detail see the `agents/*` files.

// Always start with high quality coding conventions.
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Loco application hooks: route registration, boot, and test truncation.
pub mod app;
/// Bearer-token (RS256 JWT) verification extractors for the REST surface.
pub mod auth;
/// Async bulk import/export (BLK-5): job model, codecs (JSONL/CSV),
/// pipeline, artifact store, background worker, and REST handlers.
/// See `agents/share/bulk-import-export.md`.
pub mod bulk;
/// REST controllers: organization CRUD, matching, audit, and the docs endpoints.
/// Compliance controls: row-level integrity and keyed MACs.
pub mod compliance;
pub mod controllers;

/// HL7 FHIR R5 interop (`Organization` resource) — conversions, wire
/// types, and search-parameter parsing for the mounted `/fhir` endpoints.
pub mod fhir;
/// Loco initializers (currently empty; reserved for app-startup wiring).
pub mod initializers;
/// Pure record-merge logic (fold a duplicate into a survivor).
pub mod merge;
/// Process-wide Prometheus metrics, served at `/metrics.prom`.
pub mod metrics;
/// `SeaORM` entities plus the CRUD/audit helpers layered over them.
pub mod models;
/// Structured logging plus real `OpenTelemetry` OTLP export (repo
/// `tasks.md` PRO-H12).
pub mod observability;
/// Hand-written OpenAPI 3 document served at `/api-docs/openapi.json`.
pub mod openapi;

/// Field masking + the GDPR right-of-access export.
pub mod privacy;
/// Durable event bus Phase 3: the outbox relay (drain → sink → mark
/// published) + retention purge. See [`agents/share/event-bus.md`].
pub mod relay;
/// Tantivy full-text search: index schema, engine, and query surface.
pub mod search;
/// In-memory event stream published on every CRUD action.
pub mod streaming;
/// CLI tasks (`cargo loco task <name>`).
pub mod tasks;
/// Field-level validation for incoming `Organization` payloads (→ `422`).
pub mod validation;
/// Header-based API versioning (`Accepts-version`) for the REST surface.
pub mod version;
