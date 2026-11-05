//! Placeholder for route grouping and per-group middleware.
//!
//! Routes are currently assembled directly in
//! [`crate::api::rest::create_router`]. This module is reserved for splitting
//! them into logical sub-routers (and attaching group-scoped middleware) if
//! that wiring grows; it is intentionally empty for now.

// This module can be used for organizing routes into logical groups
// and applying middleware to specific route groups
