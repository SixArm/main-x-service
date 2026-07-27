//! Compliance controls: keyed integrity over the `auth_events` trail.
//!
//! ## Scope, and why it is this table
//!
//! `auth_events` records **who logged in and who was granted which
//! authorization attributes**. The `attributes_assigned` event is the
//! ABAC grant trail — the evidence that someone gave an account
//! `access=admin`. An attacker who escalated privilege and could then
//! edit that row would erase the only account of how they did it.
//!
//! `sessions` are ephemeral and already expire; `users` rows change
//! through ordinary account operations. Neither is where a quiet rewrite
//! would be most damaging.
//!
//! ## Key separation from the token signing key
//!
//! This is **not** the key that signs PASETO tokens
//! (`TOKEN_PRIVATE_KEY_SEED`). It is a separate secret,
//! `AUTH_INTEGRITY_MAC_KEY`, for a separate purpose, and the two must not
//! be conflated: the signing key is handed to a verifier population by
//! design, while this one never leaves the service.
//!
//! ## What a MAC does and does not prove
//!
//! It proves a row's **content** is unchanged since it was written, to
//! anyone who does not hold the key. It says nothing about a row
//! **deleted wholesale** — that needs a hash chain and external-witness
//! checkpoints, which this service does not have.
//!
//! ## Default off
//!
//! With no `AUTH_INTEGRITY_MAC_KEY` (or `..._KEY_FILE`) configured, no
//! MAC is written and rows report as absent rather than as mismatches.

/// Keyed integrity MACs — this crate's binding to the shared
/// `integrity-mac` crate.
pub mod mac;

/// Keyed integrity over `auth_events` rows.
pub mod audit_integrity;
