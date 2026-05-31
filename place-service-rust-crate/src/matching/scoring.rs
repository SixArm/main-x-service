use crate::models::place::Place;
use super::name::name_similarity;
use super::address::address_similarity;
use super::geo::geo_similarity;
use super::identifier::{identifier_similarity, has_gln_match};
use super::phonetic::soundex_match;

#[derive(Debug, Clone)]
pub struct MatchWeights {
    pub name: f64,
    pub geo: f64,
    pub address: f64,
    pub place_type: f64,
    pub identifier: f64,
}

impl Default for MatchWeights {
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

#[derive(Debug, Clone)]
pub struct MatchBreakdown {
    pub name_score: f64,
    pub geo_score: f64,
    pub address_score: f64,
    pub place_type_score: f64,
    pub identifier_score: f64,
    pub phonetic_match: bool,
    pub deterministic_match: bool,
}

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub score: f64,
    pub confidence: MatchConfidence,
    pub breakdown: MatchBreakdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchConfidence {
    Certain,
    Probable,
    Possible,
    Unlikely,
}

impl MatchConfidence {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.95 { Self::Certain }
        else if score >= 0.80 { Self::Probable }
        else if score >= 0.60 { Self::Possible }
        else { Self::Unlikely }
    }
}

/// Compute match score between two places using weighted components.
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

    #[test]
    fn test_identical_places_high_score() {
        let a = central_park();
        let b = central_park();
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score > 0.95, "Score: {}", result.score);
        assert_eq!(result.confidence, MatchConfidence::Certain);
    }

    #[test]
    fn test_name_only_match() {
        let a = Place::new("Central Park");
        let b = Place::new("Central Park");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score > 0.95, "Score: {}", result.score);
    }

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

    #[test]
    fn test_match_confidence_levels() {
        assert_eq!(MatchConfidence::from_score(0.99), MatchConfidence::Certain);
        assert_eq!(MatchConfidence::from_score(0.85), MatchConfidence::Probable);
        assert_eq!(MatchConfidence::from_score(0.70), MatchConfidence::Possible);
        assert_eq!(MatchConfidence::from_score(0.40), MatchConfidence::Unlikely);
    }

    #[test]
    fn test_default_weights_sum_to_one() {
        let w = MatchWeights::default();
        let sum = w.name + w.geo + w.address + w.place_type + w.identifier;
        assert!((sum - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_fuzzy_name_match() {
        let a = Place::new("Central Park");
        let b = Place::new("Centrl Park");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score > 0.8, "Score: {}", result.score);
    }

    #[test]
    fn test_phonetic_bonus() {
        let a = Place::new("Springfield");
        let b = Place::new("Springfeild");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.breakdown.phonetic_match);
    }
}
