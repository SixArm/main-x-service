//! Loco HTTP controllers for the authentication service.
//!
//! Each submodule exposes a `routes()` builder that [`crate::app::App`]
//! mounts. The surface is deliberately small: passwordless magic-link
//! auth, the public JWKS, and the `OpenAPI`/Swagger docs.

/// Passwordless magic-link auth (signup / request / redeem / me /
/// signout / audit + GDPR account routes), mounted under `/api/auth`.
pub mod auth;
/// `OpenAPI` JSON document + Swagger UI page (CDN assets).
pub mod docs;
/// Public JWKS (`/.well-known/jwks.json`) for offline peer verification.
pub mod jwks;
/// Prometheus metrics (`/metrics.prom`), mounted at the root.
pub mod metrics;
