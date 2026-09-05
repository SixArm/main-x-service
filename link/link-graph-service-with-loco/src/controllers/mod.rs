//! HTTP controllers for the link-graph read-model aggregator. Read-only
//! to the world (spec §9): no link-write endpoints live here.

/// Operator control-plane endpoint: force a reconciliation pass (T-36).
pub mod admin;
/// Integrity-verification endpoint for the audit trail.
pub mod compliance;
pub mod docs;
pub mod graph;
pub mod metrics;
