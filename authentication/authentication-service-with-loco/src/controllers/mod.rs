//! Loco HTTP controllers for the authentication service.
//!
//! Each submodule exposes a `routes()` builder that [`crate::app::App`]
//! mounts. The surface is deliberately small: passwordless magic-link
//! auth, the published PASETO key set, and the `OpenAPI`/Swagger docs.

pub mod admin;
/// Passwordless magic-link auth (signup / request / redeem / me /
/// signout / audit + GDPR account routes), mounted under `/api/auth`.
pub mod auth;
/// Admin surface: ABAC attribute assignment over HTTP
/// (`/api/auth/admin/users/{pid}/attributes`), gated by `access=admin`.
/// Integrity-verification endpoint for the audit trail.
pub mod compliance;
/// `OpenAPI` JSON document + Swagger UI page (CDN assets).
pub mod docs;
/// Prometheus metrics (`/metrics.prom`), mounted at the root.
pub mod metrics;
/// Public key set (`/.well-known/paseto-keys`) for offline peer verification.
pub mod paseto_keys;
