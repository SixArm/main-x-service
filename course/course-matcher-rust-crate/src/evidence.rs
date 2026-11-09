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

/// One term of the evidence sum (match-evidence.md §8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentBits {
    /// The component, e.g. `"name"`, `"identifier"`,
    /// `"contradiction:identifier:doi"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
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
    /// Every term of the sum; the terms add up to `weight_bits`.
    #[serde(default)]
    pub components: Vec<ComponentBits>,
}

// ─── Correlated components (match-evidence.md §6) ──────────────────

/// Weight of each member of a correlated group after its strongest.
pub const CORRELATED_DISCOUNT: f64 = 0.5;

/// This crate's correlated groups, by component name. Public contract
/// (spec §18a):
///
/// - `keywords`, `teaches`, `tags` — often authored from one syllabus
///   or catalogue entry, so two unrelated courses on one subject agree
///   on all three for one reason.
/// - `provider`, `educational_level` — a provider's catalogue sits at a
///   few levels (a university's is all higher education), so two
///   unrelated courses from one provider usually share a level too.
pub const CORRELATED_GROUPS: &[&[&str]] = &[
    &["keywords", "teaches", "tags"],
    &["provider", "educational_level"],
];

/// Index of the strongest member: the largest absolute value, ties to
/// the larger value, then to the first index.
fn strongest_index(member_bits: &[f64]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, &v) in member_bits.iter().enumerate() {
        best = match best {
            None => Some(i),
            Some(j) => {
                let w = member_bits[j];
                let order = v.abs().total_cmp(&w.abs()).then(v.total_cmp(&w));
                if order == std::cmp::Ordering::Greater {
                    Some(i)
                } else {
                    Some(j)
                }
            }
        };
    }
    best
}

/// The strongest member (largest absolute value, ties to the larger
/// value) and the rest, or `None` when there are no members.
#[must_use]
pub fn strongest_first(member_bits: &[f64]) -> Option<(f64, Vec<f64>)> {
    let i = strongest_index(member_bits)?;
    let rest = member_bits
        .iter()
        .enumerate()
        .filter(|&(j, _)| j != i)
        .map(|(_, &v)| v)
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

// ─── Common and rare values (match-evidence.md §7) ─────────────────

/// Where the share of records holding a value comes from.
pub trait ValueFrequencies {
    /// The share, in `0.0..=1.0`, of records whose `component` holds
    /// `value` (already in the matcher's normalised form, see
    /// [`frequency_key`]), or `None` when unknown.
    fn share(&self, component: &str, value: &str) -> Option<f64>;
}

/// No frequencies: every share is unknown, so the m/u table stands.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFrequencies;

impl ValueFrequencies for NoFrequencies {
    fn share(&self, _component: &str, _value: &str) -> Option<f64> {
        None
    }
}

/// An in-memory table of shares, keyed by component and normalised
/// value (the output of [`frequency_key`]).
#[derive(Debug, Clone, Default)]
pub struct FrequencyTable {
    shares: std::collections::HashMap<(String, String), f64>,
}

impl FrequencyTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from `(component, key, count)` triples over `total`
    /// records: each share is `count / total`. A `total` of `0` gives an
    /// empty table.
    #[must_use]
    pub fn from_counts<I, C, K>(counts: I, total: u64) -> Self
    where
        I: IntoIterator<Item = (C, K, u64)>,
        C: Into<String>,
        K: Into<String>,
    {
        let mut table = Self::new();
        if total == 0 {
            return table;
        }
        #[allow(clippy::cast_precision_loss)]
        let total_f = total as f64;
        for (component, key, count) in counts {
            #[allow(clippy::cast_precision_loss)]
            let share = count as f64 / total_f;
            table.insert(component, key, share);
        }
        table
    }

    /// Set the share of `component` holding `key`. `key` must already be
    /// in [`frequency_key`] form.
    pub fn insert(&mut self, component: impl Into<String>, key: impl Into<String>, share: f64) {
        self.shares.insert((component.into(), key.into()), share);
    }
}

impl ValueFrequencies for FrequencyTable {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        self.shares
            .get(&(component.to_string(), value.to_string()))
            .copied()
    }
}

/// Smallest share used, so a value seen once cannot give unbounded bits.
pub const MIN_SHARE: f64 = 1e-9;

/// The components whose agreement weight a [`ValueFrequencies`] source
/// adjusts. Public contract (spec §18a).
pub const FREQUENCY_COMPONENTS: &[&str] = &["name"];

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

/// The key the matcher looks up for `component`'s raw value, so a
/// service can count values in the same form. `"name"` is the primary
/// course name under [`normalize::fold`] (trim, NFKC, lowercase).
/// Returns `None` for a component not in [`FREQUENCY_COMPONENTS`] or a
/// value that normalises to empty.
#[must_use]
pub fn frequency_key(component: &str, raw: &str) -> Option<String> {
    match component {
        "name" => Some(normalize::fold(raw)).filter(|k| !k.is_empty()),
        _ => None,
    }
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

/// The frequency-adjusted agreement bits for `component`, when the two
/// raw values have the same non-empty key and the source knows its
/// share. `None` means use the m/u table.
fn adjusted_agreement(
    component: &str,
    a_raw: &str,
    b_raw: &str,
    m: f64,
    frequencies: &dyn ValueFrequencies,
) -> Option<f64> {
    let ak = frequency_key(component, a_raw)?;
    let bk = frequency_key(component, b_raw)?;
    if ak != bk {
        return None;
    }
    frequencies
        .share(component, &ak)
        .map(|share| frequency_bits(m, share))
}

/// The per-component terms, after frequency adjustment and the
/// correlated-group discount, and the number of components compared.
fn component_terms(
    a: &Course,
    b: &Course,
    components: &MatchBreakdown,
    frequencies: &dyn ValueFrequencies,
) -> (Vec<ComponentBits>, u32) {
    let mut fields_compared = 0_u32;
    let table = [
        ("name", components.name_score, NAME_M, NAME_U),
        (
            "provider",
            components.provider_score,
            PROVIDER_M,
            PROVIDER_U,
        ),
        (
            "educational_level",
            components.educational_level_score,
            EDUCATIONAL_LEVEL_M,
            EDUCATIONAL_LEVEL_U,
        ),
        (
            "keywords",
            components.keywords_score,
            KEYWORDS_M,
            KEYWORDS_U,
        ),
        ("teaches", components.teaches_score, TEACHES_M, TEACHES_U),
        (
            "relationships",
            components.relationships_score,
            RELATIONSHIPS_M,
            RELATIONSHIPS_U,
        ),
        ("tags", components.tags_score, TAGS_M, TAGS_U),
    ];
    // Raw per-component terms, before any correlated discount.
    let mut terms: Vec<ComponentBits> = Vec::new();
    for (name, similarity, m, u) in table {
        let Some(s) = similarity else { continue };
        fields_compared += 1;
        // Only an exact agreement on the same normalised value adjusts.
        let adjusted = if name == "name" {
            adjusted_agreement(name, &a.name, &b.name, m, frequencies)
        } else {
            None
        };
        let (bits, frequency_adjusted) = match adjusted {
            Some(bits) => (bits, true),
            None => (component_bits(s, m, u), false),
        };
        terms.push(ComponentBits {
            component: name.to_string(),
            bits,
            correlated: false,
            frequency_adjusted,
        });
    }
    // Correlated groups: the strongest present member counts in full,
    // every other present member at CORRELATED_DISCOUNT.
    for group in CORRELATED_GROUPS {
        let members: Vec<usize> = terms
            .iter()
            .enumerate()
            .filter(|(_, t)| group.contains(&t.component.as_str()))
            .map(|(i, _)| i)
            .collect();
        let bits: Vec<f64> = members.iter().map(|&i| terms[i].bits).collect();
        let Some(strongest) = strongest_index(&bits) else {
            continue;
        };
        for (k, &i) in members.iter().enumerate() {
            if k != strongest {
                terms[i].bits *= CORRELATED_DISCOUNT;
                terms[i].correlated = true;
            }
        }
    }

    (terms, fields_compared)
}

/// The identifier and contradiction terms, the contradictions, and
/// whether any identifier rule compared values on both sides.
fn identifier_terms(a: &Course, b: &Course) -> (Vec<ComponentBits>, Vec<Contradiction>, bool) {
    let mut terms = Vec::new();
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
        terms.push(ComponentBits {
            component: "identifier".to_string(),
            bits: IDENTIFIER_BITS,
            correlated: false,
            frequency_adjusted: false,
        });
    }
    for c in &contradictions {
        terms.push(ComponentBits {
            component: format!("contradiction:{}", c.field),
            bits: CONTRADICTION_BITS,
            correlated: false,
            frequency_adjusted: false,
        });
    }
    (terms, contradictions, identifiers_compared)
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
    frequencies: &dyn ValueFrequencies,
) -> Evidence {
    let (mut terms, mut fields_compared) = component_terms(a, b, components, frequencies);
    let (identifier, contradictions, identifiers_compared) = identifier_terms(a, b);
    terms.extend(identifier);
    if identifiers_compared {
        fields_compared += 1;
    }

    // The total is the sum of the listed terms, in list order, so the
    // listing adds up to it exactly.
    let weight_bits: f64 = terms.iter().map(|t| t.bits).sum();
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
    // the default bar. Every field it carries, all agreeing, summed to
    // about 27.6 bits before the correlated groups; with them it is about
    // 23.62 (keywords/teaches/tags and provider/level each count their
    // strongest member in full and the rest at half). That is deliberate.
    // Two sections or two years of one course agree on all of them and
    // are still separate records, so without an identifier (or a
    // provider-scoped code) the data is consistent with a duplicate but
    // never proof of one.
    #[test]
    fn richest_identical_record_without_identifier_is_not_proven() {
        let a = rich("Algorithms and Data Structures");
        let r = MatchingEngine::default_config().match_courses(&a, &a.clone());
        assert_eq!(r.evidence.fields_compared, 7);
        let expected = (NAME_M / NAME_U).log2()
            + (RELATIONSHIPS_M / RELATIONSHIPS_U).log2()
            + (TEACHES_M / TEACHES_U).log2()
            + 0.5 * ((KEYWORDS_M / KEYWORDS_U).log2() + (TAGS_M / TAGS_U).log2())
            + (PROVIDER_M / PROVIDER_U).log2()
            + 0.5 * (EDUCATIONAL_LEVEL_M / EDUCATIONAL_LEVEL_U).log2();
        assert!((r.evidence.weight_bits - expected).abs() < 1e-9);
        assert!((r.evidence.weight_bits - 23.62).abs() < 0.01);
        assert_eq!(r.evidence.strength, MatchStrength::Consistent);
        // A deployment with a lower bar can accept it.
        let lenient = MatchConfig {
            proof_bits: 20.0,
            ..MatchConfig::default()
        };
        let r = MatchingEngine::new(lenient).match_courses(&a, &a.clone());
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    fn term<'a>(e: &'a Evidence, component: &str) -> &'a ComponentBits {
        e.components
            .iter()
            .find(|t| t.component == component)
            .unwrap_or_else(|| panic!("no term {component}"))
    }

    fn sum_of_terms(e: &Evidence) -> f64 {
        e.components.iter().map(|t| t.bits).sum()
    }

    #[test]
    fn group_bits_discounts_all_but_the_strongest() {
        assert!((group_bits(&[4.0, 3.0]) - 5.5).abs() < 1e-12);
        assert!(group_bits(&[4.0, 3.0]) < 4.0 + 3.0);
        assert!(group_bits(&[]).abs() < 1e-12);
        assert!((group_bits(&[2.5]) - 2.5).abs() < 1e-12);
    }

    // Order of the members does not change the result, ties included.
    #[test]
    fn group_bits_is_order_independent() {
        let perms = [[4.0, 3.0, -2.0], [3.0, -2.0, 4.0], [-2.0, 4.0, 3.0]];
        for p in perms {
            assert!((group_bits(&p) - (4.0 + 0.5 * 1.0)).abs() < 1e-12);
        }
        // Tie on absolute value: the larger value is the strongest.
        assert_eq!(strongest_first(&[-3.0, 3.0]).map(|(s, _)| s), Some(3.0));
        assert_eq!(strongest_first(&[3.0, -3.0]).map(|(s, _)| s), Some(3.0));
        assert!((group_bits(&[-3.0, 3.0]) - group_bits(&[3.0, -3.0])).abs() < 1e-12);
        assert!((group_bits(&[-3.0, 3.0]) - 1.5).abs() < 1e-12);
    }

    // Disagreements in one group are discounted too: one reason, not two.
    #[test]
    fn correlated_disagreements_are_discounted() {
        assert!((group_bits(&[-4.0, -2.0]) - (-5.0)).abs() < 1e-12);
        let mut a = Course::new("Algorithms");
        let mut b = Course::new("Algorithms");
        a.keywords = vec!["graphs".into()];
        b.keywords = vec!["poetry".into()];
        a.teaches = vec!["recursion".into()];
        b.teaches = vec!["metre".into()];
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        let kw = ((1.0 - KEYWORDS_M) / (1.0 - KEYWORDS_U)).log2();
        let te = ((1.0 - TEACHES_M) / (1.0 - TEACHES_U)).log2();
        // teaches disagrees more strongly, so keywords is the one halved.
        assert_eq!(term(&r.evidence, "teaches").bits.to_bits(), te.to_bits());
        assert!((term(&r.evidence, "keywords").bits - 0.5 * kw).abs() < 1e-12);
        assert!(term(&r.evidence, "keywords").correlated);
        assert!(!term(&r.evidence, "teaches").correlated);
    }

    // Two agreeing correlated fields add less than their sum, whichever
    // record order the matcher sees.
    #[test]
    fn correlated_agreements_add_less_than_their_sum() {
        let mut a = Course::new("Algorithms");
        a.keywords = vec!["graphs".into()];
        a.teaches = vec!["recursion".into()];
        let b = a.clone();
        let engine = MatchingEngine::default_config();
        let r = engine.match_courses(&a, &b);
        let kw = (KEYWORDS_M / KEYWORDS_U).log2();
        let te = (TEACHES_M / TEACHES_U).log2();
        let group = term(&r.evidence, "keywords").bits + term(&r.evidence, "teaches").bits;
        assert!(group < kw + te);
        assert!((group - (te + 0.5 * kw)).abs() < 1e-12);
        let rev = engine.match_courses(&b, &a);
        assert!((rev.evidence.weight_bits - r.evidence.weight_bits).abs() < 1e-12);
    }

    // Components in different groups (or none) still add in full.
    #[test]
    fn uncorrelated_fields_still_sum() {
        let mut a = Course::new("Algorithms");
        a.keywords = vec!["graphs".into()];
        a.educational_level = Some(EducationalLevel::Undergraduate);
        let r = MatchingEngine::default_config().match_courses(&a, &a.clone());
        let expected = (NAME_M / NAME_U).log2()
            + (KEYWORDS_M / KEYWORDS_U).log2()
            + (EDUCATIONAL_LEVEL_M / EDUCATIONAL_LEVEL_U).log2();
        assert!((r.evidence.weight_bits - expected).abs() < 1e-12);
        assert!(r.evidence.components.iter().all(|t| !t.correlated));
    }

    #[test]
    fn frequency_bits_clamps_to_zero_and_min_share() {
        assert!(frequency_bits(NAME_M, NAME_M).abs() < 1e-12);
        assert!(frequency_bits(NAME_M, 0.99).abs() < 1e-12);
        assert!(frequency_bits(NAME_M, 1.0) >= 0.0);
        assert!((frequency_bits(NAME_M, 0.0) - (NAME_M / MIN_SHARE).log2()).abs() < 1e-9);
        assert!(frequency_bits(NAME_M, f64::NAN).abs() < 1e-12);
    }

    fn name_table(name: &str, share: f64) -> FrequencyTable {
        let mut t = FrequencyTable::new();
        let key = frequency_key("name", name).expect("non-empty name");
        t.insert("name", key, share);
        t
    }

    // "Introduction to Biology" is offered by many providers. Agreeing on
    // it, held by 1% of records, proves less than the table's average
    // u; a name held by one record in a million proves more.
    #[test]
    fn common_name_proves_less_and_rare_name_more() {
        let engine = MatchingEngine::default_config();
        let default_bits = (NAME_M / NAME_U).log2();

        let common = Course::new("Introduction to Biology");
        let freq = name_table("  INTRODUCTION to Biology ", 0.01);
        let r = engine.match_courses_with_frequencies(&common, &common.clone(), &freq);
        let t = term(&r.evidence, "name");
        assert!(t.frequency_adjusted);
        assert!(t.bits < default_bits);
        assert!((t.bits - (NAME_M / 0.01).log2()).abs() < 1e-12);

        let rare = Course::new("Paleoclimatology of the Ordovician");
        let freq = name_table("Paleoclimatology of the Ordovician", 1e-6);
        let r = engine.match_courses_with_frequencies(&rare, &rare.clone(), &freq);
        assert!(term(&r.evidence, "name").bits > default_bits);
    }

    // A value most records hold is no evidence, and never evidence against.
    #[test]
    fn share_at_or_above_m_gives_zero_bits() {
        let a = Course::new("Introduction to Biology");
        let engine = MatchingEngine::default_config();
        for share in [NAME_M, 0.99, 1.0] {
            let freq = name_table("Introduction to Biology", share);
            let r = engine.match_courses_with_frequencies(&a, &a.clone(), &freq);
            let t = term(&r.evidence, "name");
            assert!(t.frequency_adjusted);
            assert!(t.bits.abs() < 1e-12, "share {share} gave {}", t.bits);
        }
    }

    // Partial agreement and disagreement keep the m/u table.
    #[test]
    fn no_frequency_adjustment_on_partial_agreement_or_disagreement() {
        let engine = MatchingEngine::default_config();
        let mut freq = name_table("Introduction to Biology", 1e-6);
        freq.insert("name", "intro to biology", 1e-6);
        freq.insert("name", "tudor history", 1e-6);
        let a = Course::new("Introduction to Biology");
        for other in ["Intro to Biology", "Tudor History"] {
            let b = Course::new(other);
            let plain = engine.match_courses(&a, &b);
            let r = engine.match_courses_with_frequencies(&a, &b, &freq);
            assert!(!term(&r.evidence, "name").frequency_adjusted);
            assert_eq!(r.evidence, plain.evidence);
        }
    }

    #[test]
    fn no_frequencies_equals_the_plain_call() {
        let engine = MatchingEngine::default_config();
        let mut a = rich("Algorithms");
        let b = rich("Algorithms and Data Structures");
        a.identifiers.push(ident(IdentifierScheme::Doi, "10.1/x"));
        for (x, y) in [(&a, &b), (&a, &a)] {
            let plain = engine.match_courses(x, y);
            let none = engine.match_courses_with_frequencies(x, y, &NoFrequencies);
            assert_eq!(plain.evidence, none.evidence);
            assert!((plain.score - none.score).abs() < f64::EPSILON);
            assert_eq!(plain.is_match, none.is_match);
            assert_eq!(plain.confidence, none.confidence);
        }
    }

    // Frequencies change only the evidence, never score / is_match /
    // confidence.
    #[test]
    fn frequencies_leave_score_unchanged() {
        let engine = MatchingEngine::default_config();
        let a = Course::new("Introduction to Biology");
        let freq = name_table("Introduction to Biology", 0.5);
        let plain = engine.match_courses(&a, &a.clone());
        let r = engine.match_courses_with_frequencies(&a, &a.clone(), &freq);
        assert!((plain.score - r.score).abs() < f64::EPSILON);
        assert_eq!(plain.is_match, r.is_match);
        assert_eq!(plain.confidence, r.confidence);
    }

    #[test]
    fn frequency_key_matches_the_compared_form() {
        assert_eq!(
            frequency_key("name", "  Café  ÉTUDES "),
            Some(normalize::fold("Café  ÉTUDES"))
        );
        assert_eq!(frequency_key("name", "   "), None);
        assert_eq!(frequency_key("provider", "prov-1"), None);
        assert!(
            FREQUENCY_COMPONENTS
                .iter()
                .all(|c| frequency_key(c, "x").is_some())
        );
    }

    #[test]
    fn frequency_table_from_counts() {
        let t = FrequencyTable::from_counts([("name", "introduction to biology", 25_u64)], 100);
        assert_eq!(t.share("name", "introduction to biology"), Some(0.25));
        assert_eq!(t.share("name", "other"), None);
        let empty = FrequencyTable::from_counts([("name", "x", 1_u64)], 0);
        assert_eq!(empty.share("name", "x"), None);
    }

    // Identifier and contradiction terms are listed, and the terms add
    // up to weight_bits exactly.
    #[test]
    fn components_list_every_term_and_sum_to_weight_bits() {
        let mut a = rich("Algorithms");
        let mut b = a.clone();
        a.identifiers.push(ident(IdentifierScheme::Doi, "10.1/x"));
        b.identifiers.push(ident(IdentifierScheme::Doi, "10.1/x"));
        a.identifiers.push(ident(IdentifierScheme::Uuid, "u-1"));
        b.identifiers.push(ident(IdentifierScheme::Uuid, "u-2"));
        let r = MatchingEngine::default_config().match_courses(&a, &b);
        let e = &r.evidence;
        assert!((term(e, "identifier").bits - IDENTIFIER_BITS).abs() < 1e-12);
        assert!((term(e, "contradiction:identifier:uuid").bits - CONTRADICTION_BITS).abs() < 1e-12);
        assert_eq!(sum_of_terms(e).to_bits(), e.weight_bits.to_bits());
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
        let e: Evidence = serde_json::from_str(
            r#"{"weight_bits":1.0,"fields_compared":1,"contradictions":[],"strength":"consistent"}"#,
        )
        .expect("deserialize phase-1 evidence");
        assert!(e.components.is_empty());
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
