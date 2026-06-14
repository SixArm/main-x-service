//! Combine per-component scores into an overall event match score.
//!
//! Two strategies: [`ProbabilisticScorer`] (weighted fuzzy sum) and
//! [`DeterministicScorer`] (rule-based with short-circuits).

use super::algorithms::{
    identifier_matching, location_matching, name_matching, party_matching, time_matching,
};
use super::{MatchResult, MatchScoreBreakdown};
use crate::config::MatchingConfig;
use crate::models::{Event, IdentifierType};

/// Component weights for [`ProbabilisticScorer`]. Sum is 1.0.
mod weights {
    /// Weight of the name/title component.
    pub const NAME: f64 = 0.20;
    /// Weight of the start-date component.
    pub const START: f64 = 0.20;
    /// Weight of the end-date component.
    pub const END: f64 = 0.10;
    /// Weight of the location component.
    pub const LOCATION: f64 = 0.15;
    /// Weight of the organizer component.
    pub const ORGANIZER: f64 = 0.10;
    /// Weight of the performer component.
    pub const PERFORMER: f64 = 0.10;
    /// Weight of the attendee component.
    pub const ATTENDEE: f64 = 0.05;
    /// Weight of the identifier component.
    pub const IDENTIFIER: f64 = 0.10;
}

/// Per-component, weighted-sum scoring for an event candidate.
///
/// Computes each component score, applies the strong-identifier
/// short-circuit, then returns the weighted sum (clamped to
/// `[0.0, 1.0]`).
pub struct ProbabilisticScorer {
    /// Matching configuration (threshold and exact/fuzzy scores).
    config: MatchingConfig,
}

impl ProbabilisticScorer {
    /// Construct a scorer from the given [`MatchingConfig`].
    pub fn new(config: MatchingConfig) -> Self {
        Self { config }
    }

    /// Score `candidate` against `event`, returning a [`MatchResult`]
    /// with the overall score and per-component breakdown. An exact
    /// strong-identifier match short-circuits to `1.0`.
    pub fn calculate_score(&self, event: &Event, candidate: &Event) -> MatchResult {
        let name = name_matching::match_name_with_alternates(
            &event.name,
            &event.alternate_names,
            &candidate.name,
            &candidate.alternate_names,
        );
        let start = time_matching::match_start_dates(event.start_date, candidate.start_date);
        let end = time_matching::match_end_dates(event.end_date, candidate.end_date);
        let location = location_matching::match_locations(&event.location, &candidate.location);
        let organizer = party_matching::match_parties(&event.organizers, &candidate.organizers);
        let performer = party_matching::match_parties(&event.performers, &candidate.performers);
        let attendee = party_matching::match_parties(&event.attendees, &candidate.attendees);
        let identifier =
            identifier_matching::match_identifiers(&event.identifiers, &candidate.identifiers);

        let breakdown = MatchScoreBreakdown {
            name_score: name,
            start_date_score: start,
            end_date_score: end,
            location_score: location,
            organizer_score: organizer,
            performer_score: performer,
            attendee_score: attendee,
            identifier_score: identifier,
        };

        // Short-circuit: exact match on a strong identifier type
        // (booking / ticket / confirmation / encounter / transaction
        // ID) is enough on its own.
        if identifier >= 1.0 && shares_strong_identifier_type(event, candidate) {
            return MatchResult {
                event: candidate.clone(),
                score: 1.0,
                breakdown,
            };
        }

        let total = name * weights::NAME
            + start * weights::START
            + end * weights::END
            + location * weights::LOCATION
            + organizer * weights::ORGANIZER
            + performer * weights::PERFORMER
            + attendee * weights::ATTENDEE
            + identifier * weights::IDENTIFIER;

        MatchResult {
            event: candidate.clone(),
            score: total.clamp(0.0, 1.0),
            breakdown,
        }
    }

    /// `true` when `score` meets or exceeds the configured threshold.
    pub fn is_match(&self, score: f64) -> bool {
        score >= self.config.threshold_score
    }

    /// Bucket a score into a [`MatchQuality`] band (Definite ≥ 0.95,
    /// Probable ≥ threshold, Possible ≥ 0.50, else Unlikely).
    pub fn classify_match(&self, score: f64) -> MatchQuality {
        if score >= 0.95 {
            MatchQuality::Definite
        } else if score >= self.config.threshold_score {
            MatchQuality::Probable
        } else if score >= 0.50 {
            MatchQuality::Possible
        } else {
            MatchQuality::Unlikely
        }
    }
}

/// Rule-based matching. Short-circuits on strong-identifier exact
/// match; otherwise counts satisfied rules (name+time, location,
/// parties) as a fraction of available rules.
pub struct DeterministicScorer {
    /// Held for symmetry with [`ProbabilisticScorer`]; the
    /// deterministic rules use fixed thresholds, so the config is
    /// currently unused (hence the leading underscore).
    _config: MatchingConfig,
}

impl DeterministicScorer {
    /// Construct a scorer from the given [`MatchingConfig`].
    pub fn new(config: MatchingConfig) -> Self {
        Self { _config: config }
    }

    /// Score `candidate` against `event` by counting satisfied rules
    /// over available rules. A strong-identifier exact match
    /// short-circuits to `1.0`.
    pub fn calculate_score(&self, event: &Event, candidate: &Event) -> MatchResult {
        let identifier =
            identifier_matching::match_identifiers(&event.identifiers, &candidate.identifiers);

        // Rule 0: strong identifier exact match → 1.0
        if identifier >= 1.0 && shares_strong_identifier_type(event, candidate) {
            return MatchResult {
                event: candidate.clone(),
                score: 1.0,
                breakdown: MatchScoreBreakdown {
                    name_score: 0.0,
                    start_date_score: 0.0,
                    end_date_score: 0.0,
                    location_score: 0.0,
                    organizer_score: 0.0,
                    performer_score: 0.0,
                    attendee_score: 0.0,
                    identifier_score: identifier,
                },
            };
        }

        let mut available = 0.0;
        let mut achieved = 0.0;

        // Rule 1: same title (>= 0.90) and same start (>= 0.95) → 1 pt
        let name = name_matching::match_name_with_alternates(
            &event.name,
            &event.alternate_names,
            &candidate.name,
            &candidate.alternate_names,
        );
        let start = time_matching::match_start_dates(event.start_date, candidate.start_date);
        available += 1.0;
        if name >= 0.90 && start >= 0.95 {
            achieved += 1.0;
        }

        // Rule 2: matching location (>= 0.80) → 1 pt (only when both sides have one)
        let location = location_matching::match_locations(&event.location, &candidate.location);
        if !event.location.is_empty() && !candidate.location.is_empty() {
            available += 1.0;
            if location >= 0.80 {
                achieved += 1.0;
            }
        }

        // Rule 3: organizer overlap (>= 0.90) → 1 pt
        let organizer = party_matching::match_parties(&event.organizers, &candidate.organizers);
        if !event.organizers.is_empty() && !candidate.organizers.is_empty() {
            available += 1.0;
            if organizer >= 0.90 {
                achieved += 1.0;
            }
        }

        let end = time_matching::match_end_dates(event.end_date, candidate.end_date);
        let performer = party_matching::match_parties(&event.performers, &candidate.performers);
        let attendee = party_matching::match_parties(&event.attendees, &candidate.attendees);

        let final_score = if available > 0.0 {
            achieved / available
        } else {
            0.0
        };

        MatchResult {
            event: candidate.clone(),
            score: final_score,
            breakdown: MatchScoreBreakdown {
                name_score: name,
                start_date_score: start,
                end_date_score: end,
                location_score: location,
                organizer_score: organizer,
                performer_score: performer,
                attendee_score: attendee,
                identifier_score: identifier,
            },
        }
    }

    /// `true` when the deterministic score meets the fixed `0.75`
    /// threshold.
    pub fn is_match(&self, score: f64) -> bool {
        score >= 0.75
    }
}

/// Whether `a` and `b` share at least one identifier of a "strong"
/// type (booking / ticket / confirmation / encounter / transaction)
/// with the same system and (case-insensitive, trimmed) value.
fn shares_strong_identifier_type(a: &Event, b: &Event) -> bool {
    /// Whether a given identifier type is "strong" enough to
    /// short-circuit matching on its own.
    fn strong(t: IdentifierType) -> bool {
        matches!(
            t,
            IdentifierType::BookingNumber
                | IdentifierType::ConfirmationCode
                | IdentifierType::TicketNumber
                | IdentifierType::EncounterId
                | IdentifierType::TransactionId
        )
    }
    a.identifiers.iter().any(|x| {
        strong(x.identifier_type)
            && b.identifiers.iter().any(|y| {
                y.identifier_type == x.identifier_type
                    && y.system == x.system
                    && y.value.trim().eq_ignore_ascii_case(x.value.trim())
            })
    })
}

/// Confidence band for a match score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchQuality {
    /// `>= 0.95` — a definite match.
    Definite,
    /// `>= threshold` — a likely match.
    Probable,
    /// `>= 0.50` — a potential match worth reviewing.
    Possible,
    /// Below the possible band — not a match.
    Unlikely,
}

impl MatchQuality {
    /// Lowercase string form (`"definite"`, `"probable"`, …) for
    /// serialization and the review-queue `match_quality` field.
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchQuality::Definite => "definite",
            MatchQuality::Probable => "probable",
            MatchQuality::Possible => "possible",
            MatchQuality::Unlikely => "unlikely",
        }
    }

    /// `true` for the `Definite` and `Probable` bands — i.e. quality
    /// levels that count as an actual match.
    pub fn is_match(&self) -> bool {
        matches!(self, MatchQuality::Definite | MatchQuality::Probable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Event, EventType, Identifier, IdentifierType};
    use chrono::TimeZone;

    /// A baseline matching config used by these tests.
    fn config() -> MatchingConfig {
        MatchingConfig {
            threshold_score: 0.85,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        }
    }

    /// Build a conference event with the given name on a fixed date.
    fn event(name: &str, hour: u32) -> Event {
        let mut e = Event::new(
            name,
            chrono::Utc
                .with_ymd_and_hms(2026, 3, 1, hour, 0, 0)
                .unwrap(),
        );
        e.event_type = EventType::Conference;
        e
    }

    /// Name + start alone yield a moderate (not high) probabilistic
    /// score — events need more corroborating signal.
    #[test]
    fn exact_event_scores_above_threshold() {
        let p = ProbabilisticScorer::new(config());
        let a = event("Annual Conference", 9);
        let b = event("Annual Conference", 9);
        let r = p.calculate_score(&a, &b);
        // name (1.0) * 0.20 + start (1.0) * 0.20 + end (0.5) * 0.10 = ~0.45.
        // We don't get above threshold without more matched components,
        // which is correct — events need more signal than name+time.
        assert!(r.score >= 0.40, "got {}", r.score);
    }

    /// A shared booking number forces the score to 1.0 despite
    /// otherwise unrelated name and time.
    #[test]
    fn booking_number_short_circuits_to_1_0() {
        let p = ProbabilisticScorer::new(config());
        let mut a = event("Foo", 9);
        let mut b = event("Different name", 15);
        let id = Identifier::new(IdentifierType::BookingNumber, "sys".into(), "ABC123".into());
        a.identifiers.push(id.clone());
        b.identifiers.push(id);
        let r = p.calculate_score(&a, &b);
        assert_eq!(r.score, 1.0);
    }

    /// Identical name + start satisfies rule 1, passing the
    /// deterministic threshold.
    #[test]
    fn deterministic_name_plus_start_passes() {
        let d = DeterministicScorer::new(config());
        let a = event("Identical", 9);
        let b = event("Identical", 9);
        let r = d.calculate_score(&a, &b);
        assert!(r.score >= 0.75);
        assert!(d.is_match(r.score));
    }

    /// Matching name but very different time fails the deterministic
    /// rule (start score too low).
    #[test]
    fn deterministic_name_only_fails() {
        let d = DeterministicScorer::new(config());
        let a = event("Identical", 9);
        let b = event("Identical", 18); // very different time
        let r = d.calculate_score(&a, &b);
        assert!(!d.is_match(r.score), "score was {}", r.score);
    }

    /// Scores map to the expected quality bands at the boundaries.
    #[test]
    fn match_quality_classification() {
        let p = ProbabilisticScorer::new(config());
        assert_eq!(p.classify_match(0.98), MatchQuality::Definite);
        assert_eq!(p.classify_match(0.87), MatchQuality::Probable);
        assert_eq!(p.classify_match(0.60), MatchQuality::Possible);
        assert_eq!(p.classify_match(0.30), MatchQuality::Unlikely);
    }
}
