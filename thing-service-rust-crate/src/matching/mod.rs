pub mod adapter;
pub mod description;
pub mod identifier;
pub mod name;
pub mod phonetic;
pub mod scoring;
pub mod url;

/// Re-export the canonical `thing-matcher` library so callers can reach
/// `MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, and
/// the `Thing` builder without taking a separate dependency. Pair this
/// with [`adapter::to_matcher_thing`] to score two service `Thing`
/// records through the reference algorithm.
pub use ::thing_matcher as matcher_lib;
