//! HTTP request-layer integration tests for the auth API.
//!
//! `auth` holds the endpoint tests (DB-gated flows + DB-free contract
//! assertions); `prepare_data` holds shared sign-in helpers.

mod auth;
mod prepare_data;
