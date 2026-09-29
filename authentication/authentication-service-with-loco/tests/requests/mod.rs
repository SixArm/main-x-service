//! HTTP request-layer integration tests for the auth API.
//!
//! `auth` holds the endpoint tests (DB-gated flows + DB-free contract
//! assertions); `prepare_data` holds shared sign-in helpers; `rate_limit`
//! holds the DB-gated sliding-window limiter tests.

#[cfg(feature = "paseto")]
mod admin;
mod auth;
#[cfg(feature = "paseto")]
mod compliance;
#[cfg(feature = "oidc")]
mod oidc;
#[cfg(feature = "paseto")]
mod prepare_data;
mod rate_limit;
