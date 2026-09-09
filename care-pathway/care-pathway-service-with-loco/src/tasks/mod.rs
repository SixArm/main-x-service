//! CLI tasks (loco extension point).

/// Generate, check, or report the integrity MAC key.
pub mod integrity_key;
/// Re-MAC history under the current key after a rotation.
pub mod integrity_resign;
/// `journeys:seed` — persist a deterministic synthetic journey cohort.
pub mod journeys_seed;
/// `search_reindex` — rebuild the full-text index from the database.
pub mod search;
