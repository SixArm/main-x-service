//! Service-side matcher facade — drives the canonical
//! `course-matcher` crate through [`adapter::to_matcher_course`].
//!
//! Per-component weights and deterministic-rule semantics live in the
//! matcher crate; this facade only configures the `is_match` threshold
//! and re-exports the result shape. See `AGENTS/matching.md` for the
//! field-routing rules.

use course_matcher::{MatchConfig, MatchingEngine};
use serde::Serialize;
use utoipa::ToSchema;

use crate::config::MatchingConfig;
use crate::models::Course;

pub mod adapter;

/// Re-export of the canonical `course-matcher` library crate so
/// callers (integration tests, the bridge suite under `tests/`) can
/// access `MatchingEngine`, `MatchConfig`, `Confidence`, etc without
/// adding the matcher as their own direct dependency.
pub use course_matcher as matcher_lib;

/// Wraps `course_matcher::MatchingEngine` with the service's
/// configured threshold.
pub struct CourseMatcher {
    engine: MatchingEngine,
    threshold: f64,
}

impl CourseMatcher {
    pub fn new(config: MatchingConfig) -> Self {
        let mut cfg = MatchConfig::default();
        cfg.threshold = config.threshold_score;
        Self {
            threshold: config.threshold_score,
            engine: MatchingEngine::new(cfg),
        }
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Score two service-side `Course` records via the canonical
    /// `course-matcher` algorithm.
    pub fn match_courses(&self, a: &Course, b: &Course) -> MatchResult {
        let ma = adapter::to_matcher_course(a);
        let mb = adapter::to_matcher_course(b);
        let r = self.engine.match_courses(&ma, &mb);
        from_matcher_result(r)
    }

    /// Rank `candidates` against `course` by descending score.
    pub fn find_matches(&self, course: &Course, candidates: &[Course]) -> Vec<MatchResult> {
        let mc = adapter::to_matcher_course(course);
        let mcands: Vec<_> = candidates.iter().map(adapter::to_matcher_course).collect();
        self.engine
            .rank(&mc, &mcands)
            .into_iter()
            .map(|(_idx, r)| from_matcher_result(r))
            .collect()
    }
}

fn from_matcher_result(r: course_matcher::MatchResult) -> MatchResult {
    MatchResult {
        score: r.score,
        is_match: r.is_match,
        confidence: match r.confidence {
            course_matcher::Confidence::High => MatchConfidence::High,
            course_matcher::Confidence::Medium => MatchConfidence::Medium,
            course_matcher::Confidence::Low => MatchConfidence::Low,
        },
        breakdown: MatchBreakdown {
            name_score: r.breakdown.name_score,
            course_code_score: r.breakdown.course_code_score,
            provider_score: r.breakdown.provider_score,
            educational_level_score: r.breakdown.educational_level_score,
            keywords_score: r.breakdown.keywords_score,
            teaches_score: r.breakdown.teaches_score,
            identifier_score: None,
            deterministic_match: r.breakdown.deterministic_match,
        },
    }
}

/// Service-side match result. Mirrors `course_matcher::MatchResult` so
/// REST handlers can pass it straight through without translation.
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

#[derive(Debug, Clone, Default, Serialize, ToSchema)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MatchingConfig;
    use crate::models::{Course, CourseIdentifier, IdentifierType};

    fn cfg() -> MatchingConfig {
        MatchingConfig { threshold_score: 0.85 }
    }

    #[test]
    fn identical_records_score_one() {
        let m = CourseMatcher::new(cfg());
        let a = Course::new("Intro to Computer Science");
        let b = a.clone();
        let r = m.match_courses(&a, &b);
        assert!(r.score >= 0.95, "expected near-1, got {}", r.score);
        assert!(r.is_match);
    }

    #[test]
    fn doi_short_circuits_to_one() {
        let m = CourseMatcher::new(cfg());
        let mut a = Course::new("CS101");
        let mut b = Course::new("Totally Different Title");
        let ident = |v: &str| CourseIdentifier {
            property_id: IdentifierType::Doi,
            value: v.into(),
            name: None,
            url: None,
        };
        a.identifiers = vec![ident("10.1234/abc")];
        b.identifiers = vec![ident("10.1234/abc")];
        let r = m.match_courses(&a, &b);
        assert!(r.breakdown.deterministic_match);
        assert!((r.score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn find_matches_orders_by_score() {
        let m = CourseMatcher::new(cfg());
        let probe = Course::new("Linear Algebra");
        let close = Course::new("Linear Algebra I");
        let far = Course::new("Organic Chemistry");
        let ranked = m.find_matches(&probe, &[far.clone(), close.clone()]);
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].score >= ranked[1].score);
    }
}
