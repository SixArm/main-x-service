//! Course Service — schema.org/Course-aligned course-identity registry.
//!
//! Library entry-point. Re-exports the domain models and the
//! HTTP-layer plumbing so downstream callers (test harnesses, the
//! `course-service` binary, future gRPC stubs) can build a
//! [`api::rest::AppState`] without poking at internal modules.
//!
//! For the canonical behaviour reference, see [`../spec.md`](../spec.md).
//! For per-area detail (domain model, matching, REST surface, testing),
//! see the `AGENTS/*` files under [`AGENTS/`](../AGENTS/).

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod matching;
pub mod models;
pub mod privacy;
pub mod search;
pub mod streaming;
pub mod validation;

/// Crate-wide [`enum@Error`] enum and [`Result`] alias, re-exported at the
/// crate root for convenient `course_service::{Error, Result}` use.
pub use error::{Error, Result};
