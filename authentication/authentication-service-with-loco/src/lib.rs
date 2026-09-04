//! Authentication service — the Main X Index federation's central
//! single sign-on provider.
//!
//! A loco.rs application that authenticates users via passwordless email
//! magic links and issues short-lived PASETO v4.public (Ed25519) access
//! tokens. Every other service verifies those tokens offline against the
//! public key set published at `/.well-known/paseto-keys` (see the
//! sibling `authentication-verifier` crate). There is no shared secret
//! and no per-request introspection.
//!
//! See `spec/index.md` for the living specification and `AGENTS.md` for
//! the working conventions.
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Loco `Hooks` boot: route registration, workers, seeding, truncation.
pub mod app;
/// PASETO v4.public issuance, verification, key-set publication, bearer extractor.
pub mod auth;
/// Loco HTTP controllers (magic-link auth + paseto-keys endpoint).
/// Compliance controls: keyed integrity over the auth-event trail.
pub mod compliance;
pub mod controllers;
/// The `__Host-mxi_session` session cookie (build / clear / parse).
pub mod cookie;
/// CSRF synchroniser-token helpers (generation, constant-time compare,
/// and the `__Host-mxi_csrf` non-httpOnly delivery cookie).
pub mod csrf;
/// Static data loaders (loco `data/` convention).
pub mod data;
/// Localised user-facing copy (magic-link email subject + bodies).
pub mod i18n;
/// Loco app initializers.
pub mod initializers;
/// Mailers (magic-link / welcome emails).
pub mod mailers;
/// Prometheus metrics registry + `/metrics.prom` rendering.
pub mod metrics;
/// Database migrations (`SeaORM` / loco migrator).
pub mod migration;
/// Domain models and generated `SeaORM` entities.
pub mod models;
/// Hand-written `OpenAPI` 3 document for the auth API.
pub mod openapi;
/// Postgres-backed per-email sliding-window rate limiter for magic-link issuance.
pub mod rate_limit;
/// Hash-at-rest of bearer-equivalent secrets (SEC-A9).
pub mod secret_hash;
/// Loco CLI tasks.
pub mod tasks;
/// Header-based API versioning (`Accepts-version`; agents/share/api-versioning.md).
pub mod version;
/// HTTP response view types.
pub mod views;
