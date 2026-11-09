//! Match evidence: Fellegi–Sunter log-likelihood weights beside the
//! similarity score.
//!
//! The similarity `score` answers "how alike is the data both records
//! hold?". It cannot tell "nothing here contradicts a match" from "the
//! data proves a match": two sparse records holding only the same title
//! score `1.0`, and so would two different appeals filed under one
//! template title. [`Evidence`] answers the second question, in bits: each
//! bit doubles the odds that the pair is one case. A missing component
//! contributes exactly `0`. The family-wide contract is
//! `agents/share/match-evidence.md`; this file is this crate's copy of it
//! plus its own `m`/`u` table.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::normalize;
use crate::scoring::MatchBreakdown;

/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;
/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// Bits one shared globally-unique identifier contributes, once per pair:
/// `log2(0.99 / 1e-12)` ≈ 39.85. Clears the default `proof_bits` alone.
pub const IDENTIFIER_BITS: f64 = 39.848_637_568_953_24;

/// Bits each contradicted identifier scheme contributes:
/// `log2(0.01 / (1 − 1e-12))` ≈ −6.64.
pub const CONTRADICTION_BITS: f64 = -6.643_856_189_773_282;

/// Default evidence bar for [`MatchStrength::Proven`]: `log2(N) +
/// log2(1/α)` for a million records at one false merge in a thousand.
pub const DEFAULT_PROOF_BITS: f64 = 30.0;

/// Per-component `(m, u)`: `m` is the chance the component agrees when the
/// records are one case, `u` the chance it agrees by coincidence when they
/// are different cases. Informed estimates, not measurements
/// (`agents/share/match-evidence.md` §5).
///
/// Title (Jaro-Winkler). `u`: agencies reuse template titles ("Housing
/// benefit appeal — …"), so a near-identical title for an unrelated case
/// is roughly one in two hundred, not one in a million.
pub const TITLE_M_U: (f64, f64) = (0.95, 0.005);
/// Subjects (Jaccard over involved parties). `u`: a shared party list
/// between unrelated cases is about one in a hundred (the same claimant
/// files several matters); `m` below `0.9` because parties are added and
/// removed over a case's life.
pub const SUBJECTS_M_U: (f64, f64) = (0.85, 0.01);
/// Same-agency case number. `u`: one agency's numbering collides with
/// itself about one time in a thousand (re-keyed or recycled numbers);
/// `m` allows for transcription errors.
pub const CASE_NUMBER_M_U: (f64, f64) = (0.95, 0.001);
/// Case type (categorical). `u`: about a dozen types, unevenly used, so
/// two unrelated cases share one about 15% of the time.
pub const CASE_TYPE_M_U: (f64, f64) = (0.95, 0.15);
/// Status (categorical). `u`: a handful of statuses, most cases open or
/// closed, so about one in four; `m` low because status changes over time.
pub const STATUS_M_U: (f64, f64) = (0.80, 0.25);
/// Keywords (Jaccard). `u`: agencies tag from a small vocabulary, so
/// overlapping tag sets are common (about 5%); `m` low because tags drift.
/// Set overlap corroborates; it does not identify.
pub const KEYWORDS_M_U: (f64, f64) = (0.80, 0.05);

/// How strong the evidence for a match is (contract §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStrength {
    /// A contradiction rules the pair out of any automatic merge.
    Contradicted,
    /// The evidence clears `proof_bits`, with no contradiction.
    Proven,
    /// `is_match`, no contradiction, but the evidence is short of proof.
    Consistent,
    /// Not a match.
    #[default]
    NonMatch,
}

/// One hard conflict between the two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contradiction {
    /// What conflicts, e.g. `"identifier:docket"`.
    pub field: String,
}

/// How much the compared data proves, beside the similarity score.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Evidence {
    /// Sum of the components' log-likelihood weights, in bits.
    pub weight_bits: f64,
    /// Components both records held (identifiers count as one).
    pub fields_compared: u32,
    /// Hard conflicts; any one makes the strength `Contradicted`.
    pub contradictions: Vec<Contradiction>,
    /// The verdict (§4).
    pub strength: MatchStrength,
    /// Every term of the sum, in a fixed order; they add up to
    /// `weight_bits` exactly (contract §8). Absent from older JSON ⇒ empty.
    #[serde(default)]
    pub components: Vec<ComponentBits>,
}

/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentBits {
    /// The component, e.g. `"title"`, `"identifier"`,
    /// `"contradiction:identifier:docket"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}

// ─── Correlated components (contract §6) ─────────────────────────────

/// Weight of each member of a correlated group after its strongest.
pub const CORRELATED_DISCOUNT: f64 = 0.5;

/// This crate's correlated groups, by component name. Public contract
/// (spec §18a). The groups are disjoint, so each component is discounted
/// at most once.
///
/// - `title`, `subjects`, `keywords`: agencies generate titles from the
///   matter and the party ("Housing benefit appeal — J. Smith"), and tag
///   keywords from the same matter, so one claimant's related cases agree
///   on all three for one reason.
/// - `case_type`, `status`: a case type sets the workflow, and with it the
///   statuses a case of that type passes through.
pub const CORRELATED_GROUPS: &[&[&str]] =
    &[&["title", "subjects", "keywords"], &["case_type", "status"]];

/// Index of the strongest member: largest absolute value, ties to the
/// larger value. `None` for an empty slice.
fn strongest_index(member_bits: &[f64]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, &b) in member_bits.iter().enumerate() {
        best = match best {
            None => Some(i),
            Some(j) => {
                let c = member_bits[j];
                let stronger = b.abs().total_cmp(&c.abs()).then(b.total_cmp(&c)).is_gt();
                Some(if stronger { i } else { j })
            }
        };
    }
    best
}

/// The strongest member (largest absolute value, ties to the larger
/// value) and the rest, so a group's total does not depend on order.
#[must_use]
pub fn strongest_first(member_bits: &[f64]) -> Option<(f64, Vec<f64>)> {
    let i = strongest_index(member_bits)?;
    let rest = member_bits
        .iter()
        .enumerate()
        .filter(|(j, _)| *j != i)
        .map(|(_, &b)| b)
        .collect();
    Some((member_bits[i], rest))
}

/// The bits a correlated group adds, from its present members' bits.
#[must_use]
pub fn group_bits(member_bits: &[f64]) -> f64 {
    let Some((strongest, rest)) = strongest_first(member_bits) else {
        return 0.0;
    };
    strongest + CORRELATED_DISCOUNT * rest.iter().sum::<f64>()
}

// ─── Common and rare values (contract §7) ────────────────────────────

/// Where the share of records holding a value comes from.
pub trait ValueFrequencies {
    /// The share, in `0.0..=1.0`, of records whose `component` holds
    /// `value` (already in the matcher's normalised form), or `None`
    /// when unknown.
    fn share(&self, component: &str, value: &str) -> Option<f64>;
}

/// No frequencies: every share is unknown, so the §2 table stands.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFrequencies;

impl ValueFrequencies for NoFrequencies {
    fn share(&self, _component: &str, _value: &str) -> Option<f64> {
        None
    }
}

/// A frequency source built from counts. Values must already be keys
/// from [`frequency_key`].
#[derive(Debug, Clone, Default)]
pub struct FrequencyTable {
    /// component → normalised value → share.
    shares: HashMap<String, HashMap<String, f64>>,
}

impl FrequencyTable {
    /// Build from `(component, key, count)` triples over `total` records.
    /// A `total` of zero gives an empty table.
    #[must_use]
    pub fn from_counts(counts: &[(&str, &str, u64)], total: u64) -> Self {
        let mut table = Self::default();
        if total == 0 {
            return table;
        }
        for &(component, key, count) in counts {
            #[allow(clippy::cast_precision_loss)] // shares need no exactness
            let share = count as f64 / total as f64;
            table.insert(component, key, share);
        }
        table
    }

    /// Set the share of records whose `component` holds `key`.
    pub fn insert(&mut self, component: &str, key: &str, share: f64) {
        self.shares
            .entry(component.to_string())
            .or_default()
            .insert(key.to_string(), share);
    }
}

impl ValueFrequencies for FrequencyTable {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        self.shares.get(component)?.get(value).copied()
    }
}

/// Smallest share used, so a value seen once cannot give unbounded bits.
pub const MIN_SHARE: f64 = 1e-9;

/// Agreement bits for a value held by `share` of records.
#[must_use]
pub fn frequency_bits(m: f64, share: f64) -> f64 {
    let share = if share.is_finite() {
        share.clamp(MIN_SHARE, m)
    } else {
        m
    };
    (m / share).log2()
}

/// The components whose agreement weight follows the value's frequency.
/// Public contract (spec §18a). Template titles ("Housing Benefit Claim")
/// are very common, so agreeing on one proves little.
pub const FREQUENCY_COMPONENTS: &[&str] = &["title"];

/// The key the matcher looks up for `raw` under `component`: the same
/// normalised form it compares (`normalize::fold` for `title`, the
/// primary title only). `None` for a component that does not adjust, or
/// a value that normalises to empty.
#[must_use]
pub fn frequency_key(component: &str, raw: &str) -> Option<String> {
    match component {
        "title" => Some(normalize::fold(raw)).filter(|k| !k.is_empty()),
        _ => None,
    }
}

/// The weight, in bits, one compared component adds.
///
/// `similarity` at or above [`AGREE_AT`] adds `log2(m/u)`; at or below
/// [`DISAGREE_AT`] adds `log2((1−m)/(1−u))` (negative); in between it is
/// interpolated linearly. A non-finite similarity counts as `0.0`.
#[must_use]
pub fn component_bits(similarity: f64, m: f64, u: f64) -> f64 {
    let agree = (m / u).log2();
    let disagree = ((1.0 - m) / (1.0 - u)).log2();
    let s = if similarity.is_finite() {
        similarity.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if s >= AGREE_AT {
        agree
    } else if s <= DISAGREE_AT {
        disagree
    } else {
        disagree + (agree - disagree) * (s - DISAGREE_AT) / (AGREE_AT - DISAGREE_AT)
    }
}

/// Assemble the [`Evidence`] for one pair.
///
/// `breakdown` holds the component similarities (`None` adds nothing and
/// is not counted). `identifier_shared` is whether the deterministic
/// short-circuit fired; it adds [`IDENTIFIER_BITS`] once and counts as one
/// field. When it fired, an agreeing case number is the identifier itself
/// (rule R-1) and is not weighed a second time. `shares` maps a
/// [`FREQUENCY_COMPONENTS`] name to its value's share, given only when the
/// two records hold the same normalised value; it replaces `u` on full
/// agreement. Correlated groups ([`CORRELATED_GROUPS`]) are then
/// discounted, and `contradictions` each add [`CONTRADICTION_BITS`].
/// `is_match` and `proof_bits` decide the strength.
#[must_use]
pub fn assess(
    breakdown: &MatchBreakdown,
    identifier_shared: bool,
    contradictions: Vec<Contradiction>,
    shares: &[(&str, f64)],
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let case_number = if identifier_shared && breakdown.case_number_score == Some(1.0) {
        None
    } else {
        breakdown.case_number_score
    };
    let present = [
        ("title", breakdown.title_score, TITLE_M_U),
        ("subjects", breakdown.subjects_score, SUBJECTS_M_U),
        ("case_number", case_number, CASE_NUMBER_M_U),
        ("case_type", breakdown.case_type_score, CASE_TYPE_M_U),
        ("status", breakdown.status_score, STATUS_M_U),
        ("keywords", breakdown.keywords_score, KEYWORDS_M_U),
    ];
    let mut terms: Vec<ComponentBits> = Vec::new();
    for (name, similarity, (m, u)) in present {
        let Some(s) = similarity else { continue };
        let share = shares
            .iter()
            .find(|(c, _)| *c == name && FREQUENCY_COMPONENTS.contains(c))
            .map(|&(_, share)| share);
        // Only an exact agreement adjusts (contract §7).
        let (bits, frequency_adjusted) = match share {
            Some(share) if s.is_finite() && s >= AGREE_AT => (frequency_bits(m, share), true),
            _ => (component_bits(s, m, u), false),
        };
        terms.push(ComponentBits {
            component: name.to_string(),
            bits,
            correlated: false,
            frequency_adjusted,
        });
    }
    let fields_compared = u32::try_from(terms.len()).unwrap_or(u32::MAX);

    // Discount every non-strongest present member of each group.
    for group in CORRELATED_GROUPS {
        let members: Vec<usize> = terms
            .iter()
            .enumerate()
            .filter(|(_, t)| group.contains(&t.component.as_str()))
            .map(|(i, _)| i)
            .collect();
        let bits: Vec<f64> = members.iter().map(|&i| terms[i].bits).collect();
        if let Some(strongest) = strongest_index(&bits) {
            for (k, &i) in members.iter().enumerate() {
                if k != strongest {
                    terms[i].bits *= CORRELATED_DISCOUNT;
                    terms[i].correlated = true;
                }
            }
        }
    }

    let mut fields_compared = fields_compared;
    if identifier_shared {
        terms.push(ComponentBits {
            component: "identifier".into(),
            bits: IDENTIFIER_BITS,
            correlated: false,
            frequency_adjusted: false,
        });
        fields_compared += 1;
    }
    for c in &contradictions {
        terms.push(ComponentBits {
            component: format!("contradiction:{}", c.field),
            bits: CONTRADICTION_BITS,
            correlated: false,
            frequency_adjusted: false,
        });
    }
    let weight_bits = terms.iter().map(|t| t.bits).sum::<f64>();
    let strength = if !contradictions.is_empty() {
        MatchStrength::Contradicted
    } else if weight_bits >= proof_bits {
        MatchStrength::Proven
    } else if is_match {
        MatchStrength::Consistent
    } else {
        MatchStrength::NonMatch
    };
    Evidence {
        weight_bits,
        fields_compared,
        contradictions,
        strength,
        components: terms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_their_formulas() {
        assert!((IDENTIFIER_BITS - (0.99_f64 / 1e-12).log2()).abs() < 1e-9);
        assert!((CONTRADICTION_BITS - (0.01_f64 / (1.0 - 1e-12)).log2()).abs() < 1e-9);
    }

    #[test]
    fn component_bits_agree_disagree_and_interpolate() {
        let (m, u) = TITLE_M_U;
        let agree = (m / u).log2();
        let disagree = ((1.0 - m) / (1.0 - u)).log2();
        assert!((component_bits(1.0, m, u) - agree).abs() < 1e-12);
        assert!((component_bits(0.0, m, u) - disagree).abs() < 1e-12);
        assert!(disagree < 0.0 && agree > 0.0);
        let mid = component_bits(0.775, m, u);
        assert!(mid > disagree && mid < agree);
        // Non-finite similarity is treated as full disagreement, not NaN.
        assert!((component_bits(f64::NAN, m, u) - disagree).abs() < 1e-12);
    }

    #[test]
    fn missing_component_adds_zero_and_is_not_counted() {
        let with_title = MatchBreakdown {
            title_score: Some(1.0),
            ..Default::default()
        };
        let with_status_missing = assess(&with_title, false, Vec::new(), &[], true, 30.0);
        assert_eq!(with_status_missing.fields_compared, 1);
        let (m, u) = TITLE_M_U;
        assert!((with_status_missing.weight_bits - (m / u).log2()).abs() < 1e-12);

        let empty = assess(
            &MatchBreakdown::default(),
            false,
            Vec::new(),
            &[],
            false,
            30.0,
        );
        assert_eq!(empty.fields_compared, 0);
        assert!(empty.weight_bits.abs() < 1e-12);
        assert_eq!(empty.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn contradiction_wins_over_proof() {
        let e = assess(
            &MatchBreakdown::default(),
            true,
            vec![Contradiction {
                field: "identifier:docket".into(),
            }],
            &[],
            true,
            30.0,
        );
        assert_eq!(e.strength, MatchStrength::Contradicted);
        assert!((e.weight_bits - (IDENTIFIER_BITS + CONTRADICTION_BITS)).abs() < 1e-9);
    }

    #[test]
    fn old_json_without_evidence_deserialises() {
        let json = r#"{"score":0.5,"is_match":false,"confidence":"Low",
            "breakdown":{"title_score":0.5,"subjects_score":null,
            "case_number_score":null,"case_type_score":null,
            "status_score":null,"keywords_score":null,
            "deterministic_match":false}}"#;
        let r: crate::MatchResult = serde_json::from_str(json).expect("old JSON parses");
        assert_eq!(r.evidence, Evidence::default());
        assert!(r.evidence.components.is_empty());
        assert_eq!(r.evidence.strength, MatchStrength::NonMatch);

        let cfg: crate::MatchConfig = serde_json::from_str(
            r#"{"threshold":0.85,"title_weight":0.3,"subjects_weight":0.25,
            "case_number_weight":0.15,"case_type_weight":0.1,
            "status_weight":0.05,"keywords_weight":0.15}"#,
        )
        .expect("old config parses");
        assert!((cfg.proof_bits - DEFAULT_PROOF_BITS).abs() < 1e-12);
    }

    fn bits_of(m_u: (f64, f64), s: f64) -> f64 {
        component_bits(s, m_u.0, m_u.1)
    }

    fn total(b: &MatchBreakdown) -> f64 {
        assess(b, false, Vec::new(), &[], true, 30.0).weight_bits
    }

    // Two agreeing correlated fields give less than their sum, and the
    // group total does not depend on member order.
    #[test]
    fn correlated_agreements_are_discounted_and_order_free() {
        let b = MatchBreakdown {
            case_type_score: Some(1.0),
            status_score: Some(1.0),
            ..Default::default()
        };
        let t = bits_of(CASE_TYPE_M_U, 1.0);
        let s = bits_of(STATUS_M_U, 1.0);
        let got = total(&b);
        assert!(got < t + s);
        assert!((got - (t + 0.5 * s)).abs() < 1e-12);
        assert!((group_bits(&[t, s]) - group_bits(&[s, t])).abs() < 1e-12);
        assert!((group_bits(&[t, s]) - got).abs() < 1e-12);
        let e = assess(&b, false, Vec::new(), &[], true, 30.0);
        assert!(!e.components[0].correlated && e.components[1].correlated);
    }

    // Ties break to the larger value; empty groups add nothing.
    #[test]
    fn strongest_first_ties_and_empty() {
        let (s, rest) = strongest_first(&[-2.0, 2.0]).expect("non-empty");
        assert!((s - 2.0).abs() < 1e-12 && (rest[0] + 2.0).abs() < 1e-12);
        let (s, _) = strongest_first(&[2.0, -2.0]).expect("non-empty");
        assert!((s - 2.0).abs() < 1e-12);
        assert!(strongest_first(&[]).is_none());
        assert!(group_bits(&[]).abs() < 1e-12);
    }

    // Uncorrelated fields (title and case_type, in different groups)
    // still sum in full.
    #[test]
    fn uncorrelated_fields_still_sum() {
        let b = MatchBreakdown {
            title_score: Some(1.0),
            case_type_score: Some(1.0),
            ..Default::default()
        };
        let want = bits_of(TITLE_M_U, 1.0) + bits_of(CASE_TYPE_M_U, 1.0);
        assert!((total(&b) - want).abs() < 1e-12);
    }

    // Correlated disagreements are one piece of evidence against, not two.
    #[test]
    fn correlated_disagreements_are_discounted() {
        let b = MatchBreakdown {
            case_type_score: Some(0.0),
            status_score: Some(0.0),
            ..Default::default()
        };
        let t = bits_of(CASE_TYPE_M_U, 0.0); // −4.09, the strongest
        let s = bits_of(STATUS_M_U, 0.0); // −1.91
        let got = total(&b);
        assert!(got < 0.0 && got > t + s);
        assert!((got - (t + 0.5 * s)).abs() < 1e-12);
    }

    // A common title (share 0.01) gives fewer bits than the default u; a
    // rare one (1e-6) more; a share at or above m gives 0, never negative.
    #[test]
    fn frequency_bits_common_rare_and_ceiling() {
        let (m, u) = TITLE_M_U;
        let default = (m / u).log2();
        let common = frequency_bits(m, 0.01);
        let rare = frequency_bits(m, 1e-6);
        assert!(common < default, "common {common} vs {default}");
        assert!(rare > default, "rare {rare} vs {default}");
        for share in [m, 0.99, 1.0, f64::NAN] {
            let b = frequency_bits(m, share);
            assert!(b.abs() < 1e-12, "share {share} gave {b}");
        }
        assert!((frequency_bits(m, 0.0) - (m / MIN_SHARE).log2()).abs() < 1e-9);
    }

    // No frequency adjustment on partial agreement or on disagreement,
    // even when a share is supplied.
    #[test]
    fn no_frequency_adjustment_on_partial_or_disagreement() {
        for s in [0.8, 0.0] {
            let b = MatchBreakdown {
                title_score: Some(s),
                ..Default::default()
            };
            let e = assess(&b, false, Vec::new(), &[("title", 1e-6)], true, 30.0);
            assert!(!e.components[0].frequency_adjusted);
            assert!((e.weight_bits - bits_of(TITLE_M_U, s)).abs() < 1e-12);
        }
        // A share for a component not in FREQUENCY_COMPONENTS is ignored.
        let b = MatchBreakdown {
            status_score: Some(1.0),
            ..Default::default()
        };
        let e = assess(&b, false, Vec::new(), &[("status", 1e-6)], true, 30.0);
        assert!(!e.components[0].frequency_adjusted);
    }

    #[test]
    fn frequency_table_counts_and_lookup() {
        let t = FrequencyTable::from_counts(&[("title", "x", 5)], 100);
        assert!((t.share("title", "x").expect("share") - 0.05).abs() < 1e-12);
        assert!(t.share("title", "y").is_none());
        assert!(
            FrequencyTable::from_counts(&[("title", "x", 5)], 0)
                .share("title", "x")
                .is_none()
        );
        assert!(NoFrequencies.share("title", "x").is_none());
    }
}
