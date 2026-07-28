//! HTTP controllers for the link-graph read-model aggregator. Read-only
//! to the world (spec §9): no link-write endpoints live here.

/// Integrity-verification endpoint for the audit trail.
pub mod compliance;
pub mod docs;
pub mod graph;
pub mod metrics;
