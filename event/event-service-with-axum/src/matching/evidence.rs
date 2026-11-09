//! The service's wire form of the matcher's match evidence
//! (`agents/share/match-evidence.md`).
//!
//! The `event-matcher` crate returns an [`event_matcher::Evidence`] beside
//! every score: how much the compared data **proves** (a Fellegi–Sunter
//! weight in bits), any hard contradiction, and the verdict
//! ([`EvidenceStrength`]). These types mirror it field for field, in the
//! same JSON shape, so the API can document it in its `OpenAPI` schema
//! (the matcher's types carry no `utoipa` derive).
//!
//! **Safe on masked paths.** Evidence holds component names, bit weights
//! and flags only. The one string that is not a fixed component name is a
//! contradiction's `field`, `identifier:<scheme>`, where the scheme is a
//! matcher scheme name or, for the service's own identifier types, the
//! type name plus the identifier's `system` URI (see
//! [`super::adapter::map_identifier_scheme`]). The system is a namespace,
//! not a value, and the masked view returns it unmasked too; no
//! identifier value, name, date or party ever reaches evidence.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The evidence verdict for a pair (`match-evidence.md` §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStrength {
    /// A hard contradiction rules the pair out of any automatic merge.
    Contradicted,
    /// The evidence clears the proof bar with no contradiction: the only
    /// strength that permits an automatic merge.
    Proven,
    /// A match on score, with no contradiction, but short of proof:
    /// review, never auto-merge.
    Consistent,
    /// Not a match.
    #[default]
    NonMatch,
}

impl From<event_matcher::MatchStrength> for EvidenceStrength {
    fn from(s: event_matcher::MatchStrength) -> Self {
        match s {
            event_matcher::MatchStrength::Contradicted => Self::Contradicted,
            event_matcher::MatchStrength::Proven => Self::Proven,
            event_matcher::MatchStrength::Consistent => Self::Consistent,
            event_matcher::MatchStrength::NonMatch => Self::NonMatch,
        }
    }
}

/// One hard conflict between the two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EvidenceContradiction {
    /// What conflicts, e.g. `"identifier:eventbrite"`.
    pub field: String,
}

/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EvidenceComponent {
    /// The component, e.g. `"name"`, `"identifier"`,
    /// `"contradiction:identifier:eventbrite"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}

/// How much the compared data proves, beside the similarity score.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, ToSchema)]
pub struct MatchEvidence {
    /// Sum of the components' log-likelihood weights, in bits.
    pub weight_bits: f64,
    /// Components both records held (identifiers count as one).
    pub fields_compared: u32,
    /// Hard conflicts; any one makes the strength `contradicted`.
    #[serde(default)]
    pub contradictions: Vec<EvidenceContradiction>,
    /// The verdict.
    #[serde(default)]
    pub strength: EvidenceStrength,
    /// Every term of the sum; they add up to `weight_bits`.
    #[serde(default)]
    pub components: Vec<EvidenceComponent>,
}

impl MatchEvidence {
    /// Whether this evidence permits an automatic merge: `Proven` only.
    #[must_use]
    pub fn is_proven(&self) -> bool {
        self.strength == EvidenceStrength::Proven
    }
}

impl From<&event_matcher::Evidence> for MatchEvidence {
    fn from(e: &event_matcher::Evidence) -> Self {
        Self {
            weight_bits: e.weight_bits,
            fields_compared: e.fields_compared,
            contradictions: e
                .contradictions
                .iter()
                .map(|c| EvidenceContradiction {
                    field: c.field.clone(),
                })
                .collect(),
            strength: e.strength.into(),
            components: e
                .components
                .iter()
                .map(|c| EvidenceComponent {
                    component: c.component.clone(),
                    bits: c.bits,
                    correlated: c.correlated,
                    frequency_adjusted: c.frequency_adjusted,
                })
                .collect(),
        }
    }
}

/// Whether a pair may be marked or executed as an automatic merge: its
/// score clears `auto_merge_threshold` **and** its evidence is `Proven`
/// (`match-evidence.md` §4). `Consistent` and `Contradicted` pairs go to
/// review whatever their score.
#[must_use]
pub fn permits_auto_merge(score: f64, auto_merge_threshold: f64, evidence: &MatchEvidence) -> bool {
    score >= auto_merge_threshold && evidence.is_proven()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strength_serialises_snake_case() {
        assert_eq!(
            serde_json::to_value(EvidenceStrength::NonMatch).unwrap(),
            "non_match"
        );
        assert_eq!(
            serde_json::to_value(EvidenceStrength::Proven).unwrap(),
            "proven"
        );
    }

    #[test]
    fn same_json_shape_as_the_matcher() {
        let m = event_matcher::Evidence {
            weight_bits: 12.5,
            fields_compared: 3,
            contradictions: vec![event_matcher::Contradiction {
                field: "identifier:meetup".into(),
            }],
            strength: event_matcher::MatchStrength::Contradicted,
            components: vec![event_matcher::ComponentBits {
                component: "name".into(),
                bits: 12.5,
                correlated: false,
                frequency_adjusted: true,
            }],
        };
        let ours = serde_json::to_value(MatchEvidence::from(&m)).unwrap();
        let theirs = serde_json::to_value(&m).unwrap();
        assert_eq!(ours, theirs);
    }

    #[test]
    fn only_proven_above_threshold_permits_auto_merge() {
        let mut e = MatchEvidence {
            strength: EvidenceStrength::Proven,
            ..MatchEvidence::default()
        };
        assert!(permits_auto_merge(0.99, 0.95, &e));
        assert!(!permits_auto_merge(0.90, 0.95, &e));
        for s in [
            EvidenceStrength::Consistent,
            EvidenceStrength::Contradicted,
            EvidenceStrength::NonMatch,
        ] {
            e.strength = s;
            assert!(!permits_auto_merge(1.0, 0.95, &e), "{s:?}");
        }
    }
}
