use thing_service::matching::description::description_similarity;
use thing_service::matching::identifier::{has_deterministic_match, identifier_similarity};
use thing_service::matching::name::name_similarity;
use thing_service::matching::phonetic::{soundex, soundex_match};
use thing_service::matching::scoring::{compute_match, MatchConfidence, MatchWeights};
use thing_service::matching::url::{url_list_similarity, url_similarity};
use thing_service::models::identifier::{IdentifierType, ThingIdentifier};
use thing_service::models::thing::Thing;

// -- Name matching edge cases --

#[test]
fn test_name_similarity_unicode() {
    let score = name_similarity("Café Society", "Cafe Society");
    assert!(score > 0.8, "Unicode name similarity: {score}");
}

#[test]
fn test_name_similarity_very_long_names() {
    let s = "The Very Long Name of an Incredibly Important Object With Many Words";
    let score = name_similarity(s, s);
    assert!((score - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_name_similarity_single_character() {
    let score = name_similarity("A", "B");
    assert!(score < 1.0);
    assert!(score >= 0.0);
}

#[test]
fn test_name_similarity_reversed_words() {
    let score = name_similarity("Prejudice and Pride", "Pride and Prejudice");
    assert!(score > 0.5, "Reversed words: {score}");
    assert!(score < 1.0);
}

// -- Description matching --

#[test]
fn test_description_similarity_exact() {
    let s = description_similarity("Same description", "Same description");
    assert!((s - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_description_similarity_different() {
    let s = description_similarity(
        "A book about software engineering.",
        "A film about underwater exploration.",
    );
    assert!(s < 0.7, "{s}");
}

// -- URL matching --

#[test]
fn test_url_similarity_identical() {
    assert_eq!(url_similarity("https://example.com", "https://example.com"), 1.0);
}

#[test]
fn test_url_similarity_scheme_insensitive() {
    assert_eq!(url_similarity("http://example.com", "https://example.com"), 1.0);
}

#[test]
fn test_url_list_best_match() {
    let a = vec!["https://example.com".to_string()];
    let b = vec![
        "https://other.com".to_string(),
        "https://example.com".to_string(),
    ];
    assert_eq!(url_list_similarity(&a, &b), 1.0);
}

// -- Identifier matching edge cases --

#[test]
fn test_identifier_similarity_multiple_matches() {
    let a = vec![
        ThingIdentifier::isbn("9780141439518"),
        ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1394865W"),
    ];
    let b = a.clone();
    let score = identifier_similarity(&a, &b);
    assert!((score - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_identifier_no_match_different_values() {
    let a = vec![ThingIdentifier::isbn("9780141439518")];
    let b = vec![ThingIdentifier::isbn("9780199536566")];
    assert_eq!(identifier_similarity(&a, &b), 0.0);
}

#[test]
fn test_deterministic_match_among_many_identifiers() {
    let a = vec![
        ThingIdentifier::sku("SKU-1"),
        ThingIdentifier::isbn("9780141439518"),
        ThingIdentifier::new(IdentifierType::Custom("Local".into()), "X1"),
    ];
    let b = vec![
        ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1394865W"),
        ThingIdentifier::isbn("9780141439518"),
    ];
    assert!(has_deterministic_match(&a, &b));
}

// -- Phonetic matching edge cases --

#[test]
fn test_soundex_codes_consistency() {
    let code1 = soundex("Programming");
    let code2 = soundex("Programming");
    assert_eq!(code1, code2);
    assert_eq!(code1.len(), 4);
}

#[test]
fn test_soundex_numeric_input() {
    let code = soundex("123");
    assert_eq!(code, "0000");
}

#[test]
fn test_soundex_match_similar_thing_names() {
    assert!(soundex_match("Steven", "Stevn"));
    assert!(soundex_match("Springfield", "Springfeild"));
}

// -- Scoring integration tests --

#[test]
fn test_match_with_custom_weights() {
    let mut a = Thing::new("Pride and Prejudice");
    a.url = Some("https://example.com/a".into());

    let mut b = Thing::new("Pride and Prejudice");
    b.url = Some("https://different.example.com/b".into());

    let name_heavy = MatchWeights {
        name: 0.92,
        identifier: 0.02,
        description: 0.02,
        url: 0.02,
        same_as: 0.02,
    };
    let url_heavy = MatchWeights {
        name: 0.10,
        identifier: 0.10,
        description: 0.10,
        url: 0.60,
        same_as: 0.10,
    };

    let result_name = compute_match(&a, &b, &name_heavy);
    let result_url = compute_match(&a, &b, &url_heavy);

    assert!(
        result_name.score > result_url.score,
        "name-heavy {:.3} should > url-heavy {:.3} when names match but URLs differ",
        result_name.score,
        result_url.score
    );
}

#[test]
fn test_match_confidence_boundaries() {
    assert_eq!(MatchConfidence::from_score(0.95), MatchConfidence::Certain);
    assert_eq!(MatchConfidence::from_score(0.949), MatchConfidence::Probable);
    assert_eq!(MatchConfidence::from_score(0.80), MatchConfidence::Probable);
    assert_eq!(MatchConfidence::from_score(0.799), MatchConfidence::Possible);
    assert_eq!(MatchConfidence::from_score(0.60), MatchConfidence::Possible);
    assert_eq!(MatchConfidence::from_score(0.599), MatchConfidence::Unlikely);
    assert_eq!(MatchConfidence::from_score(0.0), MatchConfidence::Unlikely);
    assert_eq!(MatchConfidence::from_score(1.0), MatchConfidence::Certain);
}

#[test]
fn test_match_score_always_in_range() {
    let things = [
        Thing::new(""),
        Thing::new("A"),
        Thing::new("A very long thing name that goes on and on"),
    ];
    let weights = MatchWeights::default();

    for a in &things {
        for b in &things {
            let r = compute_match(a, b, &weights);
            assert!(
                r.score >= 0.0 && r.score <= 1.0,
                "out of range: {} for {:?} vs {:?}",
                r.score,
                a.name,
                b.name
            );
        }
    }
}

#[test]
fn test_match_phonetic_bonus_applied() {
    let a = Thing::new("Springfield");
    let b = Thing::new("Springfeild");
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.breakdown.phonetic_match);
    assert!(result.score > 0.85, "with phonetic bonus: {}", result.score);
}

#[test]
fn test_match_no_phonetic_bonus_when_score_high() {
    let a = Thing::new("Pride and Prejudice");
    let b = Thing::new("Pride and Prejudice");
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.breakdown.phonetic_match);
    assert!((result.score - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_match_with_all_components() {
    let mut a = Thing::new("Pride and Prejudice");
    a.description = Some("A novel by Jane Austen.".into());
    a.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
    a.identifiers = vec![ThingIdentifier::sku("PNP-001")];
    a.same_as = vec!["https://www.wikidata.org/wiki/Q170583".into()];

    let mut b = a.clone();
    b.id = uuid::Uuid::new_v4();
    b.name = "Pride and Prejudice (1813)".into();

    let result = compute_match(&a, &b, &MatchWeights::default());

    assert!(result.breakdown.name_score > 0.5);
    assert!((result.breakdown.identifier_score - 1.0).abs() < f64::EPSILON);
    assert!((result.breakdown.description_score - 1.0).abs() < f64::EPSILON);
    assert!((result.breakdown.url_score - 1.0).abs() < f64::EPSILON);
    assert!((result.breakdown.same_as_score - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_batch_matching_sorted_by_relevance() {
    let target = Thing::new("Pride and Prejudice");
    let candidates = [
        Thing::new("Pride and Prejudice"),         // exact
        Thing::new("Pride and Prejudice and Zombies"), // partial
        Thing::new("War and Peace"),               // different
    ];
    let weights = MatchWeights::default();

    let scores: Vec<f64> = candidates
        .iter()
        .map(|c| compute_match(&target, c, &weights).score)
        .collect();

    assert!(scores[0] > scores[1], "exact > partial");
    assert!(scores[1] > scores[2], "partial > different");
}
