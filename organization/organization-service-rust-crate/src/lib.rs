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
//! detail see the `AGENTS/*` files.

// Always start with high quality coding conventions.
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Loco application hooks: route registration, boot, and test truncation.
pub mod app;
/// Bearer-token (RS256 JWT) verification extractors for the REST surface.
pub mod auth;
/// REST controllers: organization CRUD, matching, audit, and the docs endpoints.
pub mod controllers;
/// Loco initializers (currently empty; reserved for app-startup wiring).
pub mod initializers;
/// Pure record-merge logic (fold a duplicate into a survivor).
pub mod merge;
/// Process-wide Prometheus metrics, served at `/metrics.prom`.
pub mod metrics;
/// `SeaORM` entities plus the CRUD/audit helpers layered over them.
pub mod models;
/// Hand-written OpenAPI 3 document served at `/api-docs/openapi.json`.
pub mod openapi;
/// In-memory event stream published on every CRUD action.
pub mod streaming;
