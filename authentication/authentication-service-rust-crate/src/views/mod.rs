//! HTTP response view types — the JSON DTOs returned by the controllers.
//!
//! Views own the wire shape so handlers stay thin and so sensitive
//! fields (password hash, api key, magic-link token) can never leak into
//! a response by construction.

/// Auth response DTOs: login / current-user / GDPR account export.
pub mod auth;
