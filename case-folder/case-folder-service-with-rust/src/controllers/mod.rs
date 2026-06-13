//! JSON API controllers.
//!
//! Every route is mounted under `/api` (versioning is signalled by an
//! HTTP `Accept` / `Content-Type` mediatype, not by a URL prefix).
//! The single exception is `/healthz`, mounted at the root for
//! convention.

pub mod alerts;
pub mod auth;
pub mod folders;
pub mod healthz;
pub mod moves;
pub mod patients;
pub mod places;
pub mod stats;
pub mod volumes;
pub mod workers;
