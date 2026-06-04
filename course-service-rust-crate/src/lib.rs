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

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod matching;
pub mod models;
pub mod search;

pub use error::{Error, Result};
