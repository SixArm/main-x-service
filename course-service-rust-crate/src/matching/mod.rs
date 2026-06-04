//! Service-side matcher facade. Thin wrapper around the canonical
//! `course-matcher` crate so the service's `AppState` carries a single
//! `CourseMatcher` value that adapts service-side `Course` records
//! into the matcher's domain types.
//!
//! See `AGENTS/matching.md` for the field-routing rules and per-
//! component weights.

use crate::config::MatchingConfig;
use crate::models::Course;

/// Wraps `course_matcher::MatchingEngine` and stores the configured
/// threshold. The adapter for service-Course → matcher-Course lives
/// in `adapter.rs` (next iteration).
pub struct CourseMatcher {
    threshold: f64,
}

impl CourseMatcher {
    pub fn new(config: MatchingConfig) -> Self {
        Self { threshold: config.threshold_score }
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Score two service-side `Course` records. STUB returning a
    /// zero score until the adapter + `course-matcher` integration
    /// lands. Always returns `false` for `is_match`.
    pub fn match_courses(&self, _a: &Course, _b: &Course) -> MatchResult {
        MatchResult::default()
    }

    /// Find probable duplicates from a candidate pool. STUB — returns
    /// an empty list.
    pub fn find_matches(&self, _course: &Course, _candidates: &[Course]) -> Vec<MatchResult> {
        Vec::new()
    }
}

/// Service-side match result (mirrors the canonical
/// `course_matcher::MatchResult` shape so the REST layer doesn't have
/// to convert).
#[derive(Debug, Clone, Default)]
pub struct MatchResult {
    pub score: f64,
    pub is_match: bool,
    pub confidence: MatchConfidence,
    pub breakdown: MatchBreakdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchConfidence {
    High,
    Medium,
    #[default]
    Low,
}

#[derive(Debug, Clone, Default)]
pub struct MatchBreakdown {
    pub name_score: Option<f64>,
    pub course_code_score: Option<f64>,
    pub provider_score: Option<f64>,
    pub educational_level_score: Option<f64>,
    pub keywords_score: Option<f64>,
    pub teaches_score: Option<f64>,
    pub identifier_score: Option<f64>,
    pub deterministic_match: bool,
}
