//! Route grouping helpers (reserved).
//!
//! Placeholder for organizing routes into logical groups and attaching
//! per-group middleware (e.g. auth on mutating routes). Today all routes
//! are assembled inline in [`create_router`](crate::api::rest::create_router); this
//! module exists for when that wiring grows large enough to split out.

// This module can be used for organizing routes into logical groups
// and applying middleware to specific route groups
