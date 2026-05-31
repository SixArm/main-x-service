//! Event matcher strategies and scoring.
//!
//! [`EventMatcher`] is the trait that callers use; concrete
//! strategies are [`ProbabilisticMatcher`] (weighted fuzzy) and
//! [`DeterministicMatcher`] (rule-based).

use crate::config::MatchingConfig;
use crate::models::Event;
use crate::Result;

pub mod algorithms;
pub mod phonetic;
pub mod scoring;

pub use scoring::{DeterministicScorer, MatchQuality, ProbabilisticScorer};

/// One candidate event with its score and per-component breakdown.
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub event: Event,
    pub score: f64,
    pub breakdown: MatchScoreBreakdown,
}

/// Per-component scores produced by a scorer.
#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct MatchScoreBreakdown {
    pub name_score: f64,
    pub start_date_score: f64,
    pub end_date_score: f64,
    pub location_score: f64,
    pub organizer_score: f64,
    pub performer_score: f64,
    pub attendee_score: f64,
    pub identifier_score: f64,
}

impl MatchScoreBreakdown {
    /// Human-readable summary listing which components matched well.
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

/// Match-strategy trait.
pub trait EventMatcher: Send + Sync {
    fn match_events(&self, event: &Event, candidate: &Event) -> Result<MatchResult>;
    fn find_matches(&self, event: &Event, candidates: &[Event]) -> Result<Vec<MatchResult>>;
    fn is_match(&self, score: f64) -> bool;
}

/// Weighted-fuzzy matching strategy.
pub struct ProbabilisticMatcher {
    scorer: ProbabilisticScorer,
    threshold: f64,
}

impl ProbabilisticMatcher {
    pub fn new(config: MatchingConfig) -> Self {
        let threshold = config.threshold_score;
        Self {
            scorer: ProbabilisticScorer::new(config),
            threshold,
        }
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    pub fn classify_match(&self, score: f64) -> MatchQuality {
        self.scorer.classify_match(score)
    }
}

impl EventMatcher for ProbabilisticMatcher {
    fn match_events(&self, event: &Event, candidate: &Event) -> Result<MatchResult> {
        Ok(self.scorer.calculate_score(event, candidate))
    }

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

/// Rule-based matching strategy.
pub struct DeterministicMatcher {
    scorer: DeterministicScorer,
}

impl DeterministicMatcher {
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            scorer: DeterministicScorer::new(config),
        }
    }
}

impl EventMatcher for DeterministicMatcher {
    fn match_events(&self, event: &Event, candidate: &Event) -> Result<MatchResult> {
        Ok(self.scorer.calculate_score(event, candidate))
    }

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
    use chrono::{TimeZone, Utc};

    fn config() -> MatchingConfig {
        MatchingConfig {
            threshold_score: 0.85,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        }
    }

    #[test]
    fn find_matches_returns_only_above_threshold() {
        let m = ProbabilisticMatcher::new(MatchingConfig {
            threshold_score: 0.20,
            ..config()
        });
        let when = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap();
        let query = Event::new("Conference", when);
        let candidates = vec![
            Event::new("Conference", when),
            Event::new("Totally Different Event", when + chrono::Duration::days(40)),
        ];
        let matches = m.find_matches(&query, &candidates).unwrap();
        assert!(!matches.is_empty());
        // Sorted descending
        for w in matches.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

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

    #[test]
    fn identifier_short_circuits_match() {
        let m = ProbabilisticMatcher::new(config());
        let when = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap();
        let mut a = Event::new("A", when);
        let mut b = Event::new("Z", when + chrono::Duration::days(30));
        let id = Identifier::new(IdentifierType::TicketNumber, "sys".into(), "T-1".into());
        a.identifiers.push(id.clone());
        b.identifiers.push(id);
        let r = m.match_events(&a, &b).unwrap();
        assert_eq!(r.score, 1.0);
    }
}
