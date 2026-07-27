//! Compliance controls: row-level integrity for organization records and
//! the audit trail.
//!
//! ## What is here, and what is deliberately not
//!
//! Each organization row and each `audit_logs` row carries three stored
//! values over the same pre-image: SHA-256, SHA3-256, and a keyed
//! HMAC-SHA256. The **MAC is the one that defends against a deliberate
//! edit** — the two digests are unkeyed and their pre-image format is
//! published in `spec/12-compliance.md`, so anyone who can write SQL
//! recomputes them. What the digests catch is careless or unaware
//! modification; what the MAC catches is someone who knows exactly what
//! they are doing but does not hold the key.
//!
//! **There is no hash chain here yet.** The person, worker, care-pathway,
//! and case services link their audit rows into a chain
//! (`prev_hash`/`hash`) and take external-witness checkpoints, which
//! together detect deletion and reordering. This service has neither, so
//! state the limit plainly: a MAC proves a row's *content* is unchanged
//! since it was written, and says nothing about a row that was **deleted
//! wholesale**. Adding the chain is a separate, larger control and is not
//! claimed by anything in this module.
//!
//! ## Default off
//!
//! With no `ORGANIZATION_INTEGRITY_MAC_KEY` (or `..._KEY_FILE`)
//! configured, no MAC is written and rows are reported `mac_absent`
//! rather than as mismatches. Adopting the control on a populated table
//! must not produce a wall of false accusations.

/// Keyed integrity MACs — this crate's binding to the shared
/// `integrity-mac` crate.
pub mod mac;

/// Keyed integrity over `audit_logs` rows.
pub mod audit_integrity;

/// Row-level integrity hashing over the `organizations` table.
pub mod record_integrity;
