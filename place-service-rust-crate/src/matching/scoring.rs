//! Weighted match scoring: the entry point that combines every component.
//!
//! [`compute_match`] is the heart of the matching layer. It scores two
//! [`Place`] records by:
//!
//! 1. **Deterministic short-circuit** — a shared GLN pins the result to a
//!    certain 1.0 match (see [`has_gln_match`]).
//! 2. **Component scoring** — name, geo, address, place type, and identifier
//!    each yield a `[0.0, 1.0]` sub-score.
//! 3. **Adaptive weighting** — only components where *both* places carry data
//!    contribute, and the weighted sum is normalized by the participating
//!    weight (name always participates).
//! 4. **Phonetic bonus** — a `+0.05` nudge when the names sound alike
//!    (Soundex) but the score is still below the certain threshold.
//! 5. **Confidence classification** — the final score maps to a
//!    [`MatchConfidence`] band.
//!
//! # Examples
//!
//! ```
//! use place_service::models::place::Place;
//! use place_service::matching::scoring::{compute_match, MatchWeights, MatchConfidence};
//!
//! let a = Place::new("Central Park");
//! let b = Place::new("Central Park");
//! let r = compute_match(&a, &b, &MatchWeights::default());
//! assert_eq!(r.confidence, MatchConfidence::Certain);
//! ```

use crate::models::place::Place;
use super::name::name_similarity;
use super::address::address_similarity;
use super::geo::geo_similarity;
use super::identifier::{identifier_similarity, has_gln_match};
use super::phonetic::soundex_match;

/// Relative importance of each match component.
///
/// The fields need not sum to 1.0 in general — [`compute_match`] normalizes
/// by the weights of the components that participated — but
/// [`MatchWeights::default`] is tuned to sum to exactly 1.0.
#[derive(Debug, Clone)]
pub struct MatchWeights {
    /// Weight of the name similarity component (default 0.35).
    pub name: f64,
    /// Weight of the geo similarity component (default 0.25).
    pub geo: f64,
    /// Weight of the address similarity component (default 0.20).
    pub address: f64,
    /// Weight of the place-type equality component (default 0.10).
    pub place_type: f64,
    /// Weight of the identifier match component (default 0.10).
    pub identifier: f64,
}

impl Default for MatchWeights {
    /// The tuned default weights (name-heavy), summing to 1.0.
    fn default() -> Self {
        Self {
            name: 0.35,
            geo: 0.25,
            address: 0.20,
            place_type: 0.10,
            identifier: 0.10,
        }
    }
}

/// Per-component detail behind a [`MatchResult`], for auditing and display.
///
/// Every sub-score is reported even when its component did not participate in
/// the weighted total (e.g. a place with no geo), so callers can see exactly
/// what drove the overall score.
#[derive(Debug, Clone)]
pub struct MatchBreakdown {
    /// Name similarity sub-score.
    pub name_score: f64,
    /// Geo similarity sub-score (0.0 when either place lacks coordinates).
    pub geo_score: f64,
    /// Address similarity sub-score (0.0 when either place lacks an address).
    pub address_score: f64,
    /// Place-type equality sub-score: 1.0 if both present and equal, else 0.0.
    pub place_type_score: f64,
    /// Identifier match sub-score.
    pub identifier_score: f64,
    /// Whether the names share a Soundex code.
    pub phonetic_match: bool,
    /// Whether a deterministic rule (GLN) decided the result.
    pub deterministic_match: bool,
}

/// The outcome of comparing two places.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Overall confidence score in `[0.0, 1.0]`.
    pub score: f64,
    /// The [`score`](Self::score) classified into a confidence band.
    pub confidence: MatchConfidence,
    /// The per-component breakdown behind the score.
    pub breakdown: MatchBreakdown,
}

/// Qualitative confidence band derived from a numeric match score.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchConfidence {
    /// Score ≥ 0.95 — a definite match.
    Certain,
    /// Score ≥ 0.80 — a likely match.
    Probable,
    /// Score ≥ 0.60 — a potential match worth review.
    Possible,
    /// Score < 0.60 — not a match.
    Unlikely,
}

impl MatchConfidence {
    /// Classifies a numeric score into a confidence band.
    ///
    /// Thresholds: `≥ 0.95` Certain, `≥ 0.80` Probable, `≥ 0.60` Possible,
    /// otherwise Unlikely.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::matching::scoring::MatchConfidence;
    ///
    /// assert_eq!(MatchConfidence::from_score(0.95), MatchConfidence::Certain);
    /// assert_eq!(MatchConfidence::from_score(0.50), MatchConfidence::Unlikely);
    /// ```
    pub fn from_score(score: f64) -> Self {
        if score >= 0.95 { Self::Certain }
        else if score >= 0.80 { Self::Probable }
        else if score >= 0.60 { Self::Possible }
        else { Self::Unlikely }
    }
}

/// Compute the match score between two places using weighted components.
///
/// See the [module docs](self) for the full algorithm. The returned
/// [`MatchResult`] carries the overall score, its confidence band, and a
/// per-component [`MatchBreakdown`].
///
/// # Examples
///
/// ```
/// use place_service::models::place::Place;
/// use place_service::models::identifier::PlaceIdentifier;
/// use place_service::matching::scoring::{compute_match, MatchWeights};
///
/// // A shared GLN deterministically pins the score to 1.0.
/// let mut a = Place::new("Store A");
/// a.identifiers = vec![PlaceIdentifier::gln("1234567890123")];
/// let mut b = Place::new("Store B");
/// b.identifiers = vec![PlaceIdentifier::gln("1234567890123")];
/// let r = compute_match(&a, &b, &MatchWeights::default());
/// assert_eq!(r.score, 1.0);
/// assert!(r.breakdown.deterministic_match);
/// ```
pub fn compute_match(a: &Place, b: &Place, weights: &MatchWeights) -> MatchResult {
    // Deterministic: GLN match short-circuits to 1.0
    let deterministic = has_gln_match(&a.identifiers, &b.identifiers);
    if deterministic {
        return MatchResult {
            score: 1.0,
            confidence: MatchConfidence::Certain,
            breakdown: MatchBreakdown {
                name_score: 1.0,
                geo_score: 1.0,
                address_score: 1.0,
                place_type_score: 1.0,
                identifier_score: 1.0,
                phonetic_match: true,
                deterministic_match: true,
            },
        };
    }

    let name_score = name_similarity(&a.name, &b.name);

    let geo_score = match (&a.geo, &b.geo) {
        (Some(ga), Some(gb)) => geo_similarity(ga, gb),
        _ => 0.0,
    };

    let address_score = match (&a.address, &b.address) {
        (Some(aa), Some(ab)) => address_similarity(aa, ab),
        _ => 0.0,
    };

    let place_type_score = match (&a.place_type, &b.place_type) {
        (Some(ta), Some(tb)) => if ta == tb { 1.0 } else { 0.0 },
        _ => 0.0,
    };

    let identifier_score = identifier_similarity(&a.identifiers, &b.identifiers);

    let phonetic = soundex_match(&a.name, &b.name);

    let mut total = 0.0;
    let mut weight_sum = 0.0;

    total += weights.name * name_score;
    weight_sum += weights.name;

    if a.geo.is_some() && b.geo.is_some() {
        total += weights.geo * geo_score;
        weight_sum += weights.geo;
    }
    if a.address.is_some() && b.address.is_some() {
        total += weights.address * address_score;
        weight_sum += weights.address;
    }
    if a.place_type.is_some() && b.place_type.is_some() {
        total += weights.place_type * place_type_score;
        weight_sum += weights.place_type;
    }
    if !a.identifiers.is_empty() && !b.identifiers.is_empty() {
        total += weights.identifier * identifier_score;
        weight_sum += weights.identifier;
    }

    let score = if weight_sum > 0.0 { total / weight_sum } else { 0.0 };

    // Phonetic bonus: +5% if names sound alike but scored below 0.95
    let score = if phonetic && score < 0.95 {
        (score + 0.05).min(1.0)
    } else {
        score
    };

    MatchResult {
        confidence: MatchConfidence::from_score(score),
        score,
        breakdown: MatchBreakdown {
            name_score,
            geo_score,
            address_score,
            place_type_score,
            identifier_score,
            phonetic_match: phonetic,
            deterministic_match: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::address::PostalAddress;
    use crate::models::geo::GeoCoordinates;
    use crate::models::identifier::PlaceIdentifier;
    use crate::models::place_type::PlaceType;

    /// A fully-populated `Central Park` fixture (type, address, geo) reused
    /// across the scorer tests.
    fn central_park() -> Place {
        let mut p = Place::new("Central Park");
        p.place_type = Some(PlaceType::Park);
        p.address = Some(PostalAddress {
            street_address: Some("14 E 60th St".into()),
            address_locality: Some("New York".into()),
            address_region: Some("NY".into()),
            address_country: Some("US".into()),
            postal_code: Some("10022".into()),
        });
        p.geo = Some(GeoCoordinates::new(40.7829, -73.9654));
        p
    }

    /// Two fully-populated identical places score Certain.
    #[test]
    fn test_identical_places_high_score() {
        let a = central_park();
        let b = central_park();
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score > 0.95, "Score: {}", result.score);
        assert_eq!(result.confidence, MatchConfidence::Certain);
    }

    /// With only names present, an exact name match still scores Certain
    /// (name always participates).
    #[test]
    fn test_name_only_match() {
        let a = Place::new("Central Park");
        let b = Place::new("Central Park");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score > 0.95, "Score: {}", result.score);
    }

    /// Two unrelated places (different name, type, geo, address) score Unlikely.
    #[test]
    fn test_different_places_low_score() {
        let a = central_park();
        let mut b = Place::new("Buckingham Palace");
        b.place_type = Some(PlaceType::CivicStructure);
        b.geo = Some(GeoCoordinates::new(51.5014, -0.1419));
        b.address = Some(PostalAddress {
            street_address: Some("London".into()),
            address_locality: Some("London".into()),
            address_region: None,
            address_country: Some("GB".into()),
            postal_code: Some("SW1A 1AA".into()),
        });
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score < 0.3, "Score: {}", result.score);
        assert_eq!(result.confidence, MatchConfidence::Unlikely);
    }

    /// A shared GLN deterministically pins the score to 1.0 despite mismatched
    /// names.
    #[test]
    fn test_gln_deterministic_match() {
        let mut a = Place::new("Store A");
        a.identifiers = vec![PlaceIdentifier::gln("1234567890123")];
        let mut b = Place::new("Store B");
        b.identifiers = vec![PlaceIdentifier::gln("1234567890123")];
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!((result.score - 1.0).abs() < f64::EPSILON);
        assert!(result.breakdown.deterministic_match);
    }

    /// Each confidence band maps from a representative score.
    #[test]
    fn test_match_confidence_levels() {
        assert_eq!(MatchConfidence::from_score(0.99), MatchConfidence::Certain);
        assert_eq!(MatchConfidence::from_score(0.85), MatchConfidence::Probable);
        assert_eq!(MatchConfidence::from_score(0.70), MatchConfidence::Possible);
        assert_eq!(MatchConfidence::from_score(0.40), MatchConfidence::Unlikely);
    }

    /// The tuned default weights sum to exactly 1.0.
    #[test]
    fn test_default_weights_sum_to_one() {
        let w = MatchWeights::default();
        let sum = w.name + w.geo + w.address + w.place_type + w.identifier;
        assert!((sum - 1.0).abs() < f64::EPSILON);
    }

    /// A single-character name typo still scores Probable or better.
    #[test]
    fn test_fuzzy_name_match() {
        let a = Place::new("Central Park");
        let b = Place::new("Centrl Park");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score > 0.8, "Score: {}", result.score);
    }

    /// Sounds-alike misspelled names report a phonetic match in the breakdown.
    #[test]
    fn test_phonetic_bonus() {
        let a = Place::new("Springfield");
        let b = Place::new("Springfeild");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.breakdown.phonetic_match);
    }
}
