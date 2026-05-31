use crate::models::thing::Thing;

use super::description::description_similarity;
use super::identifier::{has_deterministic_match, identifier_similarity};
use super::name::name_similarity;
use super::phonetic::soundex_match;
use super::url::{url_list_similarity, url_similarity};

#[derive(Debug, Clone)]
pub struct MatchWeights {
    pub name: f64,
    pub identifier: f64,
    pub description: f64,
    pub url: f64,
    pub same_as: f64,
}

impl Default for MatchWeights {
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

#[derive(Debug, Clone)]
pub struct MatchBreakdown {
    pub name_score: f64,
    pub identifier_score: f64,
    pub description_score: f64,
    pub url_score: f64,
    pub same_as_score: f64,
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
pub fn compute_match(a: &Thing, b: &Thing, weights: &MatchWeights) -> MatchResult {
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

    let score = if weight_sum > 0.0 { total / weight_sum } else { 0.0 };
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

    fn pride_and_prejudice() -> Thing {
        let mut t = Thing::new("Pride and Prejudice");
        t.description = Some("A novel of manners by Jane Austen".into());
        t.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
        t.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        t.same_as = vec!["https://www.wikidata.org/wiki/Q170583".into()];
        t
    }

    #[test]
    fn test_identical_things_high_score() {
        let a = pride_and_prejudice();
        let b = pride_and_prejudice();
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score >= 0.95, "Score: {}", result.score);
        assert_eq!(result.confidence, MatchConfidence::Certain);
    }

    #[test]
    fn test_name_only_match() {
        let a = Thing::new("Pride and Prejudice");
        let b = Thing::new("Pride and Prejudice");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score > 0.95, "Score: {}", result.score);
    }

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
        let sum = w.name + w.identifier + w.description + w.url + w.same_as;
        assert!((sum - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_fuzzy_name_match() {
        let a = Thing::new("Pride and Prejudice");
        let b = Thing::new("Prde and Prejudice");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.score > 0.85, "Score: {}", result.score);
    }

    #[test]
    fn test_phonetic_bonus_applied() {
        let a = Thing::new("Springfield");
        let b = Thing::new("Springfeild");
        let result = compute_match(&a, &b, &MatchWeights::default());
        assert!(result.breakdown.phonetic_match);
    }
}
