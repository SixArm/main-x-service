//! Match evidence: Fellegi–Sunter log-likelihood weights beside the
//! similarity score.
//!
//! The similarity `score` answers "how alike is the data both records
//! hold?". [`Evidence`] answers a different question: "how much does that
//! data **prove**?". A missing component contributes exactly `0` bits, a
//! shared globally unique identifier contributes [`IDENTIFIER_BITS`], and a
//! conflicting one is a [`Contradiction`]. The family-wide contract is
//! `agents/share/match-evidence.md`; this crate's per-component `m`/`u`
//! table is below. Spec §18a.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::course::{Course, IdentifierScheme};
use crate::normalize;
use crate::scoring::MatchBreakdown;

/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;
/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// Bits a shared globally unique identifier contributes, once per pair:
/// `log2(0.99 / 1e-12)` ≈ 39.85.
pub const IDENTIFIER_BITS: f64 = 39.848_637_568_953_24;

/// Bits each contradicted identifier scheme contributes:
/// `log2(0.01 / (1 − 1e-12))` ≈ −6.64.
pub const CONTRADICTION_BITS: f64 = -6.643_856_189_773_282;

/// Default evidence bar for [`MatchStrength::Proven`], in bits.
pub const DEFAULT_PROOF_BITS: f64 = 30.0;

// ─── Per-component m/u table ────────────────────────────────────
//
// `m` = P(agree | same course); `u` = P(agree | different courses).
// Informed starting estimates, not measurements (match-evidence.md §5).
// The course-code component is NOT in this table: within one provider a
// course code is an identifier, so it is weighed as one (see `assess`).

/// Name: m 0.95, u 0.005. Unrelated courses rarely share a near-identical
/// title, but generic titles ("Introduction to Programming") recur across
/// providers, so `u` is well above a person's full-name rate.
pub const NAME_M: f64 = 0.95;
/// See [`NAME_M`].
pub const NAME_U: f64 = 0.005;
/// Provider: m 0.95, u 0.05. A registry holds many courses per provider,
/// so two unrelated courses share one roughly as often as the largest
/// providers' share of the catalogue.
pub const PROVIDER_M: f64 = 0.95;
/// See [`PROVIDER_M`].
pub const PROVIDER_U: f64 = 0.05;
/// Educational level: m 0.90, u 0.20. A handful of levels, unevenly
/// used; a course can also be re-graded between catalogues.
pub const EDUCATIONAL_LEVEL_M: f64 = 0.90;
/// See [`EDUCATIONAL_LEVEL_M`].
pub const EDUCATIONAL_LEVEL_U: f64 = 0.20;
/// Keywords: m 0.80, u 0.10. Set overlap corroborates; subject keywords
/// are shared by every course in a field.
pub const KEYWORDS_M: f64 = 0.80;
/// See [`KEYWORDS_M`].
pub const KEYWORDS_U: f64 = 0.10;
/// Teaches / competencies: m 0.80, u 0.05. More specific than keywords,
/// but still shared by courses covering the same syllabus.
pub const TEACHES_M: f64 = 0.80;
/// See [`TEACHES_M`].
pub const TEACHES_U: f64 = 0.05;
/// Relationships: m 0.70, u 0.05. Sibling courses in one programme point
/// at the same related courses; relationships also drift over time.
pub const RELATIONSHIPS_M: f64 = 0.70;
/// See [`RELATIONSHIPS_M`].
pub const RELATIONSHIPS_U: f64 = 0.05;
/// Tags: m 0.70, u 0.10. Operator-applied, coarse, and changeable.
pub const TAGS_M: f64 = 0.70;
/// See [`TAGS_M`].
pub const TAGS_U: f64 = 0.10;

/// The globally unique schemes, with the lowercase name used in a
/// contradiction's `field` (`identifier:<name>`). Same set as
/// [`IdentifierScheme::is_deterministic`].
const UNIQUE_SCHEMES: [(IdentifierScheme, &str); 6] = [
    (IdentifierScheme::Doi, "doi"),
    (IdentifierScheme::Wikidata, "wikidata"),
    (IdentifierScheme::Oer, "oer"),
    (IdentifierScheme::Lom, "lom"),
    (IdentifierScheme::Uri, "uri"),
    (IdentifierScheme::Uuid, "uuid"),
];

/// The weight, in bits, one compared component adds.
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

/// The verdict on a pair (match-evidence.md §4).
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
    /// What conflicts, e.g. `"identifier:doi"`.
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
}

/// Folded, non-empty values `c` holds under `scheme`.
fn scheme_values(c: &Course, scheme: &IdentifierScheme) -> HashSet<String> {
    c.identifiers
        .iter()
        .filter(|i| &i.scheme == scheme)
        .map(|i| normalize::fold(&i.value))
        .filter(|v| !v.is_empty())
        .collect()
}

/// Folded, non-empty `same_as` URLs.
fn same_as_values(c: &Course) -> HashSet<String> {
    c.same_as
        .iter()
        .map(|u| normalize::fold(u))
        .filter(|v| !v.is_empty())
        .collect()
}

/// The normalised course codes of both records when they share a
/// non-empty provider and both codes normalise to non-empty values.
fn provider_scoped_codes(a: &Course, b: &Course) -> Option<(String, String)> {
    let ap = a.provider_id.as_deref()?;
    let bp = b.provider_id.as_deref()?;
    if ap.is_empty() || ap != bp {
        return None;
    }
    let ac = normalize::course_code(a.course_code.as_deref()?);
    let bc = normalize::course_code(b.course_code.as_deref()?);
    if ac.is_empty() || bc.is_empty() {
        return None;
    }
    Some((ac, bc))
}

/// Compute the evidence for a pair.
///
/// `components` is the probabilistic breakdown, computed even when the
/// deterministic short-circuit fired. Its `course_code_score` is not
/// weighed as a component: the provider-scoped course code is weighed as
/// an identifier instead, so it is not counted twice.
#[must_use]
pub(crate) fn assess(
    a: &Course,
    b: &Course,
    components: &MatchBreakdown,
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let mut weight_bits = 0.0_f64;
    let mut fields_compared = 0_u32;
    let table = [
        (components.name_score, NAME_M, NAME_U),
        (components.provider_score, PROVIDER_M, PROVIDER_U),
        (
            components.educational_level_score,
            EDUCATIONAL_LEVEL_M,
            EDUCATIONAL_LEVEL_U,
        ),
        (components.keywords_score, KEYWORDS_M, KEYWORDS_U),
        (components.teaches_score, TEACHES_M, TEACHES_U),
        (
            components.relationships_score,
            RELATIONSHIPS_M,
            RELATIONSHIPS_U,
        ),
        (components.tags_score, TAGS_M, TAGS_U),
    ];
    for (similarity, m, u) in table {
        if let Some(s) = similarity {
            weight_bits += component_bits(s, m, u);
            fields_compared += 1;
        }
    }

    // Identifiers: one hit is worth IDENTIFIER_BITS once; each disjoint
    // scheme is a contradiction.
    let mut identifier_hit = false;
    let mut identifiers_compared = false;
    let mut contradictions = Vec::new();
    for (scheme, label) in &UNIQUE_SCHEMES {
        let av = scheme_values(a, scheme);
        let bv = scheme_values(b, scheme);
        if av.is_empty() || bv.is_empty() {
            continue;
        }
        identifiers_compared = true;
        if av.is_disjoint(&bv) {
            contradictions.push(Contradiction {
                field: format!("identifier:{label}"),
            });
        } else {
            identifier_hit = true;
        }
    }
    // A shared `same_as` URL is an asserted identity. Disjoint `same_as`
    // sets are not a contradiction: one record may list a Wikidata page
    // and the other an OER entry for the same course.
    let a_same = same_as_values(a);
    let b_same = same_as_values(b);
    if !a_same.is_disjoint(&b_same) {
        identifiers_compared = true;
        identifier_hit = true;
    }
    // Within one provider a course code is an identifier.
    if let Some((ac, bc)) = provider_scoped_codes(a, b) {
        identifiers_compared = true;
        if ac == bc {
            identifier_hit = true;
        } else {
            contradictions.push(Contradiction {
                field: "identifier:course_code".to_string(),
            });
        }
    }
    if identifier_hit {
        weight_bits += IDENTIFIER_BITS;
    }
    for _ in &contradictions {
        weight_bits += CONTRADICTION_BITS;
    }
    if identifiers_compared {
        fields_compared += 1;
    }

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
    }
}

/// Unit tests for the evidence weights and the strength verdict.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::{CourseIdentifier, EducationalLevel, RelationKind, RelationshipRef};
    use crate::{MatchConfig, MatchResult, MatchingEngine};

    fn ident(scheme: IdentifierScheme, value: &str) -> CourseIdentifier {
        CourseIdentifier {
            scheme,
            value: value.into(),
        }
    }

    /// A course carrying every probabilistic field, but no identifier.
    fn rich(name: &str) -> Course {
        let mut c = Course::new(name);
        c.provider_id = Some("prov-1".into());
        c.educational_level = Some(EducationalLevel::Undergraduate);
        c.keywords = vec!["algorithms".into(), "programming".into()];
        c.teaches = vec!["recursion".into(), "sorting".into()];
        c.tags = vec!["core".into()];
        c.relationships = vec![RelationshipRef {
            relation: RelationKind::SimilarTo,
            course_id: "course-9".into(),
        }];
        c
    }

    #[test]
    fn identifier_and_contradiction_constants_match_their_formulas() {
        assert!((IDENTIFIER_BITS - (0.99_f64 / 1e-12).log2()).abs() < 1e-9);
        assert!((CONTRADICTION_BITS - (0.01_f64 / (1.0 - 1e-12)).log2()).abs() < 1e-9);
    }

    #[test]
    fn component_bits_interpolates_between_disagree_and_agree() {
        let agree = component_bits(1.0, NAME_M, NAME_U);
        let disagree = component_bits(0.0, NAME_M, NAME_U);
        assert!(agree > 0.0 && disagree < 0.0);
        let mid = component_bits(0.775, NAME_M, NAME_U);
        assert!((mid - f64::midpoint(agree, disagree)).abs() < 1e-9);
        // Non-finite similarity counts as full disagreement.
        assert!((component_bits(f64::NAN, NAME_M, NAME_U) - disagree).abs() < 1e-12);
    }

    // A component either record lacks adds 0 and is not counted.
    #[test]
    fn missing_component_adds_zero_and_is_not_counted() {
        let a = Course::new("Linear Algebra");
        let mut b = Course::new("Linear Algebra");
        b.educational_level = Some(EducationalLevel::Undergraduate);
        let engine = MatchingEngine::default_config();
        let name_only = engine.match_courses(&a, &Course::new("Linear Algebra"));
        let r = engine.match_courses(&a, &b);
        assert_eq!(r.breakdown.educational_level_score, None);
        assert_eq!(r.evidence.fields_compared, 1);
        assert!((r.evidence.weight_bits - name_only.evidence.weight_bits).abs() < 1e-12);
    }

    // Identical sparse records are a match, but consistent, not proven.
    #[test]
    fn identical_sparse_records_are_consistent_not_proven() {
        let a = Course::new("Linear Algebra");
        let r = MatchingEngine::default_config().match_courses(&a, &a.clone());
        assert!(r.is_match);
        assert_eq!(r.evidence.strength, MatchStrength::Consistent);
        assert!((r.evidence.weight_bits - (NAME_M / NAME_U).log2()).abs() < 1e-9);
    }

    #[test]
    fn shared_unique_identifier_is_proven() {
        let mut a = Course::new("A");
        let mut b = Course::new("Completely different");
        a.identifiers.push(ident(IdentifierScheme::Doi, "10.1/x"));
        b.identifiers.push(ident(IdentifierScheme::Doi, "10.1/X"));
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        assert!(r.breakdown.deterministic_match);
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
        // Name components are still weighed beside the identifier.
        assert_eq!(r.evidence.fields_compared, 2);
    }

    #[test]
    fn shared_same_as_is_proven() {
        let mut a = Course::new("A");
        let mut b = Course::new("A");
        a.same_as.push("https://www.wikidata.org/wiki/Q1".into());
        b.same_as.push("https://www.wikidata.org/wiki/Q1".into());
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    // Within one provider a shared course code is an identifier hit.
    #[test]
    fn same_provider_course_code_is_proven() {
        let mut a = Course::new("Intro CS");
        let mut b = Course::new("Introduction to Computer Science");
        for c in [&mut a, &mut b] {
            c.provider_id = Some("prov-1".into());
        }
        a.course_code = Some("cs101".into());
        b.course_code = Some("CS 101".into());
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    #[test]
    fn conflicting_unique_identifiers_are_contradicted() {
        let mut a = rich("Algorithms");
        let mut b = a.clone();
        a.identifiers.push(ident(IdentifierScheme::Wikidata, "Q1"));
        b.identifiers.push(ident(IdentifierScheme::Wikidata, "Q2"));
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        assert!(r.is_match);
        assert_eq!(r.evidence.strength, MatchStrength::Contradicted);
        assert_eq!(
            r.evidence.contradictions,
            vec![Contradiction {
                field: "identifier:wikidata".into()
            }]
        );
    }

    // A contradiction wins over a shared identifier under another scheme.
    #[test]
    fn contradiction_beats_a_shared_identifier_elsewhere() {
        let mut a = Course::new("A");
        let mut b = Course::new("A");
        a.identifiers.push(ident(IdentifierScheme::Doi, "10.1/x"));
        b.identifiers.push(ident(IdentifierScheme::Doi, "10.1/x"));
        a.identifiers.push(ident(IdentifierScheme::Uuid, "u-1"));
        b.identifiers.push(ident(IdentifierScheme::Uuid, "u-2"));
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        assert_eq!(r.evidence.strength, MatchStrength::Contradicted);
    }

    // A record holding an old and a new value is not contradicted by a
    // partner holding either; blank values are ignored.
    #[test]
    fn overlapping_value_sets_and_blanks_do_not_contradict() {
        let mut a = Course::new("A");
        let mut b = Course::new("A");
        a.identifiers.push(ident(IdentifierScheme::Lom, "old"));
        a.identifiers.push(ident(IdentifierScheme::Lom, "new"));
        b.identifiers.push(ident(IdentifierScheme::Lom, "new"));
        a.identifiers.push(ident(IdentifierScheme::Oer, "  "));
        b.identifiers.push(ident(IdentifierScheme::Oer, "oer-1"));
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        assert!(r.evidence.contradictions.is_empty());
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    #[test]
    fn same_provider_different_course_codes_are_contradicted() {
        let mut a = Course::new("Algorithms");
        let mut b = Course::new("Algorithms");
        for c in [&mut a, &mut b] {
            c.provider_id = Some("prov-1".into());
        }
        a.course_code = Some("CS101".into());
        b.course_code = Some("CS102".into());
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        assert_eq!(
            r.evidence.contradictions,
            vec![Contradiction {
                field: "identifier:course_code".into()
            }]
        );
        assert_eq!(r.evidence.strength, MatchStrength::Contradicted);
    }

    #[test]
    fn different_providers_course_codes_do_not_contradict() {
        let mut a = Course::new("Algorithms");
        let mut b = Course::new("Algorithms");
        a.provider_id = Some("prov-1".into());
        b.provider_id = Some("prov-2".into());
        a.course_code = Some("CS101".into());
        b.course_code = Some("CS102".into());
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        assert!(r.evidence.contradictions.is_empty());
    }

    // The richest identifier-free course this crate models cannot reach
    // the default bar: every field it carries, all agreeing, sums to about
    // 27.6 bits. That is deliberate. Two sections or two years of one
    // course agree on all of them and are still separate records, so
    // without an identifier (or a provider-scoped code) the data is
    // consistent with a duplicate but never proof of one.
    #[test]
    fn richest_identical_record_without_identifier_is_not_proven() {
        let a = rich("Algorithms and Data Structures");
        let r = MatchingEngine::default_config().match_courses(&a, &a.clone());
        assert_eq!(r.evidence.fields_compared, 7);
        assert!(r.evidence.weight_bits < DEFAULT_PROOF_BITS);
        assert!(r.evidence.weight_bits > 27.0);
        assert_eq!(r.evidence.strength, MatchStrength::Consistent);
        // A deployment with a lower bar can accept it.
        let lenient = MatchConfig {
            proof_bits: 25.0,
            ..MatchConfig::default()
        };
        let r = MatchingEngine::new(lenient).match_courses(&a, &a.clone());
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    #[test]
    fn clear_disagreement_gives_negative_bits() {
        let mut a = rich("Linear Algebra");
        let mut b = rich("Tudor History 1485-1603");
        b.provider_id = Some("prov-2".into());
        b.educational_level = Some(EducationalLevel::PrimaryEducation);
        b.keywords = vec!["monarchy".into()];
        b.teaches = vec!["source analysis".into()];
        a.tags.clear();
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        assert!(r.evidence.weight_bits < 0.0, "{}", r.evidence.weight_bits);
        assert_eq!(r.evidence.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn old_json_without_evidence_deserialises() {
        let json = r#"{"score":0.9,"is_match":true,"confidence":"High",
            "breakdown":{"name_score":0.9,"course_code_score":null,
            "provider_score":null,"educational_level_score":null,
            "keywords_score":null,"teaches_score":null,
            "deterministic_match":false}}"#;
        let r: MatchResult = serde_json::from_str(json).expect("deserialize");
        assert_eq!(r.evidence, Evidence::default());
        assert_eq!(r.evidence.strength, MatchStrength::NonMatch);
        let cfg: MatchConfig = serde_json::from_str(
            r#"{"threshold":0.85,"name_weight":0.35,"course_code_weight":0.15,
            "provider_weight":0.15,"educational_level_weight":0.1,
            "keywords_weight":0.1,"teaches_weight":0.15,
            "relationships_weight":0.05,"tags_weight":0.05}"#,
        )
        .expect("deserialize config");
        assert!((cfg.proof_bits - DEFAULT_PROOF_BITS).abs() < 1e-12);
    }

    #[test]
    fn strength_serialises_snake_case() {
        let json = serde_json::to_string(&MatchStrength::NonMatch).expect("serialize");
        assert_eq!(json, "\"non_match\"");
    }
}
