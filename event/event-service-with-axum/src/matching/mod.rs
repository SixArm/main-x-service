//! Event matching, through the `event-matcher` crate.
//!
//! [`EventMatcher`] is the trait callers use; [`ProbabilisticMatcher`]
//! implements it by projecting both service [`Event`]s through
//! [`adapter::to_matcher_event`] and scoring them with the canonical
//! [`matcher_lib::MatchingEngine`]. The **score**, **`is_match`** and
//! **quality** are the matcher's, and so is the **evidence**
//! ([`evidence::MatchEvidence`], `agents/share/match-evidence.md`).
//!
//! Until spec §13 T-16 the service scored pairs itself
//! (`scoring::ProbabilisticScorer`, now removed), and counted a component
//! neither event held as `0`: two identical events without a shared
//! identifier reached only `0.75`, under the `0.85` threshold. The
//! matcher renormalises over the components both events hold, so absence
//! is not disagreement, and evidence (not similarity) gates auto-merge.
//!
//! [`algorithms`] and [`phonetic`] remain as standalone component
//! similarity functions (benchmarked in `benches/matching_bench.rs`); no
//! request path scores with them.

use crate::Result;
use crate::config::MatchingConfig;
use crate::models::Event;

pub mod adapter;
pub mod algorithms;
pub mod evidence;
pub mod frequencies;
pub mod phonetic;

pub use evidence::{MatchEvidence, permits_auto_merge};
pub use frequencies::{FrequencyCache, FrequencySource, MIN_FREQUENCY_POPULATION};

/// Re-export the canonical `event-matcher` library so callers can reach
/// `MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, the
/// `Event` builder, and the `EventCategory` / `EventIdScheme` enums
/// without taking a separate dependency. Pair this with
/// [`adapter::to_matcher_event`] to score two service `Event` records
/// through the reference algorithm.
pub use ::event_matcher as matcher_lib;

use ::event_matcher::{Confidence, MatchConfig, MatchingEngine, ValueFrequencies};

/// One candidate event with its score, verdict and evidence.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// The candidate event that was scored.
    pub event: Event,
    /// Overall match score in `[0.0, 1.0]`: the matcher's, renormalised
    /// over the components both events hold.
    pub score: f64,
    /// Whether the matcher counts the pair as a match (score at or above
    /// the configured `threshold_score`).
    pub is_match: bool,
    /// The matcher's coarse confidence band for the score.
    pub confidence: Confidence,
    /// Per-component scores that produced `score`.
    pub breakdown: MatchScoreBreakdown,
    /// How much the compared data proves (`match-evidence.md`).
    pub evidence: MatchEvidence,
}

impl MatchResult {
    /// The API's quality label for this result: the matcher's confidence
    /// band, `High` → `"certain"` (score `≥ 0.90`), `Medium` →
    /// `"probable"` (`≥ 0.75`), `Low` → `"possible"`. A label describes
    /// similarity only; whether a pair may be auto-merged is the
    /// evidence's call.
    #[must_use]
    pub fn quality(&self) -> &'static str {
        quality_label(self.confidence)
    }
}

/// See [`MatchResult::quality`].
#[must_use]
pub fn quality_label(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::High => "certain",
        Confidence::Medium => "probable",
        Confidence::Low => "possible",
    }
}

/// Per-component scores, the matcher's breakdown in the API's schema.
///
/// Each field is `null` when either event lacks the component, so it took
/// no part in the score: absence is not disagreement. Field names are the
/// matcher's (`event-matcher` `MatchBreakdown`); the service's own
/// breakdown before T-16 had `performer_score`, `attendee_score` and
/// `identifier_score`, which are now `performers_score`, (none; the
/// matcher does not compare attendees) and `event_ids_score`.
#[derive(
    Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema,
)]
pub struct MatchScoreBreakdown {
    /// Name / alternate-name similarity.
    pub name_score: Option<f64>,
    /// Soundex agreement over the names, when phonetic matching is on.
    pub name_phonetic_score: Option<f64>,
    /// Start-date proximity.
    pub start_date_score: Option<f64>,
    /// End-date proximity.
    pub end_date_score: Option<f64>,
    /// Venue / address / coordinates similarity.
    pub location_score: Option<f64>,
    /// Category (event type) equality.
    pub category_score: Option<f64>,
    /// Country-code equality.
    pub country_code_score: Option<f64>,
    /// Whether the events share an identifier.
    pub event_ids_score: Option<f64>,
    /// Organizer-name similarity.
    pub organizer_score: Option<f64>,
    /// Best performer-pair similarity.
    pub performers_score: Option<f64>,
    /// URL equality.
    pub url_score: Option<f64>,
    /// Relationship-set overlap.
    pub relationships_score: Option<f64>,
    /// Tag-set overlap.
    pub tags_score: Option<f64>,
}

impl From<&::event_matcher::MatchBreakdown> for MatchScoreBreakdown {
    fn from(b: &::event_matcher::MatchBreakdown) -> Self {
        Self {
            name_score: b.name_score,
            name_phonetic_score: b.name_phonetic_score,
            start_date_score: b.start_date_score,
            end_date_score: b.end_date_score,
            location_score: b.location_score,
            category_score: b.category_score,
            country_code_score: b.country_code_score,
            event_ids_score: b.event_ids_score,
            organizer_score: b.organizer_score,
            performers_score: b.performers_score,
            url_score: b.url_score,
            relationships_score: b.relationships_score,
            tags_score: b.tags_score,
        }
    }
}

impl MatchScoreBreakdown {
    /// Human-readable summary listing which components matched well
    /// (above per-component display thresholds), or
    /// `"no strong matches"` when none cleared the bar.
    #[must_use]
    pub fn summary(&self) -> String {
        let at = |s: Option<f64>, bar: f64| s.is_some_and(|v| v >= bar);
        let mut parts = Vec::new();
        if at(self.name_score, 0.90) {
            parts.push("name");
        }
        if at(self.start_date_score, 0.95) {
            parts.push("start");
        }
        if at(self.end_date_score, 0.95) {
            parts.push("end");
        }
        if at(self.location_score, 0.80) {
            parts.push("location");
        }
        if at(self.organizer_score, 0.90) {
            parts.push("organizer");
        }
        if at(self.performers_score, 0.90) {
            parts.push("performers");
        }
        if at(self.event_ids_score, 0.95) {
            parts.push("identifier");
        }
        if parts.is_empty() {
            "no strong matches".into()
        } else {
            parts.join(", ")
        }
    }
}

/// Strategy trait. Object-safe so callers can hold an
/// `Arc<dyn EventMatcher>` in [`AppState`](crate::api::rest::state::AppState).
pub trait EventMatcher: Send + Sync {
    /// Score a single `candidate` against `event`, weighing the evidence's
    /// frequency-adjusted components with `frequencies`.
    ///
    /// # Errors
    ///
    /// Returns an error if scoring fails.
    fn match_events(
        &self,
        event: &Event,
        candidate: &Event,
        frequencies: &dyn ValueFrequencies,
    ) -> Result<MatchResult>;
    /// Score all `candidates`, keep the matches, sorted by descending
    /// score.
    ///
    /// # Errors
    ///
    /// Returns an error if scoring any candidate fails.
    fn find_matches(
        &self,
        event: &Event,
        candidates: &[Event],
        frequencies: &dyn ValueFrequencies,
    ) -> Result<Vec<MatchResult>>;
    /// Whether a score counts as a match for this strategy.
    fn is_match(&self, score: f64) -> bool;
}

/// The matcher-backed strategy: [`MatchingEngine`] with the service's
/// configured threshold.
pub struct ProbabilisticMatcher {
    /// The canonical engine.
    engine: MatchingEngine,
    /// The configured match threshold (`MatchingConfig::threshold_score`).
    threshold: f64,
}

impl ProbabilisticMatcher {
    /// Construct from a [`MatchingConfig`]: the matcher's default
    /// configuration with `match_threshold = config.threshold_score`.
    /// `exact_match_score` and `fuzzy_match_score` belonged to the removed
    /// service scorer and are not read.
    #[must_use]
    // Taken by value, as every caller already passes an owned clone; the
    // engine copies the one field it needs.
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(config: MatchingConfig) -> Self {
        let threshold = config.threshold_score;
        Self {
            engine: MatchingEngine::new(MatchConfig {
                match_threshold: threshold,
                ..MatchConfig::default()
            }),
            threshold,
        }
    }

    /// The configured match threshold.
    #[must_use]
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Score one pair through the adapter and the matcher.
    fn score(
        &self,
        event: &Event,
        candidate: &Event,
        frequencies: &dyn ValueFrequencies,
    ) -> MatchResult {
        let r = self.engine.match_events_with_frequencies(
            &adapter::to_matcher_event(event),
            &adapter::to_matcher_event(candidate),
            frequencies,
        );
        MatchResult {
            event: candidate.clone(),
            score: r.score,
            is_match: r.is_match,
            confidence: r.confidence,
            breakdown: MatchScoreBreakdown::from(&r.breakdown),
            evidence: MatchEvidence::from(&r.evidence),
        }
    }
}

impl EventMatcher for ProbabilisticMatcher {
    fn match_events(
        &self,
        event: &Event,
        candidate: &Event,
        frequencies: &dyn ValueFrequencies,
    ) -> Result<MatchResult> {
        Ok(self.score(event, candidate, frequencies))
    }

    /// Scores every candidate, keeps the matcher's matches, and sorts
    /// them by descending score.
    fn find_matches(
        &self,
        event: &Event,
        candidates: &[Event],
        frequencies: &dyn ValueFrequencies,
    ) -> Result<Vec<MatchResult>> {
        let mut out: Vec<MatchResult> = candidates
            .iter()
            .map(|c| self.score(event, c, frequencies))
            .filter(|r| r.is_match)
            .collect();
        // Best matches first; NaN scores are treated as equal so the
        // sort never panics.
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(out)
    }

    fn is_match(&self, score: f64) -> bool {
        score >= self.threshold
    }
}

#[cfg(test)]
mod tests {
    use super::evidence::EvidenceStrength;
    use super::*;
    use crate::models::{Event, Identifier, IdentifierType, Location, Party, PartyKind};
    use ::event_matcher::NoFrequencies;
    use chrono::{TimeZone, Utc};

    /// A baseline matching config used by these tests.
    fn config() -> MatchingConfig {
        MatchingConfig {
            threshold_score: 0.85,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        }
    }

    fn when() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap()
    }

    /// An event at a fixed time, venue and organiser.
    fn full(name: &str) -> Event {
        let mut e = Event::new(name, when());
        e.end_date = Some(when() + chrono::Duration::hours(8));
        e.location.push(Location::Text {
            value: "Royal Infirmary, Lecture Theatre 3".into(),
        });
        e.organizers.push(Party {
            kind: PartyKind::Organization,
            id: None,
            name: "Northern Cardiology Society".into(),
            email: None,
            url: None,
        });
        e
    }

    fn ticket(system: &str, value: &str) -> Identifier {
        Identifier::new(IdentifierType::TicketNumber, system.into(), value.into())
    }

    /// `find_matches` keeps only matches and returns them sorted by
    /// descending score.
    #[test]
    fn find_matches_returns_only_matches_sorted() {
        let m = ProbabilisticMatcher::new(MatchingConfig {
            threshold_score: 0.20,
            ..config()
        });
        let query = Event::new("Conference", when());
        let candidates = vec![
            Event::new("Conference", when()),
            Event::new("Conference", when() + chrono::Duration::minutes(30)),
            Event::new(
                "Totally Different Event",
                when() + chrono::Duration::hours(24 * 40),
            ),
        ];
        let matches = m.find_matches(&query, &candidates, &NoFrequencies).unwrap();
        assert!(!matches.is_empty());
        assert!(matches.iter().all(|r| r.is_match));
        for w in matches.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    /// `summary` names the components that cleared their display bar
    /// and ignores the ones not compared.
    #[test]
    fn breakdown_summary() {
        let b = MatchScoreBreakdown {
            name_score: Some(0.95),
            start_date_score: Some(0.99),
            end_date_score: Some(0.50),
            location_score: Some(0.85),
            organizer_score: Some(0.95),
            ..MatchScoreBreakdown::default()
        };
        let s = b.summary();
        assert!(s.contains("name"));
        assert!(s.contains("start"));
        assert!(s.contains("location"));
        assert!(s.contains("organizer"));
        assert!(!s.contains("performers"));
    }

    /// T-16: two identical events with no identifier are a match. The old
    /// service scorer counted every absent component as `0` and gave
    /// them `0.75`, under the `0.85` threshold.
    #[test]
    fn identical_events_without_an_identifier_match() {
        let m = ProbabilisticMatcher::new(config());
        let r = m
            .match_events(
                &full("Annual Cardiology Day"),
                &full("Annual Cardiology Day"),
                &NoFrequencies,
            )
            .unwrap();
        assert!(r.score > 0.99, "score {}", r.score);
        assert!(r.is_match);
        assert_eq!(r.quality(), "certain");
        // Components neither event holds are not compared.
        assert_eq!(r.breakdown.performers_score, None);
        assert_eq!(r.breakdown.event_ids_score, None);
    }

    /// Name and start alone are a match on score, but only consistent:
    /// nothing proves the pair is one event.
    #[test]
    fn name_and_start_alone_are_consistent_not_proven() {
        let m = ProbabilisticMatcher::new(config());
        let a = Event::new("Annual Cardiology Day", when());
        let r = m.match_events(&a, &a.clone(), &NoFrequencies).unwrap();
        assert!(r.is_match);
        assert_eq!(r.evidence.strength, EvidenceStrength::Consistent);
    }

    /// A shared ticket number is proof.
    #[test]
    fn shared_ticket_is_proven() {
        let m = ProbabilisticMatcher::new(config());
        let mut a = full("Annual Cardiology Day");
        let mut b = full("Annual Cardiology Day");
        a.identifiers
            .push(ticket("https://tickets.example.org", "T-1"));
        b.identifiers
            .push(ticket("https://tickets.example.org", "T-1"));
        let r = m.match_events(&a, &b, &NoFrequencies).unwrap();
        assert!(r.is_match);
        assert_eq!(r.evidence.strength, EvidenceStrength::Proven);
        assert!(
            r.evidence
                .components
                .iter()
                .any(|c| c.component == "identifier")
        );
    }

    /// Two ticket numbers from one system contradict; from two systems
    /// they are two namespaces and do not.
    #[test]
    fn conflicting_tickets_contradict_only_within_one_system() {
        let m = ProbabilisticMatcher::new(config());
        let mut a = full("Annual Cardiology Day");
        let mut b = full("Annual Cardiology Day");
        a.identifiers
            .push(ticket("https://tickets.example.org", "T-1"));
        b.identifiers
            .push(ticket("https://tickets.example.org", "T-2"));
        let r = m.match_events(&a, &b, &NoFrequencies).unwrap();
        assert_eq!(r.evidence.strength, EvidenceStrength::Contradicted);
        assert_eq!(
            r.evidence.contradictions[0].field,
            "identifier:other:ticketnumber@https://tickets.example.org"
        );

        b.identifiers[0].system = "https://other.example.org".into();
        let r = m.match_events(&a, &b, &NoFrequencies).unwrap();
        assert!(r.evidence.contradictions.is_empty());
    }

    /// Evidence carries component names and bits, never a value the
    /// masked view hides.
    #[test]
    fn evidence_carries_no_identifier_value_or_party() {
        let m = ProbabilisticMatcher::new(config());
        let mut a = full("Annual Cardiology Day");
        let mut b = full("Annual Cardiology Day");
        a.identifiers
            .push(ticket("https://tickets.example.org", "SECRET-111"));
        b.identifiers
            .push(ticket("https://tickets.example.org", "SECRET-222"));
        let r = m.match_events(&a, &b, &NoFrequencies).unwrap();
        let json = serde_json::to_string(&r.evidence).unwrap();
        assert!(!json.contains("SECRET"), "{json}");
        assert!(!json.contains("Northern Cardiology"), "{json}");
        assert!(!json.contains("Annual Cardiology"), "{json}");
    }
}
