#![warn(clippy::pedantic)]

//! Integration tests for the service-side probabilistic matcher
//! ([`compute_match`]).
//!
//! These exercise the public scoring pipeline end to end against realistic
//! book/software fixtures: exact duplicates, name typos, unrelated records,
//! deterministic ISBN/DOI short-circuits, batch ranking, and `same_as`
//! contribution.

use thing_service::matching::scoring::{MatchConfidence, MatchWeights, compute_match};
use thing_service::models::identifier::ThingIdentifier;
use thing_service::models::thing::Thing;

/// Build a book-shaped [`Thing`] from optional description / ISBN / url
/// parts, for terse fixture construction in the tests below.
fn book(name: &str, description: Option<&str>, isbn: Option<&str>, url: Option<&str>) -> Thing {
    let mut t = Thing::new(name);
    t.description = description.map(String::from);
    t.url = url.map(String::from);
    if let Some(i) = isbn {
        t.identifiers = vec![ThingIdentifier::isbn(i)];
    }
    t
}

/// An exact clone scores Certain (near 1.0).
#[test]
fn test_exact_duplicate_detection() {
    let a = book(
        "Pride and Prejudice",
        Some("A novel by Jane Austen"),
        Some("9780141439518"),
        Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice"),
    );
    let b = a.clone();
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(
        result.score > 0.95,
        "Expected near-perfect, got {}",
        result.score
    );
    assert_eq!(result.confidence, MatchConfidence::Certain);
}

/// A name typo with matching description still scores as a probable match.
#[test]
fn test_typo_in_name_still_matches() {
    let a = book(
        "Pride and Prejudice",
        Some("A novel by Jane Austen"),
        None,
        None,
    );
    let b = book(
        "Prde and Prejudice",
        Some("A novel by Jane Austen"),
        None,
        None,
    );
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(
        result.score > 0.85,
        "Expected probable, got {}",
        result.score
    );
}

/// Two unrelated books score low and are not classified as a match.
#[test]
fn test_completely_different_things() {
    let a = book(
        "Pride and Prejudice",
        Some("A novel by Jane Austen"),
        Some("9780141439518"),
        Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice"),
    );
    let b = book(
        "The Rust Programming Language",
        Some("A systems language by the Rust Project"),
        Some("9781718500457"),
        Some("https://www.rust-lang.org"),
    );
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score < 0.5, "Expected low, got {}", result.score);
    assert!(matches!(
        result.confidence,
        MatchConfidence::Possible | MatchConfidence::Unlikely
    ));
}

/// Same name but diverging descriptions: name dominates, description scores lower.
#[test]
fn test_same_name_different_descriptions() {
    let a = book(
        "Programming Manual",
        Some("Concise reference for new readers."),
        None,
        None,
    );
    let b = book(
        "Programming Manual",
        Some("Exhaustive technical specification for experts."),
        None,
        None,
    );
    let result = compute_match(&a, &b, &MatchWeights::default());
    // Name matches perfectly; description diverges. Overall score stays
    // high because name dominates the weighting, but the description
    // component score is lower than the name component score.
    assert!(result.score > 0.85, "Overall: {}", result.score);
    assert!(
        result.breakdown.description_score < result.breakdown.name_score,
        "description {} should be lower than name {}",
        result.breakdown.description_score,
        result.breakdown.name_score
    );
}

/// A shared ISBN forces 1.0 even when names differ (translations).
#[test]
fn test_isbn_deterministic_overrides_name_mismatch() {
    let mut a = Thing::new("Pride and Prejudice");
    a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    let mut b = Thing::new("Stolz und Vorurteil");
    b.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!((result.score - 1.0).abs() < f64::EPSILON);
    assert!(result.breakdown.deterministic_match);
}

/// A shared DOI forces 1.0 even when names differ (preprint vs final).
#[test]
fn test_doi_deterministic_overrides_name_mismatch() {
    let mut a = Thing::new("Original paper");
    a.identifiers = vec![ThingIdentifier::doi("10.1038/nature12373")];
    let mut b = Thing::new("Preprint of same paper");
    b.identifiers = vec![ThingIdentifier::doi("10.1038/nature12373")];
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!((result.score - 1.0).abs() < f64::EPSILON);
    assert!(result.breakdown.deterministic_match);
}

/// With only names present, an exact name match scores very high.
#[test]
fn test_matching_with_name_only() {
    let a = Thing::new("The Linux Kernel");
    let b = Thing::new("The Linux Kernel");
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score > 0.95, "Score: {}", result.score);
}

/// Ranking candidates by score puts the exact match first and orders the rest.
#[test]
fn test_batch_matching_multiple_candidates() {
    let target = book(
        "Pride and Prejudice",
        Some("A novel by Jane Austen"),
        None,
        Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice"),
    );
    let candidates = [
        book(
            "Pride and Prejudice",
            Some("A novel by Jane Austen"),
            None,
            Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice"),
        ),
        book(
            "Pride and Prejudice and Zombies",
            Some("A parody by Seth Grahame-Smith"),
            None,
            None,
        ),
        book("War and Peace", Some("A novel by Leo Tolstoy"), None, None),
    ];

    let mut results: Vec<_> = candidates
        .iter()
        .map(|c| compute_match(&target, c, &MatchWeights::default()))
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    assert!(results[0].score > 0.95);
    assert!(results[1].score > results[2].score);
}

/// A shared `same_as` URL contributes a perfect same_as component score.
#[test]
fn test_same_as_url_contributes() {
    let mut a = Thing::new("Linux");
    a.same_as = vec!["https://www.wikidata.org/wiki/Q388".into()];
    let mut b = Thing::new("Linux");
    b.same_as = vec!["https://www.wikidata.org/wiki/Q388".into()];
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!((result.breakdown.same_as_score - 1.0).abs() < f64::EPSILON);
    assert!(result.score > 0.95);
}
