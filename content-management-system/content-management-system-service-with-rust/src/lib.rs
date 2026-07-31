//! `content-management-system-service` — a loco.rs consumer
//! application for **headless content management** across six
//! modules: content modelling & authoring (operator-defined content
//! types, structured block documents, append-only revisions), digital
//! assets, editorial workflow (draft → review → approved → published
//! → archived), localization (locale variants, fallback chains,
//! translation staleness), delivery & SEO (published-only structured
//! JSON, routing, sitemaps), and content insights.
//!
//! CMS **owns content and editorial state** (its own tables) and
//! **references identities**: authors / editors / translators are
//! worker-service records and a site's owning body an
//! organization-service record, as `EntityRef` URNs — never
//! duplicated. Content *about* a registered entity carries that
//! entity's URN as a pointer, and CMS never becomes the registry.
//! **Readers are not modelled at all**: no visitor identity, no
//! profile store (spec `scope.md`, CMS-D1/CMS-D11).
//!
//! ## Modules
//!
//! - [`app`] — loco `Hooks` wiring (routes, truncate, seed).
//! - [`controllers`] — Axum controllers, one per CMS area.
//! - [`rules`] — the **pure core**: content-type schema validation and
//!   compatibility classification, locale fallback-chain validation,
//!   and (as later phases land) the lifecycle machines, block
//!   validation, routing, and insight derivations.
//! - [`models`] — `SeaORM` entities + CRUD helpers.
//! - [`clients`] — upstream service lookups (stub-first).
//! - [`auth`] — offline PASETO verification + ABAC + masking.
//! - [`streaming`] — event envelope + in-memory / outbox transports.
//! - [`storage`] — the family `ArtifactStore` seam holding asset bytes.
//!
//! See `../spec/index.md` for the living specification, and
//! `../../spec/tasks.md` for the delivery queue (CMS-T*).
//!
//! ## Delivery status
//!
//! **Phase 1 (CMS-T1–T4)**: the scaffold, sites + templates, content
//! types with the compatibility classifier, and the upstream client
//! seam. Entries, revisions, assets, workflow, localization,
//! delivery, and insights are CMS-T5 onward.

#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
// The hand-written OpenAPI document is one large nested `json!` literal.
#![recursion_limit = "512"]

pub mod app;
pub mod auth;
pub mod clients;
pub mod controllers;
pub mod metrics;
pub mod models;
pub mod openapi;
pub mod rules;
pub mod storage;
pub mod streaming;
pub mod tasks;
pub mod validation;
/// Header-based API versioning (`Accepts-version`) for the REST surface.
pub mod version;
