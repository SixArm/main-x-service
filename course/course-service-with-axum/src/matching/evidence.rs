//! Service-side mirror of `course_matcher::Evidence`, with an `OpenAPI`
//! schema (spec §6.1, `agents/share/match-evidence.md`).
//!
//! The wire shape is the matcher's, field for field, so JSON written by
//! either side reads on the other. Evidence holds component and scheme
//! names and bits only, never a field value, so it is safe to return on
//! every path a score is returned on, masked ones included.

use course_matcher as cm;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The verdict on a pair (match-evidence.md §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStrength {
    /// A contradiction rules the pair out of any automatic merge.
    Contradicted,
    /// The evidence clears the proof bar, with no contradiction. The
    /// only strength that permits an automatic merge.
    Proven,
    /// A match with no contradiction, but short of proof: review only.
    Consistent,
    /// Not a match.
    #[default]
    NonMatch,
}

impl From<cm::MatchStrength> for EvidenceStrength {
    fn from(s: cm::MatchStrength) -> Self {
        match s {
            cm::MatchStrength::Contradicted => Self::Contradicted,
            cm::MatchStrength::Proven => Self::Proven,
            cm::MatchStrength::Consistent => Self::Consistent,
            cm::MatchStrength::NonMatch => Self::NonMatch,
        }
    }
}

/// One hard conflict between the two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct EvidenceContradiction {
    /// What conflicts, e.g. `"identifier:doi"` (a scheme name, not a value).
    pub field: String,
}

/// One term of the evidence sum (match-evidence.md §8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EvidenceComponent {
    /// The component, e.g. `"name"`, `"identifier"`,
    /// `"contradiction:identifier:doi"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    #[serde(default)]
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    #[serde(default)]
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
    pub strength: EvidenceStrength,
    /// Every term of the sum; the terms add up to `weight_bits`.
    #[serde(default)]
    pub components: Vec<EvidenceComponent>,
}

impl MatchEvidence {
    /// `true` only for [`EvidenceStrength::Proven`]: the one strength
    /// that permits an automatic merge (match-evidence.md §4).
    #[must_use]
    pub fn permits_auto_merge(&self) -> bool {
        self.strength == EvidenceStrength::Proven
    }
}

impl From<&cm::Evidence> for MatchEvidence {
    fn from(e: &cm::Evidence) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The mirror serialises exactly as the matcher's own type does, so
    /// a stored row reads back either way.
    #[test]
    fn wire_shape_matches_the_matcher() {
        let a = cm::Course::new("Introduction to Biology");
        let r = cm::MatchingEngine::default_config().match_courses(&a, &a.clone());
        let ours = serde_json::to_value(MatchEvidence::from(&r.evidence)).unwrap();
        let theirs = serde_json::to_value(&r.evidence).unwrap();
        assert_eq!(ours, theirs);
        let back: cm::Evidence = serde_json::from_value(ours).unwrap();
        assert_eq!(back, r.evidence);
    }

    /// Only `proven` permits an automatic merge.
    #[test]
    fn only_proven_permits_auto_merge() {
        for (s, ok) in [
            (EvidenceStrength::Proven, true),
            (EvidenceStrength::Consistent, false),
            (EvidenceStrength::Contradicted, false),
            (EvidenceStrength::NonMatch, false),
        ] {
            let e = MatchEvidence {
                strength: s,
                ..MatchEvidence::default()
            };
            assert_eq!(e.permits_auto_merge(), ok, "{s:?}");
        }
    }

    /// Strength tokens are snake case on the wire.
    #[test]
    fn strength_tokens_are_snake_case() {
        assert_eq!(
            serde_json::to_value(EvidenceStrength::NonMatch).unwrap(),
            serde_json::json!("non_match")
        );
        assert_eq!(
            serde_json::to_value(EvidenceStrength::Consistent).unwrap(),
            serde_json::json!("consistent")
        );
    }
}
