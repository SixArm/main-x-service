//! Cross-service `same_identity` suggestion — the comparator (LNK-4,
//! spec `13-tasks.md` T-29, design decisions pinned at
//! `spec/16-open-questions.md` OQ-9,
//! `agents/share/cross-service-linking.md` §5.2).
//!
//! This module is **pure, deterministic, and offline**: no database, no
//! HTTP, no clock. It answers exactly one question — "how likely is it
//! that this person and this worker are the same human?" — as a `[0.0,
//! 1.0]` confidence plus a per-component breakdown, in the same spirit as
//! the within-entity matchers' score breakdown
//! (`agents/share/match.md`). Later tasks build on top of it without
//! changing it: T-30 adds candidate *blocking* (which pairs get compared
//! at all), T-31 the periodic job that pulls person/worker records and
//! POSTs suggested edges, T-32/T-33 the review/promotion path and
//! threshold tuning. None of that lives here — this module only scores a
//! pair it is handed.
//!
//! ## The §7 partition rule
//!
//! [`IdentityProbe`] is deliberately **not** `Person` and **not**
//! `Worker`, and carries no `From`/`Into` conversion back into either. A
//! cross-service identity suggestion produced here must never be fed into
//! `person_matcher::MatchingEngine` or `worker_matcher::MatchingEngine` —
//! those score *within-entity* sameness from a `relationships` signal;
//! cross-service links are never a matcher input
//! (`agents/share/cross-service-linking.md` §7). The following does not
//! compile, which is the point:
//!
//! ```compile_fail
//! use link_graph_service::suggest::IdentityProbe;
//! use person_matcher::Person;
//!
//! fn only_if_convertible(probe: IdentityProbe) -> Person {
//!     Person::from(probe) // no such `From` impl exists, and never should
//! }
//! ```
//!
//! ## Score composition
//!
//! Two paths, mirroring the family's probabilistic-vs-deterministic split
//! (`agents/share/match.md`):
//!
//! - **Identifier short-circuit.** If both sides carry a coded identifier
//!   with the same `(scheme, value)` after normalisation (e.g. the same
//!   NHS number), the pair is a near-certain match:
//!   [`IDENTIFIER_MATCH_CEILING`] (`0.99`). Not `1.0` — per design §5.2 a
//!   suggestion is always `confidence < 1.0`; only an operator's
//!   confirmation promotes an edge to `1.0` (T-32). Unlike a within-entity
//!   deterministic short-circuit, this is still just a *suggestion*: OQ-9
//!   pins that there is no auto-merge tier for cross-service identity —
//!   every candidate, even an exact-identifier hit, lands in the review
//!   queue (T-33).
//! - **Weighted probabilistic score**, otherwise. Three components, each
//!   `None` (excluded, not zero) when either side lacks the field — so
//!   absence never counts as a mismatch and never manufactures a match
//!   (the family's "no spurious identity on absence" invariant,
//!   `agents/share/security.md` invariant 4 / SEC-M2/M3):
//!
//!   | Component | Weight | Algorithm |
//!   |---|---|---|
//!   | Name  | 0.45 | [`person_matcher::Scorer::jaro_winkler_similarity`], family 0.6 / given 0.4 |
//!   | Birth date | 0.45 | [`score_dob_pair`] — exact / near / transposition / year-only table |
//!   | Gender | 0.10 | exact `1.0`, either `Unknown` `0.5`, mismatch `0.0` |
//!
//!   Weights are renormalised over whichever components are present (the
//!   same "only fields present in both records contribute" rule the
//!   within-entity address scorer uses). If **no** component is present
//!   at all, the score is `0.0` — no evidence, no confidence. Name and
//!   birth date dominate because they are the two fields most likely to
//!   be recorded identically across a general person registry and a
//!   workforce registry; gender is the weakest signal (few states, high
//!   prior collision rate — two unrelated records agree on gender roughly
//!   half the time) so it can only nudge the score, never decide it: even
//!   a full gender mismatch costs at most `0.10` off a perfect name+DOB
//!   match, where a genuine name **or** DOB mismatch costs up to `0.45`.
//!
//!   The weighted average is then scaled by [`PROBABILISTIC_CEILING`]
//!   (`0.97`), so a *perfect* name+DOB+gender agreement — real, but still
//!   only circumstantial evidence two different registries describe the
//!   same human — can never reach, let alone exceed,
//!   [`IDENTIFIER_MATCH_CEILING`]. A shared coded identifier is
//!   qualitatively stronger evidence than any amount of demographic
//!   agreement (two people can share a name and birthday; they cannot
//!   legitimately share an NHS number), so identifier match must
//!   *dominate* every probabilistic outcome, not merely most of them.
//!
//! ## Candidate blocking (T-30)
//!
//! Scoring every person against every worker is `O(n·m)` — exactly the
//! unbounded fan-out `agents/share/security.md` §3 invariant 3 warns
//! against. [`generate_candidates`] bounds this per OQ-9(a)'s pinned
//! design: group both sides into buckets by [`block_keys`] and only
//! score pairs that land in the same bucket, which is `O(n + m +
//! Σ|block|²)` rather than `O(n·m)`. The block key is two-tier — an
//! exact `(scheme, value)` match on a shared coded identifier when one
//! is present (the strongest block, mirroring the matcher family's own
//! deterministic identifier short-circuit), else `Soundex(family) +
//! birth_year` (reusing [`person_matcher::Normalizer::phonetic_code`],
//! the same phonetic primitive the within-entity name component already
//! uses — no fresh Soundex implementation needed). A probe with neither
//! a usable identifier nor a usable name+DOB produces no block key and
//! can never be compared to anything, which is consistent with the
//! comparator's own "no evidence, no confidence" posture.
//!
//! [`generate_candidates`] discards (never returns) anything scoring
//! below [`SUGGESTION_THRESHOLD`] (`0.7`, reused from
//! `BatchDeduplicationRequest::threshold`'s default / `IMPORT_REVIEW_THRESHOLD`
//! rather than inventing a new number) and returns everything at or
//! above it unchanged — there is no auto-merge tier here: even a `0.99`
//! identifier-ceiling hit is still only a candidate for T-32/T-33's
//! operator confirmation, never an automatic promotion. A pair reachable
//! through more than one shared block (e.g. two shared identifiers) is
//! scored and returned at most once.
//!
//! ## Birth-date table — a deliberate deviation from `score_dob_pair`
//!
//! `person-matcher`'s own private `score_dob_pair`
//! (`person-matcher-rust-crate/src/matcher.rs`) only implements two of
//! the six rows documented in
//! `person-service-with-loco/AGENTS/matching.md`'s "Birth Date Matching"
//! table (exact `1.0`, same-year transposition `0.5`, else `0.0`) — the
//! doc and the code have drifted. Rather than reach into that private
//! function (or make it `pub` to paper over a documented-but-unimplemented
//! table), [`score_dob_pair`] here is a fresh, small, pure function that
//! implements the **documented** six-row table directly, so this
//! comparator matches what the family's own docs promise rather than
//! propagating the drift. It takes no dependency on `person-matcher`'s
//! private internals.

use person_matcher::{Gender, Normalizer, Scorer};

use chrono::{Datelike, NaiveDate};
use entity_ref::EntityRef;
use std::collections::{HashMap, HashSet};

/// A coded identifier scoped to its issuing system — e.g. `("nhs",
/// "9434765919")` or `("ssn", "078-05-1120")`.
///
/// Comparison is **exact match on the normalised pair**: `scheme` is
/// trimmed and lower-cased; `value` has all whitespace removed (coded
/// identifiers like an NHS number are commonly displayed
/// space-separated — `"943 476 5919"` — but that is formatting, not
/// content) and is upper-cased (mirroring the uppercase-canonicalisation
/// convention `person-matcher::identifiers` already uses for
/// national-identifier values). A blank scheme or value on either side
/// never contributes a match — an empty string must never stand in for
/// "no identifier" and accidentally agree with another empty string
/// (`agents/share/security.md` invariant 4).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProbeIdentifier {
    scheme: String,
    value: String,
}

impl ProbeIdentifier {
    /// Build a normalised identifier. Returns `None` if either `scheme`
    /// or `value` is blank after normalisation — a blank identifier
    /// carries no evidence and must never be stored as one that could
    /// spuriously match another blank.
    #[must_use]
    pub fn new(scheme: impl AsRef<str>, value: impl AsRef<str>) -> Option<Self> {
        let scheme = scheme.as_ref().trim().to_lowercase();
        let value = value
            .as_ref()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_uppercase();
        if scheme.is_empty() || value.is_empty() {
            return None;
        }
        Some(Self { scheme, value })
    }
}

/// A name split into family and given components, the two sub-fields the
/// within-entity matchers already score independently.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProbeName {
    /// Family (last) name.
    pub family: String,
    /// Given (first) name.
    pub given: String,
}

/// The lean, cross-type identity projection both a `Person` and a
/// `Worker` record map to for comparison (design §5.2 / OQ-9). Mapping
/// from the two services' actual API responses is T-31's job (the
/// periodic suggestion job); this type and [`compare_identity`] are
/// deliberately independent of both — they only ever see this shape.
///
/// `IdentityProbe` carries no conversion to or from `person_matcher::Person`
/// or `worker_matcher::Worker` — see the module-level `§7 partition rule`
/// section.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IdentityProbe {
    /// The record's name, if known.
    pub name: Option<ProbeName>,
    /// The record's date of birth, if known.
    pub birth_date: Option<NaiveDate>,
    /// The record's administrative gender, if known.
    pub gender: Option<Gender>,
    /// Coded identifiers the record carries (NHS number, SSN, …). May be
    /// empty.
    pub identifiers: Vec<ProbeIdentifier>,
}

/// The ceiling confidence for an identifier-match suggestion. Deliberately
/// `< 1.0` — see the module doc's "Identifier short-circuit" section.
pub const IDENTIFIER_MATCH_CEILING: f64 = 0.99;

/// The ceiling confidence for the weighted probabilistic score (no shared
/// identifier), strictly below [`IDENTIFIER_MATCH_CEILING`] so an
/// identifier match always outranks even a perfect demographic agreement
/// — see the module doc's score-composition section.
pub const PROBABILISTIC_CEILING: f64 = 0.97;

/// Weight of the name component in the weighted score (see the module
/// doc's score-composition table).
const NAME_WEIGHT: f64 = 0.45;
/// Weight of the birth-date component.
const DOB_WEIGHT: f64 = 0.45;
/// Weight of the gender component.
const GENDER_WEIGHT: f64 = 0.10;

/// Weight of the family-name sub-component within the name component.
const FAMILY_WEIGHT: f64 = 0.6;
/// Weight of the given-name sub-component within the name component.
const GIVEN_WEIGHT: f64 = 0.4;

/// The result of comparing two [`IdentityProbe`]s: an overall confidence
/// plus the per-component breakdown that produced it, in the same spirit
/// as the within-entity matchers' score breakdown
/// (`agents/share/match.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IdentityMatchScore {
    /// Overall confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Whether the confidence came from the identifier short-circuit
    /// rather than the weighted probabilistic score.
    pub identifier_match: bool,
    /// The name component score, or `None` if excluded (no name on one or
    /// both sides, or both sides blank).
    pub name_score: Option<f64>,
    /// The birth-date component score, or `None` if either side has no
    /// birth date.
    pub dob_score: Option<f64>,
    /// The gender component score, or `None` if either side has no
    /// gender recorded.
    pub gender_score: Option<f64>,
}

/// Score a single name sub-component (family or given), guarding against
/// the empty/empty spurious-match case: two blank strings must not score
/// `1.0` (`Scorer::jaro_winkler_similarity`'s own documented edge case),
/// so a blank on either side excludes the sub-component instead.
fn score_name_part(a: &str, b: &str) -> Option<f64> {
    let a = a.trim();
    let b = b.trim();
    if a.is_empty() || b.is_empty() {
        return None;
    }
    Some(Scorer::jaro_winkler_similarity(a, b))
}

/// Score two names as the weighted blend of family (0.6) and given (0.4)
/// sub-component scores, renormalised over whichever sub-components are
/// present. `None` if neither sub-component is present on both sides.
fn score_name(a: &ProbeName, b: &ProbeName) -> Option<f64> {
    let family = score_name_part(&a.family, &b.family);
    let given = score_name_part(&a.given, &b.given);
    weighted_average(&[(family, FAMILY_WEIGHT), (given, GIVEN_WEIGHT)])
}

/// Score a pair of birth dates against the documented six-row table
/// (`person-service-with-loco/AGENTS/matching.md` "Birth Date Matching")
/// — see the module doc for why this is a fresh implementation rather
/// than a reuse of `person-matcher`'s private, narrower `score_dob_pair`.
///
/// | Condition | Score |
/// |---|---|
/// | Exact match | 1.0 |
/// | Day off by 1–2 (same year+month) | 0.95 |
/// | Month/day transposition (same year) | 0.90 |
/// | Same year and month | 0.80 |
/// | Year off by 1 (same month+day) | 0.85 |
/// | Same year only | 0.50 |
/// | Otherwise | 0.0 |
///
/// Rows are checked in the table's own precedence order (a transposition
/// is checked before the coarser "same year and month" test, since a
/// transposed date is *also* same-year-and-month).
#[must_use]
pub fn score_dob_pair(a: NaiveDate, b: NaiveDate) -> f64 {
    if a == b {
        return 1.0;
    }
    let same_year = a.year() == b.year();
    if same_year && a.month() == b.month() && a.day().abs_diff(b.day()) <= 2 {
        return 0.95;
    }
    if same_year
        && let Some(swapped) = NaiveDate::from_ymd_opt(a.year(), a.day(), a.month())
        && swapped == b
    {
        return 0.90;
    }
    if same_year && a.month() == b.month() {
        return 0.80;
    }
    if (a.year() - b.year()).abs() == 1 && a.month() == b.month() && a.day() == b.day() {
        return 0.85;
    }
    if same_year {
        return 0.50;
    }
    0.0
}

/// Score two genders per the documented table
/// (`person-service-with-loco/AGENTS/matching.md` "Gender Matching"):
/// exact match `1.0`, either side `Unknown` `0.5`, mismatch `0.0`.
///
/// The `Unknown` check runs **before** the equality check: two `Unknown`
/// values are not treated as a confirmed exact match, only as the same
/// soft ambiguity a single `Unknown` produces. Both sides recording "we
/// don't know" is not evidence the two records agree — scoring it `1.0`
/// would be exactly the shared-sentinel spurious match
/// `agents/share/security.md` invariant 4 warns against.
fn score_gender_pair(a: Gender, b: Gender) -> f64 {
    if a == Gender::Unknown || b == Gender::Unknown {
        0.5
    } else if a == b {
        1.0
    } else {
        0.0
    }
}

/// Weighted average over `(score, weight)` pairs, renormalised over the
/// `Some` entries. `None` if every entry is `None` (no evidence at all).
fn weighted_average(components: &[(Option<f64>, f64)]) -> Option<f64> {
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;
    for (score, weight) in components {
        if let Some(score) = score {
            weighted_sum += score * weight;
            weight_total += weight;
        }
    }
    if weight_total <= 0.0 {
        None
    } else {
        Some(weighted_sum / weight_total)
    }
}

/// Whether `a` and `b` share at least one identifier with the same
/// normalised `(scheme, value)` pair.
fn shares_identifier(a: &IdentityProbe, b: &IdentityProbe) -> bool {
    a.identifiers.iter().any(|ia| b.identifiers.contains(ia))
}

/// Compare two [`IdentityProbe`]s and return an [`IdentityMatchScore`].
///
/// Pure and deterministic: the same two probes always produce the same
/// score, and no I/O, database, or cross-service edge is consulted or
/// produced — that is entirely T-31's job, downstream of this function's
/// output. See the module doc for the full scoring design.
///
/// # Examples
///
/// ```
/// use chrono::NaiveDate;
/// use link_graph_service::suggest::{compare_identity, IdentityProbe, ProbeIdentifier, ProbeName};
/// use person_matcher::Gender;
///
/// let person = IdentityProbe {
///     name: Some(ProbeName { family: "Smith".into(), given: "Jane".into() }),
///     birth_date: NaiveDate::from_ymd_opt(1980, 6, 15),
///     gender: Some(Gender::Female),
///     identifiers: vec![ProbeIdentifier::new("nhs", "943 476 5919").unwrap()],
/// };
/// let worker = IdentityProbe {
///     identifiers: vec![ProbeIdentifier::new("nhs", "9434765919").unwrap()],
///     ..person.clone()
/// };
///
/// let score = compare_identity(&person, &worker);
/// assert!(score.identifier_match);
/// assert!(score.confidence >= 0.95);
/// ```
#[must_use]
pub fn compare_identity(a: &IdentityProbe, b: &IdentityProbe) -> IdentityMatchScore {
    let name_score = match (&a.name, &b.name) {
        (Some(na), Some(nb)) => score_name(na, nb),
        _ => None,
    };
    let dob_score = match (a.birth_date, b.birth_date) {
        (Some(da), Some(db)) => Some(score_dob_pair(da, db)),
        _ => None,
    };
    let gender_score = match (a.gender, b.gender) {
        (Some(ga), Some(gb)) => Some(score_gender_pair(ga, gb)),
        _ => None,
    };

    if shares_identifier(a, b) {
        return IdentityMatchScore {
            confidence: IDENTIFIER_MATCH_CEILING,
            identifier_match: true,
            name_score,
            dob_score,
            gender_score,
        };
    }

    let confidence = weighted_average(&[
        (name_score, NAME_WEIGHT),
        (dob_score, DOB_WEIGHT),
        (gender_score, GENDER_WEIGHT),
    ])
    .map_or(0.0, |score| score * PROBABILISTIC_CEILING);

    IdentityMatchScore {
        confidence,
        identifier_match: false,
        name_score,
        dob_score,
        gender_score,
    }
}

// ---------------------------------------------------------------------
// T-30: candidate blocking (OQ-9(a)) — see the module doc's "Candidate
// blocking" section for the design.
// ---------------------------------------------------------------------

/// The review-worthy confidence threshold below which a candidate is
/// discarded (never returned, never stored) rather than surfaced.
/// Reused, not invented — the same `0.7` line as
/// `BatchDeduplicationRequest::threshold`'s default and
/// `IMPORT_REVIEW_THRESHOLD` (`person-service-with-loco`; OQ-9(a)).
pub const SUGGESTION_THRESHOLD: f64 = 0.7;

/// A blocking key computed from an [`IdentityProbe`] (OQ-9(a)). Two
/// probes are only ever compared by [`generate_candidates`] if they
/// share at least one key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BlockKey {
    /// An exact, normalised coded-identifier `(scheme, value)` pair.
    Identifier(String, String),
    /// `Soundex(family) + birth_year`, the fallback block for a probe
    /// with no coded identifier.
    Phonetic(String, i32),
}

/// Compute every block key an [`IdentityProbe`] belongs to (OQ-9(a)).
///
/// A probe carrying one or more coded identifiers blocks on **each** of
/// their normalised `(scheme, value)` pairs — a record with multiple
/// identifiers may legitimately belong to multiple identifier-blocks,
/// and all of them are exact-match blocks, the strongest kind. A probe
/// with none falls back to a single `Soundex(family) + birth_year`
/// block, computed only when both a non-blank family name and a birth
/// date are present. A probe with neither a usable identifier nor a
/// usable name+DOB yields no block key at all: it cannot be compared to
/// anything by [`generate_candidates`], the same "no evidence, no
/// confidence" posture [`compare_identity`] already takes on absent
/// fields.
fn block_keys(probe: &IdentityProbe) -> Vec<BlockKey> {
    if !probe.identifiers.is_empty() {
        return probe
            .identifiers
            .iter()
            .map(|id| BlockKey::Identifier(id.scheme.clone(), id.value.clone()))
            .collect();
    }

    let Some(name) = &probe.name else {
        return Vec::new();
    };
    let Some(birth_date) = probe.birth_date else {
        return Vec::new();
    };
    let family = name.family.trim();
    if family.is_empty() {
        return Vec::new();
    }
    let code = Normalizer::phonetic_code(family);
    if code.is_empty() {
        return Vec::new();
    }
    vec![BlockKey::Phonetic(code, birth_date.year())]
}

/// A scored cross-service `same_identity` candidate (T-30), at or above
/// [`SUGGESTION_THRESHOLD`]. Producing the corresponding edge (T-31) and
/// routing it through review/promotion (T-32/T-33) are downstream of
/// this type — it carries no `provenance` or write-side state of its
/// own.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentityCandidate {
    /// The person-side record.
    pub person: EntityRef,
    /// The worker-side record.
    pub worker: EntityRef,
    /// The comparator's score for this pair.
    pub score: IdentityMatchScore,
}

/// Generate scored `same_identity` candidates between `persons` and
/// `workers`, using blocking to bound the comparisons performed
/// (OQ-9(a); see the module doc's "Candidate blocking" section).
///
/// Only pairs sharing at least one [`block_keys`] entry are ever passed
/// to [`compare_identity`] — a pair in different blocks is never
/// compared, which is what keeps this sub-quadratic (`O(n + m +
/// Σ|block|²)`) rather than the full `O(n·m)` cross product. A pair
/// reachable through more than one shared block (e.g. two shared
/// identifiers) is compared and, if it qualifies, returned at most
/// once. Only pairs scoring at or above [`SUGGESTION_THRESHOLD`] are
/// returned; everything below is discarded — never returned, never
/// logged as a near-miss.
///
/// Pure and deterministic, like [`compare_identity`]: no database, no
/// HTTP, no clock. Fetching the person/worker feeds this function is
/// handed is T-31's job, not this function's.
#[must_use]
pub fn generate_candidates(
    persons: &[(EntityRef, IdentityProbe)],
    workers: &[(EntityRef, IdentityProbe)],
) -> Vec<IdentityCandidate> {
    let mut person_blocks: HashMap<BlockKey, Vec<usize>> = HashMap::new();
    for (index, (_, probe)) in persons.iter().enumerate() {
        for key in block_keys(probe) {
            person_blocks.entry(key).or_default().push(index);
        }
    }

    let mut worker_blocks: HashMap<BlockKey, Vec<usize>> = HashMap::new();
    for (index, (_, probe)) in workers.iter().enumerate() {
        for key in block_keys(probe) {
            worker_blocks.entry(key).or_default().push(index);
        }
    }

    let mut seen_pairs: HashSet<(usize, usize)> = HashSet::new();
    let mut candidates = Vec::new();

    for (key, person_indexes) in &person_blocks {
        let Some(worker_indexes) = worker_blocks.get(key) else {
            continue;
        };
        for &pi in person_indexes {
            for &wi in worker_indexes {
                if !seen_pairs.insert((pi, wi)) {
                    // Already compared under an earlier shared block key.
                    continue;
                }
                let (person_ref, person_probe) = &persons[pi];
                let (worker_ref, worker_probe) = &workers[wi];
                let score = compare_identity(person_probe, worker_probe);
                if score.confidence >= SUGGESTION_THRESHOLD {
                    candidates.push(IdentityCandidate {
                        person: *person_ref,
                        worker: *worker_ref,
                        score,
                    });
                }
            }
        }
    }

    candidates
}

#[cfg(test)]
mod tests {
    //! Unit tests for the identity-suggestion comparator. All fixtures are
    //! synthetic (no PII), per the family's testing convention.

    use super::*;

    fn name(family: &str, given: &str) -> ProbeName {
        ProbeName {
            family: family.to_string(),
            given: given.to_string(),
        }
    }

    fn dob(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    /// A person/worker pair sharing an exact national identifier scores at
    /// the identifier ceiling, regardless of how the other fields compare.
    #[test]
    fn shared_identifier_scores_at_the_ceiling() {
        let person = IdentityProbe {
            name: Some(name("Smith", "Jane")),
            birth_date: Some(dob(1980, 6, 15)),
            gender: Some(Gender::Female),
            identifiers: vec![ProbeIdentifier::new("nhs", "9434765919").unwrap()],
        };
        let worker = IdentityProbe {
            name: Some(name("Smith", "Jane")),
            birth_date: Some(dob(1980, 6, 15)),
            gender: Some(Gender::Female),
            // Same value, different incidental formatting — normalisation
            // in `ProbeIdentifier::new` should still line these up.
            identifiers: vec![ProbeIdentifier::new("NHS", " 9434765919 ").unwrap()],
        };

        let score = compare_identity(&person, &worker);
        assert!(score.identifier_match);
        assert!((score.confidence - IDENTIFIER_MATCH_CEILING).abs() < f64::EPSILON);
    }

    /// A different identifier scheme or value never matches — the
    /// short-circuit requires both scheme *and* value to agree.
    #[test]
    fn different_identifier_scheme_or_value_does_not_short_circuit() {
        let a = IdentityProbe {
            identifiers: vec![ProbeIdentifier::new("nhs", "9434765919").unwrap()],
            ..IdentityProbe::default()
        };
        let same_value_other_scheme = IdentityProbe {
            identifiers: vec![ProbeIdentifier::new("ssn", "9434765919").unwrap()],
            ..IdentityProbe::default()
        };
        let other_value_same_scheme = IdentityProbe {
            identifiers: vec![ProbeIdentifier::new("nhs", "1112223333").unwrap()],
            ..IdentityProbe::default()
        };

        assert!(!compare_identity(&a, &same_value_other_scheme).identifier_match);
        assert!(!compare_identity(&a, &other_value_same_scheme).identifier_match);
    }

    /// Matching name + DOB with no shared identifier scores meaningfully
    /// high, but strictly below the identifier-match ceiling — the
    /// deterministic short-circuit must dominate the probabilistic path.
    #[test]
    fn matching_name_and_dob_scores_high_but_below_identifier_ceiling() {
        let a = IdentityProbe {
            name: Some(name("Ncube", "Thandiwe")),
            birth_date: Some(dob(1992, 3, 4)),
            gender: Some(Gender::Female),
            identifiers: vec![],
        };
        let b = IdentityProbe {
            name: Some(name("Ncube", "Thandiwe")),
            birth_date: Some(dob(1992, 3, 4)),
            gender: Some(Gender::Female),
            identifiers: vec![],
        };

        let score = compare_identity(&a, &b);
        assert!(!score.identifier_match);
        assert!(score.confidence > 0.9);
        assert!(score.confidence < IDENTIFIER_MATCH_CEILING);
    }

    /// Unrelated records — different name, different DOB, different
    /// gender — score low.
    #[test]
    fn unrelated_records_score_low() {
        let a = IdentityProbe {
            name: Some(name("Smith", "Jane")),
            birth_date: Some(dob(1980, 6, 15)),
            gender: Some(Gender::Female),
            identifiers: vec![],
        };
        let b = IdentityProbe {
            name: Some(name("Ivanov", "Dmitri")),
            birth_date: Some(dob(1955, 11, 2)),
            gender: Some(Gender::Male),
            identifiers: vec![],
        };

        let score = compare_identity(&a, &b);
        assert!(!score.identifier_match);
        assert!(score.confidence < 0.3);
    }

    /// A pair identical in name and DOB but differing in gender is not
    /// penalized as harshly as a genuine name/DOB mismatch — gender is
    /// the weakest of the three weighted components.
    #[test]
    fn gender_only_mismatch_is_a_soft_penalty() {
        let a = IdentityProbe {
            name: Some(name("Okafor", "Chinwe")),
            birth_date: Some(dob(1990, 1, 1)),
            gender: Some(Gender::Female),
            identifiers: vec![],
        };
        let gender_mismatch = IdentityProbe {
            gender: Some(Gender::Male),
            ..a.clone()
        };
        // A genuinely different person: unrelated name, same DOB/gender —
        // deliberately a very different name (not a near-miss spelling
        // variant) so the contrast with a mere gender flip is unambiguous
        // regardless of the exact Jaro-Winkler value.
        let name_mismatch = IdentityProbe {
            name: Some(name("Zubrowski", "Marguerite")),
            birth_date: Some(dob(1990, 1, 1)),
            gender: Some(Gender::Female),
            identifiers: vec![],
        };

        let perfect_score = compare_identity(&a, &a.clone()).confidence;
        let gender_mismatch_score = compare_identity(&a, &gender_mismatch).confidence;
        let harsher_score = compare_identity(&a, &name_mismatch).confidence;

        // Gender mismatch alone costs at most GENDER_WEIGHT (scaled by
        // PROBABILISTIC_CEILING) off a perfect match.
        assert!(gender_mismatch_score >= perfect_score - GENDER_WEIGHT - f64::EPSILON);
        // A name mismatch (with DOB and gender still agreeing) costs more
        // than a gender-only mismatch, since NAME_WEIGHT > GENDER_WEIGHT.
        assert!(gender_mismatch_score > harsher_score);
    }

    /// Two genders both `Unknown` score as a soft `0.5`, not a hard `0.0` —
    /// per the documented table, "either is Unknown" is distinct from a
    /// genuine mismatch.
    #[test]
    fn both_genders_unknown_scores_as_soft_ambiguity() {
        let a = IdentityProbe {
            gender: Some(Gender::Unknown),
            ..IdentityProbe::default()
        };
        let b = IdentityProbe {
            gender: Some(Gender::Unknown),
            ..IdentityProbe::default()
        };
        let score = compare_identity(&a, &b);
        assert_eq!(score.gender_score, Some(0.5));
    }

    /// A missing DOB on one side excludes the DOB component rather than
    /// scoring it as a mismatch — and the function never panics on the
    /// absence.
    #[test]
    fn missing_dob_on_one_side_excludes_the_component() {
        let a = IdentityProbe {
            name: Some(name("Haddad", "Layla")),
            birth_date: Some(dob(1975, 9, 20)),
            gender: None,
            identifiers: vec![],
        };
        let b = IdentityProbe {
            name: Some(name("Haddad", "Layla")),
            birth_date: None,
            gender: None,
            identifiers: vec![],
        };

        let score = compare_identity(&a, &b);
        assert_eq!(score.dob_score, None);
        assert_eq!(score.gender_score, None);
        // Still bounded and non-spurious: name alone carries the whole
        // weighted average, so it lands at (close to) the name score.
        assert!(score.confidence > 0.0);
        assert!(score.confidence <= 1.0);
    }

    /// Both sides entirely empty (no name, no DOB, no gender, no
    /// identifiers) scores `0.0`, never a spurious match on shared
    /// absence (`agents/share/security.md` invariant 4).
    #[test]
    fn wholly_absent_fields_never_manufacture_a_match() {
        let a = IdentityProbe::default();
        let b = IdentityProbe::default();

        let score = compare_identity(&a, &b);
        assert!(!score.identifier_match);
        assert_eq!(score.name_score, None);
        assert_eq!(score.dob_score, None);
        assert_eq!(score.gender_score, None);
        assert!(score.confidence.abs() < f64::EPSILON);
    }

    /// Two blank (but present) names never spuriously match, even though
    /// `Scorer::jaro_winkler_similarity("", "")` is documented as `1.0` in
    /// isolation — the probe-level guard excludes blank sub-components
    /// before the scorer ever sees them.
    #[test]
    fn blank_names_do_not_spuriously_match() {
        let a = IdentityProbe {
            name: Some(name("", "")),
            ..IdentityProbe::default()
        };
        let b = IdentityProbe {
            name: Some(name("", "")),
            ..IdentityProbe::default()
        };

        let score = compare_identity(&a, &b);
        assert_eq!(score.name_score, None);
        assert!(score.confidence.abs() < f64::EPSILON);
    }

    /// A blank identifier value is never constructed, so it can never
    /// stand in for "no identifier" and spuriously agree with another
    /// blank identifier.
    #[test]
    fn blank_identifier_is_rejected_at_construction() {
        assert_eq!(ProbeIdentifier::new("nhs", "   "), None);
        assert_eq!(ProbeIdentifier::new("   ", "9434765919"), None);
        assert_eq!(ProbeIdentifier::new("", ""), None);
    }

    // ---------- score_dob_pair table ----------

    #[test]
    fn dob_exact_match() {
        assert!((score_dob_pair(dob(1995, 1, 10), dob(1995, 1, 10)) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn dob_day_off_by_one_or_two() {
        assert!((score_dob_pair(dob(1995, 1, 10), dob(1995, 1, 11)) - 0.95).abs() < f64::EPSILON);
        assert!((score_dob_pair(dob(1995, 1, 10), dob(1995, 1, 12)) - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn dob_month_day_transposition() {
        assert!((score_dob_pair(dob(1995, 1, 10), dob(1995, 10, 1)) - 0.90).abs() < f64::EPSILON);
    }

    #[test]
    fn dob_same_year_and_month() {
        assert!((score_dob_pair(dob(1995, 1, 10), dob(1995, 1, 20)) - 0.80).abs() < f64::EPSILON);
    }

    #[test]
    fn dob_year_off_by_one() {
        assert!((score_dob_pair(dob(1995, 1, 10), dob(1996, 1, 10)) - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn dob_same_year_only() {
        assert!((score_dob_pair(dob(1995, 1, 10), dob(1995, 6, 25)) - 0.50).abs() < f64::EPSILON);
    }

    #[test]
    fn dob_unrelated_years() {
        assert!(score_dob_pair(dob(1995, 1, 10), dob(1980, 6, 30)).abs() < f64::EPSILON);
    }

    // ---------- T-30: candidate blocking ----------

    use entity_ref::EntityType;
    use uuid::Uuid;

    fn entity_ref(entity_type: EntityType, id: u128) -> EntityRef {
        EntityRef::new(entity_type, Uuid::from_u128(id))
    }

    fn identifier(scheme: &str, value: &str) -> ProbeIdentifier {
        ProbeIdentifier::new(scheme, value).unwrap()
    }

    /// A person sharing an exact coded identifier with the worker is
    /// found and scored at the identifier ceiling; a second person with
    /// no identifier and an unrelated phonetic block is never compared
    /// to the worker at all (different block namespace entirely: the
    /// worker's identifier-present probe never gets a phonetic block).
    #[test]
    fn identifier_block_finds_the_sharing_pair_and_excludes_the_other_person() {
        let sharing_person = IdentityProbe {
            name: Some(name("Smith", "Jane")),
            birth_date: Some(dob(1980, 6, 15)),
            gender: Some(Gender::Female),
            identifiers: vec![identifier("nhs", "9434765919")],
        };
        let unrelated_person = IdentityProbe {
            name: Some(name("Zubrowski", "Marguerite")),
            birth_date: Some(dob(2001, 11, 2)),
            gender: Some(Gender::Male),
            identifiers: vec![],
        };
        let worker = IdentityProbe {
            name: Some(name("Smith", "Jane")),
            birth_date: Some(dob(1980, 6, 15)),
            gender: Some(Gender::Female),
            identifiers: vec![identifier("nhs", "9434765919")],
        };

        let sharing_person_ref = entity_ref(EntityType::Person, 1);
        let unrelated_person_ref = entity_ref(EntityType::Person, 2);
        let worker_ref = entity_ref(EntityType::Worker, 3);

        let candidates = generate_candidates(
            &[
                (sharing_person_ref, sharing_person),
                (unrelated_person_ref, unrelated_person),
            ],
            &[(worker_ref, worker)],
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].person, sharing_person_ref);
        assert_eq!(candidates[0].worker, worker_ref);
        assert!(candidates[0].score.identifier_match);
        assert!((candidates[0].score.confidence - IDENTIFIER_MATCH_CEILING).abs() < f64::EPSILON);
    }

    /// Two records with no coded identifier, similar-sounding surnames,
    /// and the same birth year land in the same `Soundex(family) +
    /// birth_year` block and get scored — the returned confidence
    /// matches a direct `compare_identity` call on the same probes.
    #[test]
    fn phonetic_and_birth_year_fallback_block_scores_the_pair() {
        let person_probe = IdentityProbe {
            name: Some(name("Smith", "Jane")),
            birth_date: Some(dob(1985, 3, 10)),
            gender: Some(Gender::Female),
            identifiers: vec![],
        };
        // "Smyth" shares "Smith"'s Soundex code (the module doc's own
        // documented example).
        let worker_probe = IdentityProbe {
            name: Some(name("Smyth", "Jane")),
            birth_date: Some(dob(1985, 3, 10)),
            gender: Some(Gender::Female),
            identifiers: vec![],
        };

        let person_ref = entity_ref(EntityType::Person, 10);
        let worker_ref = entity_ref(EntityType::Worker, 11);
        let direct = compare_identity(&person_probe, &worker_probe);

        let candidates =
            generate_candidates(&[(person_ref, person_probe)], &[(worker_ref, worker_probe)]);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].person, person_ref);
        assert_eq!(candidates[0].worker, worker_ref);
        assert!((candidates[0].score.confidence - direct.confidence).abs() < f64::EPSILON);
        assert!(candidates[0].score.confidence >= SUGGESTION_THRESHOLD);
    }

    /// A same-block pair (identical family name, so same Soundex code
    /// and same birth year) that nonetheless scores below the
    /// suggestion threshold — because the given name, month/day, and
    /// gender all disagree — is compared but discarded, never returned.
    #[test]
    fn low_scoring_same_block_pair_is_discarded() {
        let person_probe = IdentityProbe {
            name: Some(name("Smith", "Zebulon")),
            birth_date: Some(dob(1985, 1, 10)),
            gender: Some(Gender::Female),
            identifiers: vec![],
        };
        let worker_probe = IdentityProbe {
            name: Some(name("Smith", "Priya")),
            birth_date: Some(dob(1985, 8, 25)),
            gender: Some(Gender::Male),
            identifiers: vec![],
        };

        // Sanity-check the fixture actually scores below threshold
        // (same family name and birth year keep them in one block; the
        // given name / month-day / gender disagreement is what pulls
        // the score down) before trusting the blocked result.
        let direct = compare_identity(&person_probe, &worker_probe);
        assert!(direct.confidence < SUGGESTION_THRESHOLD);

        let candidates = generate_candidates(
            &[(entity_ref(EntityType::Person, 20), person_probe)],
            &[(entity_ref(EntityType::Worker, 21), worker_probe)],
        );

        assert!(candidates.is_empty());
    }

    /// The load-bearing blocking proof: a pair that would score *above*
    /// the suggestion threshold if compared directly is never even
    /// scored by `generate_candidates`, because a one-year birth-date
    /// difference puts them in different `Soundex(family) + birth_year`
    /// blocks. This demonstrates blocking is actually bounding the
    /// comparisons performed, not merely that scoring is correct.
    #[test]
    fn pairs_in_different_blocks_are_never_compared_even_when_score_would_qualify() {
        let person_probe = IdentityProbe {
            name: Some(name("Johnson", "Alice")),
            birth_date: Some(dob(1975, 4, 10)),
            gender: Some(Gender::Female),
            identifiers: vec![],
        };
        let worker_probe = IdentityProbe {
            name: Some(name("Johnson", "Alice")),
            // Same month/day, one year off — a different block (block
            // keys on the *exact* birth year), even though this is a
            // strong demographic match.
            birth_date: Some(dob(1976, 4, 10)),
            gender: Some(Gender::Female),
            identifiers: vec![],
        };

        // Prove it WOULD qualify if the comparator ever saw it.
        let direct = compare_identity(&person_probe, &worker_probe);
        assert!(direct.confidence >= SUGGESTION_THRESHOLD);
        assert!(!direct.identifier_match);

        // Yet blocking never presents this pair to the comparator at
        // all, so it never appears as a candidate.
        let candidates = generate_candidates(
            &[(entity_ref(EntityType::Person, 30), person_probe)],
            &[(entity_ref(EntityType::Worker, 31), worker_probe)],
        );

        assert!(candidates.is_empty());
    }

    /// A probe carrying more than one identifier shared with the other
    /// side is reachable through more than one identifier-block, but
    /// still produces exactly one candidate for the pair — not one per
    /// shared identifier.
    #[test]
    fn multiple_shared_identifiers_do_not_duplicate_the_candidate() {
        let person_probe = IdentityProbe {
            identifiers: vec![
                identifier("nhs", "9434765919"),
                identifier("ssn", "078-05-1120"),
            ],
            ..IdentityProbe::default()
        };
        let worker_probe = IdentityProbe {
            identifiers: vec![
                identifier("nhs", "9434765919"),
                identifier("ssn", "078-05-1120"),
            ],
            ..IdentityProbe::default()
        };

        let candidates = generate_candidates(
            &[(entity_ref(EntityType::Person, 40), person_probe)],
            &[(entity_ref(EntityType::Worker, 41), worker_probe)],
        );

        assert_eq!(candidates.len(), 1);
    }

    /// Empty inputs (either side, or both) return an empty candidate
    /// list without panicking.
    #[test]
    fn empty_inputs_never_panic() {
        let person_probe = IdentityProbe {
            identifiers: vec![identifier("nhs", "9434765919")],
            ..IdentityProbe::default()
        };
        let worker_probe = IdentityProbe {
            identifiers: vec![identifier("nhs", "9434765919")],
            ..IdentityProbe::default()
        };

        assert!(
            generate_candidates(
                &[],
                &[(entity_ref(EntityType::Worker, 50), worker_probe.clone())]
            )
            .is_empty()
        );
        assert!(
            generate_candidates(
                &[(entity_ref(EntityType::Person, 51), person_probe.clone())],
                &[]
            )
            .is_empty()
        );
        assert!(generate_candidates(&[], &[]).is_empty());
        // And the never-blocked case: neither identifier nor name/DOB.
        assert!(
            generate_candidates(
                &[(entity_ref(EntityType::Person, 52), IdentityProbe::default())],
                &[(entity_ref(EntityType::Worker, 53), IdentityProbe::default())],
            )
            .is_empty()
        );
    }
}
