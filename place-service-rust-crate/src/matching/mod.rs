pub mod adapter;
pub mod address;
pub mod geo;
pub mod identifier;
pub mod name;
pub mod phonetic;
pub mod scoring;

/// Re-export the canonical `place-matcher` library so callers can reach
/// `MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, the
/// `Place` builder, and all `PlaceCategory` / `PlaceIdScheme` variants
/// without taking a separate dependency. Pair this with
/// [`adapter::to_matcher_place`] to score two service `Place` records
/// through the reference algorithm.
pub use ::place_matcher as matcher_lib;
