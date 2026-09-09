//! Static / seed data loaders (loco extension point).

/// Deterministic, DB-free generation of synthetic pathway-instance
/// journey cohorts (spec `13-tasks.md` T-14m), persisted by the
/// `journeys:seed` CLI task.
pub mod journeys;
