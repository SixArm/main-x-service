//! Compliance controls: keyed integrity over this service's audit trail.
//!
//! ## Scope
//!
//! Only the `audit_log` table is MACed. The `edges` table is a **derived
//! read-model** rebuilt from each entity service's event stream, so a MAC
//! there would attest to a projection: if it disagreed, the right
//! response is to rebuild, not to open a tampering investigation. The
//! authoritative edges live in each originating service's `entity_links`
//! and are protected there.
//!
//! The audit trail is not rebuildable from anywhere, which is exactly why
//! it is worth protecting here.
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
//! With no `LINK_GRAPH_INTEGRITY_MAC_KEY` (or `..._KEY_FILE`) configured,
//! no MAC is written and rows report as absent rather than as mismatches.

/// Keyed integrity MACs — this crate's binding to the shared
/// `integrity-mac` crate.
pub mod mac;

/// Keyed integrity over `audit_log` rows.
pub mod audit_integrity;
