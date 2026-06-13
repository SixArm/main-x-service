//! Authentication service — the Main X Index federation's central
//! single sign-on provider.
//!
//! A loco.rs application that authenticates users via passwordless email
//! magic links and issues RS256 JWT access tokens. Every other service
//! verifies those tokens offline against the public keys published at
//! `/.well-known/jwks.json` (see the sibling `authentication-verifier`
//! crate). There is no shared secret and no per-request introspection.
//!
//! See `spec/index.md` for the living specification and `AGENTS.md` for
//! the working conventions.
#![warn(clippy::pedantic)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// Loco `Hooks` boot: route registration, workers, seeding, truncation.
pub mod app;
/// RS256 JWT issuance, verification, JWKS publication, bearer extractor.
pub mod auth;
/// Loco HTTP controllers (magic-link auth + JWKS endpoint).
pub mod controllers;
/// Static data loaders (loco `data/` convention).
pub mod data;
/// Localised user-facing copy (magic-link email subject + bodies).
pub mod i18n;
/// Loco app initializers.
pub mod initializers;
/// Mailers (magic-link / welcome emails).
pub mod mailers;
/// Database migrations (`SeaORM` / loco migrator).
pub mod migration;
/// Domain models and generated `SeaORM` entities.
pub mod models;
/// Hand-written `OpenAPI` 3 document for the auth API.
pub mod openapi;
/// In-memory per-email rate limiter for magic-link issuance.
pub mod rate_limit;
/// Loco CLI tasks.
pub mod tasks;
/// HTTP response view types.
pub mod views;
/// Background workers.
pub mod workers;
