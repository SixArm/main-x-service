//! Event matcher strategies and scoring.
//!
//! [`EventMatcher`](crate::matching::EventMatcher) is the trait that
//! callers use; concrete strategies are
//! [`ProbabilisticMatcher`](crate::matching::ProbabilisticMatcher)
//! (weighted fuzzy) and
//! [`DeterministicMatcher`](crate::matching::DeterministicMatcher)
//! (rule-based). Both delegate to the scorers in
//! [`scoring`](crate::matching::scoring), which build on the component
//! algorithms in [`algorithms`](crate::matching::algorithms).
//!
//! For a second, independent opinion this module also re-exports the
//! canonical `event-matcher` crate as
//! [`matcher_lib`](crate::matching::matcher_lib); pair it with
//! [`adapter::to_matcher_event`](crate::matching::adapter::to_matcher_event)
//! to score two service [`Event`](crate::models::Event)s through the
//! reference implementation.

use crate::config::MatchingConfig;
use crate::models::Event;
use crate::Result;

/// Bridge to the canonical `event-matcher` crate ([`to_matcher_event`](adapter::to_matcher_event)).
pub mod adapter;
/// Per-component matching algorithms (name, time, location, …).
pub mod algorithms;
/// Soundex phonetic coding used as a name-similarity floor.
pub mod phonetic;
/// Probabilistic and deterministic scorers + quality classification.
pub mod scoring;

pub use scoring::{DeterministicScorer, MatchQuality, ProbabilisticScorer};

/// Re-export the canonical `event-matcher` library so callers can reach
/// `MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, the
/// `Event` builder, and the `EventCategory` / `EventIdScheme` enums
/// without taking a separate dependency. Pair this with
/// [`adapter::to_matcher_event`] to score two service `Event` records
/// through the reference algorithm.
pub use ::event_matcher as matcher_lib;

/// One candidate event with its score and per-component breakdown.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// The candidate event that was scored.
    pub event: Event,
    /// Overall match score in `[0.0, 1.0]`.
    pub score: f64,
    /// Per-component scores that produced `score`.
    pub breakdown: MatchScoreBreakdown,
}

/// Per-component scores produced by a scorer. Each field is in
/// `[0.0, 1.0]`.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct MatchScoreBreakdown {
    /// Name / title (incl. alternate names) component score.
    pub name_score: f64,
    /// Start-date proximity component score.
    pub start_date_score: f64,
    /// End-date proximity component score.
    pub end_date_score: f64,
    /// Best location-pair component score.
    pub location_score: f64,
    /// Best organizer-pair component score.
    pub organizer_score: f64,
    /// Best performer-pair component score.
    pub performer_score: f64,
    /// Best attendee-pair component score.
    pub attendee_score: f64,
    /// Best identifier-pair component score.
    pub identifier_score: f64,
}

impl MatchScoreBreakdown {
    /// Human-readable summary listing which components matched well
    /// (above per-component display thresholds), or
    /// `"no strong matches"` when none cleared the bar.
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if self.name_score >= 0.90 {
            parts.push("name");
        }
        if self.start_date_score >= 0.95 {
            parts.push("start");
        }
        if self.end_date_score >= 0.95 {
            parts.push("end");
        }
        if self.location_score >= 0.80 {
            parts.push("location");
        }
        if self.organizer_score >= 0.90 {
            parts.push("organizer");
        }
        if self.performer_score >= 0.90 {
            parts.push("performer");
        }
        if self.attendee_score >= 0.90 {
            parts.push("attendee");
        }
        if self.identifier_score >= 0.95 {
            parts.push("identifier");
        }
        if parts.is_empty() {
            "no strong matches".into()
        } else {
            parts.join(", ")
        }
    }
}

/// Strategy trait implemented by both matchers. Object-safe so callers
/// can hold an `Arc<dyn EventMatcher>` in [`AppState`](crate::api::rest::state::AppState).
pub trait EventMatcher: Send + Sync {
    /// Score a single `candidate` against `event`.
    fn match_events(&self, event: &Event, candidate: &Event) -> Result<MatchResult>;
    /// Score all `candidates`, keep those above threshold, sorted by
    /// descending score.
    fn find_matches(&self, event: &Event, candidates: &[Event]) -> Result<Vec<MatchResult>>;
    /// Whether a score counts as a match for this strategy.
    fn is_match(&self, score: f64) -> bool;
}

/// Weighted-fuzzy matching strategy. Wraps a [`ProbabilisticScorer`].
pub struct ProbabilisticMatcher {
    /// The underlying weighted-sum scorer.
    scorer: ProbabilisticScorer,
    /// Cached match threshold (mirrors the scorer's config).
    threshold: f64,
}

impl ProbabilisticMatcher {
    /// Construct from a [`MatchingConfig`]; the threshold is taken from
    /// `config.threshold_score`.
    pub fn new(config: MatchingConfig) -> Self {
        let threshold = config.threshold_score;
        Self {
            scorer: ProbabilisticScorer::new(config),
            threshold,
        }
    }

    /// The configured match threshold.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Classify a score into a [`MatchQuality`] band.
    pub fn classify_match(&self, score: f64) -> MatchQuality {
        self.scorer.classify_match(score)
    }
}

impl EventMatcher for ProbabilisticMatcher {
    /// Delegates to [`ProbabilisticScorer::calculate_score`].
    fn match_events(&self, event: &Event, candidate: &Event) -> Result<MatchResult> {
        Ok(self.scorer.calculate_score(event, candidate))
    }

    /// Scores every candidate, drops sub-threshold results, and sorts
    /// the survivors by descending score.
    fn find_matches(&self, event: &Event, candidates: &[Event]) -> Result<Vec<MatchResult>> {
        let mut out: Vec<MatchResult> = candidates
            .iter()
            .map(|c| self.scorer.calculate_score(event, c))
            .filter(|r| self.is_match(r.score))
            .collect();
        // Best matches first; NaN scores are treated as equal so the
        // sort never panics.
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }

    fn is_match(&self, score: f64) -> bool {
        self.scorer.is_match(score)
    }
}

/// Rule-based matching strategy. Wraps a [`DeterministicScorer`].
pub struct DeterministicMatcher {
    /// The underlying rule-counting scorer.
    scorer: DeterministicScorer,
}

impl DeterministicMatcher {
    /// Construct from a [`MatchingConfig`].
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            scorer: DeterministicScorer::new(config),
        }
    }
}

impl EventMatcher for DeterministicMatcher {
    /// Delegates to [`DeterministicScorer::calculate_score`].
    fn match_events(&self, event: &Event, candidate: &Event) -> Result<MatchResult> {
        Ok(self.scorer.calculate_score(event, candidate))
    }

    /// Scores every candidate, drops sub-threshold results, and sorts
    /// the survivors by descending score.
    fn find_matches(&self, event: &Event, candidates: &[Event]) -> Result<Vec<MatchResult>> {
        let mut out: Vec<MatchResult> = candidates
            .iter()
            .map(|c| self.scorer.calculate_score(event, c))
            .filter(|r| self.is_match(r.score))
            .collect();
        out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(out)
    }

    fn is_match(&self, score: f64) -> bool {
        self.scorer.is_match(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Event, Identifier, IdentifierType};

    /// A baseline matching config used by these tests.
    fn config() -> MatchingConfig {
        MatchingConfig {
            threshold_score: 0.85,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        }
    }

    /// `find_matches` filters out sub-threshold candidates and returns
    /// the rest sorted by descending score.
    #[test]
    fn find_matches_returns_only_above_threshold() {
        let m = ProbabilisticMatcher::new(MatchingConfig {
            threshold_score: 0.20,
            ..config()
        });
        let when = jiff::civil::datetime(2026, 3, 1, 9, 0, 0, 0).in_tz("UTC").unwrap().timestamp();
        let query = Event::new("Conference", when);
        let candidates = vec![
            Event::new("Conference", when),
            Event::new("Totally Different Event", when + jiff::SignedDuration::from_hours(24 * (40))),
        ];
        let matches = m.find_matches(&query, &candidates).unwrap();
        assert!(!matches.is_empty());
        // Sorted descending
        for w in matches.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    /// `summary` names the components that cleared their display bar.
    #[test]
    fn breakdown_summary() {
        let b = MatchScoreBreakdown {
            name_score: 0.95,
            start_date_score: 0.99,
            end_date_score: 0.50,
            location_score: 0.85,
            organizer_score: 0.95,
            performer_score: 0.40,
            attendee_score: 0.40,
            identifier_score: 0.0,
        };
        let s = b.summary();
        assert!(s.contains("name"));
        assert!(s.contains("start"));
        assert!(s.contains("location"));
        assert!(s.contains("organizer"));
    }

    /// A shared strong identifier forces the score to 1.0.
    #[test]
    fn identifier_short_circuits_match() {
        let m = ProbabilisticMatcher::new(config());
        let when = jiff::civil::datetime(2026, 3, 1, 9, 0, 0, 0).in_tz("UTC").unwrap().timestamp();
        let mut a = Event::new("A", when);
        let mut b = Event::new("Z", when + jiff::SignedDuration::from_hours(24 * (30)));
        let id = Identifier::new(IdentifierType::TicketNumber, "sys".into(), "T-1".into());
        a.identifiers.push(id.clone());
        b.identifiers.push(id);
        let r = m.match_events(&a, &b).unwrap();
        assert_eq!(r.score, 1.0);
    }
}
