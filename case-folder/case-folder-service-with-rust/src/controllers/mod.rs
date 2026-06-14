//! JSON API controllers.
//!
//! Every route is mounted under `/api` (versioning is signalled by an
//! HTTP `Accept` / `Content-Type` mediatype, not by a URL prefix).
//! The single exception is `/healthz`, mounted at the root for
//! convention.

/// Geofence-breach alerts — `GET /api/alerts`. Derives cross-building
/// moves from the move log joined to the place hierarchy.
pub mod alerts;

/// Passwordless magic-link authentication — `/api/auth/*` (request a
/// link, verify a token, current user, logout).
pub mod auth;

/// Case-folder resource — `/api/folders` CRUD plus current-location
/// lookups.
pub mod folders;

/// Liveness probe — `GET /healthz`, mounted at the root, with no
/// downstream calls.
pub mod healthz;

/// Move / scan log — `/api/moves`. The append-only audit trail that is
/// the source of truth for folder location.
pub mod moves;

/// Patient lookups — `/api/patients`, backed by the upstream Main
/// Patient Service.
pub mod patients;

/// Place hierarchy — `/api/places` (building → records room → file
/// cabinet), backed by the upstream Main Place Service.
pub mod places;

/// Dashboard statistics — `/api/stats` aggregate counters.
pub mod stats;

/// Per-folder physical volumes — `/api/volumes`.
pub mod volumes;

/// Worker lookups — `/api/workers`, backed by the upstream Main Worker
/// Service.
pub mod workers;
