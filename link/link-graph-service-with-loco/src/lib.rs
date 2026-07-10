//! `link-graph-service` — the **read-model aggregator** for cross-service
//! entity linking (the §4.3 read side of
//! `agents/share/cross-service-linking.md`).
//!
//! A loco.rs backend service that consumes every entity's event stream
//! (plus the `linked` / `unlinked` events) and serves one queryable
//! cross-service graph (`neighbors` / `edges` / `single-view` /
//! freshness). It is **read-only to the world**: all its writes are
//! event-driven, folded in through the [`events::apply_event`] seam.
//!
//! The shared cross-service-linking contracts ([`EntityType`],
//! [`EntityRef`], the closed [`EdgeKind`] registry) are provided by the
//! sibling [`entity_ref`] crate and depended on here rather than
//! re-copied (spec §5.1 / §13 T-2/T-3 deviation).
//!
//! ## Modules
//!
//! - [`graph`] — pure, DB-free projection logic (canonicalisation,
//!   status lifecycle, single-view walk).
//! - [`events`] — event decode + the `apply_event` seam.
//! - [`models`] — `SeaORM` entities + query/projection helpers.
//! - [`controllers`] — the read-only Axum controllers.
//! - [`envelope`] — the `{ success, data, error }` API wrapper.
//! - [`app`] — loco `Hooks` wiring.
//!
//! See `spec/index.md` for the living specification.
//!
//! [`EntityType`]: entity_ref::EntityType
//! [`EntityRef`]: entity_ref::EntityRef
//! [`EdgeKind`]: entity_ref::EdgeKind

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod app;
pub mod auth;
pub mod controllers;
pub mod envelope;
pub mod events;
pub mod graph;
pub mod models;
pub mod openapi;
