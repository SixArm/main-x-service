//! Match evidence on the wire (`agents/share/match-evidence.md`).
//!
//! The `worker-matcher` crate returns an [`Evidence`](matcher::Evidence)
//! beside every score: Fellegi–Sunter bits, contradictions, and a
//! [`MatchStrength`] verdict. These are the service's wire mirrors of
//! those types, so they carry an OpenAPI schema (the matcher crate has no
//! `utoipa` dependency, and should not grow one for this). They serialize
//! to exactly the matcher's JSON, so a stored value reads back as either.
//!
//! **No field values.** Evidence names components (`"family_name"`,
//! `"identifier"`, `"contradiction:identifier:us_ssn"`) and carries bits;
//! it never carries what a record holds. A contradiction names the
//! *scheme* both records hold under, which the masked view already shows
//! (masking hides identifier values, not their systems), so evidence is
//! safe on the masked and ABAC paths.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use worker_matcher as matcher;

/// The verdict the evidence supports (`match-evidence.md` §4).
///
/// Only `proven` permits an automatic merge; `consistent` and
/// `contradicted` pairs go to review whatever their score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MatchStrength {
    /// A contradiction (two different values under one unique identifier
    /// scheme) rules the pair out of any automatic merge.
    Contradicted,
    /// The evidence clears the matcher's `proof_bits`, with no
    /// contradiction.
    Proven,
    /// A match on score, with no contradiction, but short of proof: the
    /// data agrees, it does not prove the pair is one worker.
    Consistent,
    /// Not a match.
    #[default]
    NonMatch,
}

impl From<matcher::MatchStrength> for MatchStrength {
    fn from(s: matcher::MatchStrength) -> Self {
        match s {
            matcher::MatchStrength::Contradicted => Self::Contradicted,
            matcher::MatchStrength::Proven => Self::Proven,
            matcher::MatchStrength::Consistent => Self::Consistent,
            matcher::MatchStrength::NonMatch => Self::NonMatch,
        }
    }
}

/// One hard conflict between the two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Contradiction {
    /// What conflicts, e.g. `"identifier:uk_nhs_number"`. A scheme name,
    /// never a value.
    pub field: String,
}

/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ComponentBits {
    /// The component, e.g. `"family_name"`, `"identifier"`,
    /// `"contradiction:identifier:uk_nhs_number"`.
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
    /// Sum of the components' log-likelihood weights, in bits. Each bit
    /// doubles the odds that the pair is one worker.
    pub weight_bits: f64,
    /// Components both records held (identifiers count as one).
    pub fields_compared: u32,
    /// Hard conflicts; any one makes the strength `contradicted`.
    pub contradictions: Vec<Contradiction>,
    /// The verdict.
    pub strength: MatchStrength,
    /// Every term of the sum; the terms add up to `weight_bits`.
    #[serde(default)]
    pub components: Vec<ComponentBits>,
}

impl MatchEvidence {
    /// Whether this evidence permits an automatic merge: only `proven`
    /// does (`match-evidence.md` §4).
    #[must_use]
    pub fn permits_auto_merge(&self) -> bool {
        self.strength == MatchStrength::Proven
    }

    /// The aggregate only: `weight_bits`, `fields_compared` and
    /// `strength`, with `contradictions` and `components` emptied (SEC-G3).
    ///
    /// For a candidate the caller may read only masked. The terms carry
    /// component names, never values, but a name still says what the
    /// masked view hides, e.g. a contradiction under an identifier scheme
    /// says the hidden record holds one. The aggregate discloses no more
    /// than the score. The case service's `redact_evidence` is the
    /// reference.
    #[must_use]
    pub fn aggregate_only(self) -> Self {
        Self {
            contradictions: Vec::new(),
            components: Vec::new(),
            ..self
        }
    }
}

impl From<&matcher::Evidence> for MatchEvidence {
    fn from(e: &matcher::Evidence) -> Self {
        Self {
            weight_bits: e.weight_bits,
            fields_compared: e.fields_compared,
            contradictions: e
                .contradictions
                .iter()
                .map(|c| Contradiction {
                    field: c.field.clone(),
                })
                .collect(),
            strength: e.strength.into(),
            components: e
                .components
                .iter()
                .map(|c| ComponentBits {
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

    /// The mirror serializes to the matcher's own JSON, so a stored value
    /// reads back as either type.
    #[test]
    fn mirror_serializes_as_the_matcher_does() {
        let w = matcher::Worker::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build();
        let result = matcher::MatchingEngine::default_config().match_workers(&w, &w.clone());
        let ours = serde_json::to_value(MatchEvidence::from(&result.evidence)).unwrap();
        let theirs = serde_json::to_value(&result.evidence).unwrap();
        assert_eq!(ours, theirs);
        let back: matcher::Evidence = serde_json::from_value(ours).unwrap();
        assert_eq!(back, result.evidence);
    }

    /// Only `proven` permits an automatic merge.
    #[test]
    fn only_proven_permits_auto_merge() {
        for (strength, permits) in [
            (MatchStrength::Proven, true),
            (MatchStrength::Consistent, false),
            (MatchStrength::Contradicted, false),
            (MatchStrength::NonMatch, false),
        ] {
            let e = MatchEvidence {
                strength,
                ..MatchEvidence::default()
            };
            assert_eq!(e.permits_auto_merge(), permits, "{strength:?}");
        }
    }

    /// Evidence names components and carries bits; no value a record
    /// holds appears in it.
    #[test]
    fn evidence_carries_no_field_values() {
        let a = matcher::Worker::builder()
            .given_name("Zbigniewa")
            .family_name("Qwertyuiop")
            .email("secret.address@example.org")
            .phone("+44 7700 900123")
            .us_ssn("123-45-6789")
            .build();
        let mut b = a.clone();
        b.us_ssn = Some("987-65-4321".into());
        let result = matcher::MatchingEngine::default_config().match_workers(&a, &b);
        let json = serde_json::to_string(&MatchEvidence::from(&result.evidence)).unwrap();
        for value in [
            "Zbigniewa",
            "zbigniewa",
            "Qwertyuiop",
            "qwertyuiop",
            "secret",
            "7700",
            "6789",
            "4321",
        ] {
            assert!(!json.contains(value), "{value} leaked into {json}");
        }
    }
}
