//! Weighted scoring engine that combines the per-component similarity
//! functions into a single [`MatchResult`](crate::matching::scoring::MatchResult)
//! for two [`Thing`](crate::models::thing::Thing) records.
//!
//! [`compute_match`](crate::matching::scoring::compute_match) is the public
//! entry point. It implements the matching pipeline:
//!
//! 1. **Deterministic short-circuit** — if the records share a globally-unique
//!    identifier
//!    ([`has_deterministic_match`](crate::matching::identifier::has_deterministic_match)),
//!    return 1.0 immediately.
//! 2. **Component scores** — name, identifier, description, url, same_as.
//! 3. **Weighted average** over only the components for which *both* records
//!    have data (so a missing field neither helps nor hurts).
//! 4. **Phonetic bonus** — +0.05 if the names share a Soundex code and the
//!    base score is below 0.95.
//! 5. **Confidence classification** via
//!    [`MatchConfidence::from_score`](crate::matching::scoring::MatchConfidence::from_score).
//!
//! Default weights
//! ([`MatchWeights::default`](crate::matching::scoring::MatchWeights::default))
//! are name 0.40, identifier
//! 0.30, description 0.10, url 0.10, same_as 0.10 — summing to 1.0.
//!
//! # Examples
//!
//! ```
//! use thing_service::matching::scoring::{compute_match, MatchConfidence, MatchWeights};
//! use thing_service::models::identifier::ThingIdentifier;
//! use thing_service::models::thing::Thing;
//!
//! // Different names but a shared ISBN → deterministic perfect match.
//! let mut a = Thing::new("Pride and Prejudice");
//! a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
//! let mut b = Thing::new("Stolz und Vorurteil");
//! b.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
//!
//! let result = compute_match(&a, &b, &MatchWeights::default());
//! assert_eq!(result.confidence, MatchConfidence::Certain);
//! assert!(result.breakdown.deterministic_match);
//! ```

use crate::models::thing::Thing;

use super::description::description_similarity;
use super::identifier::{has_deterministic_match, identifier_similarity};
use super::name::name_similarity;
use super::phonetic::soundex_match;
use super::url::{url_list_similarity, url_similarity};

/// Per-component weights for the weighted average in [`compute_match`].
///
/// The default set sums to 1.0, but `compute_match` re-normalizes over the
/// components actually present in both records, so callers may supply any
/// non-negative weights and still get a `[0.0, 1.0]` score.
#[derive(Debug, Clone)]
pub struct MatchWeights {
    /// Weight of the name component (default 0.40).
    pub name: f64,
    /// Weight of the identifier component (default 0.30).
    pub identifier: f64,
    /// Weight of the description component (default 0.10).
    pub description: f64,
    /// Weight of the `url` component (default 0.10).
    pub url: f64,
    /// Weight of the `same_as` cross-reference component (default 0.10).
    pub same_as: f64,
}

impl Default for MatchWeights {
    /// The standard weights: name 0.40, identifier 0.30, description 0.10,
    /// url 0.10, same_as 0.10 (sum 1.0).
    fn default() -> Self {
        Self {
            name: 0.40,
            identifier: 0.30,
            description: 0.10,
            url: 0.10,
            same_as: 0.10,
        }
    }
}

/// Per-component score breakdown returned alongside the overall score, so
/// callers (and API responses) can explain *why* two records matched.
#[derive(Debug, Clone)]
pub struct MatchBreakdown {
    /// Name similarity (Jaro-Winkler), 0.0–1.0.
    pub name_score: f64,
    /// Identifier similarity (exact pair match), 0.0 or 1.0.
    pub identifier_score: f64,
    /// Description similarity (Jaro-Winkler), 0.0–1.0.
    pub description_score: f64,
    /// `url` similarity (normalized host/path), 0.0/0.75/1.0.
    pub url_score: f64,
    /// `same_as` best-pair similarity, 0.0/0.75/1.0.
    pub same_as_score: f64,
    /// Whether the two names share a Soundex code.
    pub phonetic_match: bool,
    /// Whether a deterministic identifier match short-circuited scoring.
    pub deterministic_match: bool,
}

/// The result of comparing two [`Thing`] records.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Overall match score in `[0.0, 1.0]`.
    pub score: f64,
    /// Human-facing classification of [`score`](Self::score).
    pub confidence: MatchConfidence,
    /// Per-component breakdown explaining the score.
    pub breakdown: MatchBreakdown,
}

/// Coarse, human-facing classification of a match [`score`](MatchResult::score).
///
/// See [`MatchConfidence::from_score`] for the exact thresholds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchConfidence {
    /// Definite match — score ≥ 0.95.
    Certain,
    /// Likely match — score ≥ 0.80.
    Probable,
    /// Potential match — score ≥ 0.60.
    Possible,
    /// Not a match — score < 0.60.
    Unlikely,
}

impl MatchConfidence {
    /// Classify a `[0.0, 1.0]` score into a confidence band.
    ///
    /// Thresholds: ≥ 0.95 [`Certain`](Self::Certain), ≥ 0.80
    /// [`Probable`](Self::Probable), ≥ 0.60 [`Possible`](Self::Possible),
    /// otherwise [`Unlikely`](Self::Unlikely).
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_service::matching::scoring::MatchConfidence;
    ///
    /// assert_eq!(MatchConfidence::from_score(0.96), MatchConfidence::Certain);
    /// assert_eq!(MatchConfidence::from_score(0.50), MatchConfidence::Unlikely);
    /// ```
    pub fn from_score(score: f64) -> Self {
        if score >= 0.95 {
            Self::Certain
        } else if score >= 0.80 {
            Self::Probable
        } else if score >= 0.60 {
            Self::Possible
        } else {
            Self::Unlikely
        }
    }
}

/// Compute the match score between two Things.
///
/// Deterministic short-circuit: if both records share a globally-unique
/// identifier (DOI, ISBN, ISSN, GTIN, MPN, serial number, UUID) the
/// score is pinned at 1.0.
///
/// Otherwise: weighted average over available components (name,
/// identifier, description, url, same_as), with a +0.05 phonetic bonus
/// when the name's Soundex matches and the base score is below 0.95.
///
/// Only components for which *both* records carry data participate in the
/// average; the name component always participates (a `Thing` always has a
/// name). This keeps a missing optional field from dragging the score down
/// — absent evidence is neutral, not negative.
///
/// # Examples
///
/// ```
/// use thing_service::matching::scoring::{compute_match, MatchConfidence, MatchWeights};
/// use thing_service::models::thing::Thing;
///
/// // A single-character typo in the name still scores as a likely match.
/// let a = Thing::new("Pride and Prejudice");
/// let b = Thing::new("Prde and Prejudice");
/// let result = compute_match(&a, &b, &MatchWeights::default());
/// assert!(result.score > 0.85);
/// assert_ne!(result.confidence, MatchConfidence::Unlikely);
/// ```
pub fn compute_match(a: &Thing, b: &Thing, weights: &MatchWeights) -> MatchResult {
    // Step 1: deterministic short-circuit. A shared globally-unique
    // identifier is conclusive, so skip all fuzzy scoring and pin to 1.0.
    // The breakdown is filled with 1.0/true to make the reason explicit.
    if has_deterministic_match(&a.identifiers, &b.identifiers) {
        return MatchResult {
            score: 1.0,
            confidence: MatchConfidence::Certain,
            breakdown: MatchBreakdown {
                name_score: 1.0,
                identifier_score: 1.0,
                description_score: 1.0,
                url_score: 1.0,
                same_as_score: 1.0,
                phonetic_match: true,
                deterministic_match: true,
            },
        };
    }

    // Step 2: compute each component score independently. Optional fields
    // that are absent on either side score 0.0 here, but they are excluded
    // from the weighted average below so the 0.0 never actually counts.
    let name_score = name_similarity(&a.name, &b.name);
    let identifier_score = identifier_similarity(&a.identifiers, &b.identifiers);
    let description_score = match (&a.description, &b.description) {
        (Some(da), Some(db)) => description_similarity(da, db),
        _ => 0.0,
    };
    let url_score = match (&a.url, &b.url) {
        (Some(ua), Some(ub)) => url_similarity(ua, ub),
        _ => 0.0,
    };
    let same_as_score = url_list_similarity(&a.same_as, &b.same_as);
    let phonetic = soundex_match(&a.name, &b.name);

    // Step 3: weighted average over *present* components only. Seed the
    // running total/divisor with the always-present name component, then
    // add each optional component's weight only when both records have it.
    let mut total = weights.name * name_score;
    let mut weight_sum = weights.name;

    if !a.identifiers.is_empty() && !b.identifiers.is_empty() {
        total += weights.identifier * identifier_score;
        weight_sum += weights.identifier;
    }
    if a.description.is_some() && b.description.is_some() {
        total += weights.description * description_score;
        weight_sum += weights.description;
    }
    if a.url.is_some() && b.url.is_some() {
        total += weights.url * url_score;
        weight_sum += weights.url;
    }
    if !a.same_as.is_empty() && !b.same_as.is_empty() {
        total += weights.same_as * same_as_score;
        weight_sum += weights.same_as;
    }

    // Normalize by the summed weight of the participating components. The
    // guard handles a pathological all-zero weight set (score 0.0).
    let score = if weight_sum > 0.0 {
        total / weight_sum
    } else {
        0.0
    };
    // Step 4: phonetic bonus. A shared Soundex code nudges sub-Certain
    // scores up by 0.05 (capped at 1.0), rewarding spelling variants the
    // Jaro-Winkler name score may have under-counted.
    let score = if phonetic && score < 0.95 {
        (score + 0.05).min(1.0)
    } else {
        score
    };

    // Step 5: classify and package the result with its full breakdown.
    MatchResult {
        confidence: MatchConfidence::from_score(score),
        score,
        breakdown: MatchBreakdown {
            name_score,
            identifier_score,
            description_score,
            url_score,
            same_as_score,
            phonetic_match: phonetic,
            deterministic_match: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::identifier::ThingIdentifier;

    /// A fully-populated canonical fixture (name, description, url, ISBN,
    /// same_as) reused across the scoring tests.
    fn pride_and_prejudice() -> Thing {
        let mut t = Thing::new("Pride and Prejudice");
        t.description = Some("A novel of manners by Jane Austen".into());
        t.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
        t.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        t.same_as = vec!["https://www.wikidata.org/wiki/Q170583".into()];
        t
    }

    /// Two identical fully-populated records score Certain (≥ 0.95).
    #[test]
    fn test_identical_things_high_score() {
        let a = pride_and_prejudice();
        let b = pride_and_prejudice();
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score >= 0.95, "Score: {}", result.score);
        assert_eq!(result.confidence, MatchConfidence::Certain);
    }

    /// With only names present, an exact name match alone scores very high.
    #[test]
    fn test_name_only_match() {
        let a = Thing::new("Pride and Prejudice");
        let b = Thing::new("Pride and Prejudice");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score > 0.95, "Score: {}", result.score);
    }

    /// Two wholly unrelated records (different name/desc/url/ISBN) score low.
    #[test]
    fn test_different_things_low_score() {
        let a = pride_and_prejudice();
        let mut b = Thing::new("The Rust Programming Language");
        b.description = Some("A systems programming language by the Rust Project".into());
        b.url = Some("https://www.rust-lang.org".into());
        b.identifiers = vec![ThingIdentifier::isbn("9781718500457")];
        b.same_as = vec!["https://www.wikidata.org/wiki/Q575650".into()];

        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score < 0.5, "Score: {}", result.score);
        assert!(matches!(
            result.confidence,
            MatchConfidence::Possible | MatchConfidence::Unlikely
        ));
    }

    /// A shared ISBN short-circuits to 1.0 despite differing names.
    #[test]
    fn test_isbn_deterministic_match() {
        let mut a = Thing::new("Pride and Prejudice");
        a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        let mut b = Thing::new("Pride & Prejudice");
        b.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!((result.score - 1.0).abs() < f64::EPSILON);
        assert!(result.breakdown.deterministic_match);
    }

    /// A shared DOI short-circuits to 1.0 despite differing names.
    #[test]
    fn test_doi_deterministic_match() {
        let mut a = Thing::new("Some Paper");
        a.identifiers = vec![ThingIdentifier::doi("10.1000/xyz123")];
        let mut b = Thing::new("Some Paper (Reprint)");
        b.identifiers = vec![ThingIdentifier::doi("10.1000/xyz123")];
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!((result.score - 1.0).abs() < f64::EPSILON);
        assert!(result.breakdown.deterministic_match);
    }

    /// A shared SKU is NOT deterministic, so no short-circuit fires.
    #[test]
    fn test_sku_not_deterministic() {
        let mut a = Thing::new("Widget A");
        a.identifiers = vec![ThingIdentifier::sku("WIDGET-42")];
        let mut b = Thing::new("Widget B");
        b.identifiers = vec![ThingIdentifier::sku("WIDGET-42")];
        let result = compute_match(&a, &b, &MatchWeights::default());
        // No short-circuit: SKU isn't globally unique
        assert!(!result.breakdown.deterministic_match);
    }

    /// Each confidence band maps to a representative score.
    #[test]
    fn test_match_confidence_levels() {
        assert_eq!(MatchConfidence::from_score(0.99), MatchConfidence::Certain);
        assert_eq!(MatchConfidence::from_score(0.85), MatchConfidence::Probable);
        assert_eq!(MatchConfidence::from_score(0.70), MatchConfidence::Possible);
        assert_eq!(MatchConfidence::from_score(0.40), MatchConfidence::Unlikely);
    }

    /// Boundary pinning for the service↔matcher confidence vocabulary
    /// bridge (entity task T-8, spec §5.3 normative note).
    ///
    /// The service classifies the *raw `f64` score* with its own
    /// 4-band [`MatchConfidence`]; it never translates the embedded
    /// `thing-matcher` crate's 3-band `Confidence` label back into this
    /// vocabulary. The two band edges interleave (matcher cuts at 0.90
    /// and 0.75; service cuts at 0.95, 0.80, 0.60), so the only safe
    /// bridge is to carry the score across the adapter and re-derive
    /// here. This test pins the exact service cut points — including the
    /// matcher's interleaving edges (0.90, 0.75) — so any drift in
    /// `from_score` fails loudly.
    #[test]
    fn test_confidence_boundary_pins() {
        // Service Certain edge (inclusive).
        assert_eq!(MatchConfidence::from_score(0.95), MatchConfidence::Certain);
        // Matcher High edge falls inside service Probable.
        assert_eq!(MatchConfidence::from_score(0.90), MatchConfidence::Probable);
        // Service Probable edge (inclusive).
        assert_eq!(MatchConfidence::from_score(0.80), MatchConfidence::Probable);
        // Matcher Medium edge falls inside service Possible.
        assert_eq!(MatchConfidence::from_score(0.75), MatchConfidence::Possible);
        // Service Possible edge (inclusive).
        assert_eq!(MatchConfidence::from_score(0.60), MatchConfidence::Possible);
        // Just below the Possible edge → Unlikely.
        assert_eq!(
            MatchConfidence::from_score(0.5999),
            MatchConfidence::Unlikely
        );
    }

    /// The default weight set sums to exactly 1.0.
    #[test]
    fn test_default_weights_sum_to_one() {
        let w = MatchWeights::default();
        let sum = w.name + w.identifier + w.description + w.url + w.same_as;
        assert!((sum - 1.0).abs() < f64::EPSILON);
    }

    /// A single-character name typo still scores as a probable match.
    #[test]
    fn test_fuzzy_name_match() {
        let a = Thing::new("Pride and Prejudice");
        let b = Thing::new("Prde and Prejudice");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score > 0.85, "Score: {}", result.score);
    }

    /// Names sharing a Soundex code set the `phonetic_match` flag.
    #[test]
    fn test_phonetic_bonus_applied() {
        let a = Thing::new("Springfield");
        let b = Thing::new("Springfeild");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.breakdown.phonetic_match);
    }
}
