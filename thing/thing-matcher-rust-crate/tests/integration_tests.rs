#![warn(clippy::pedantic)]

//! Integration tests for thing matcher.
//!
//! These exercise the **public API** as a downstream user would — building
//! [`Thing`] records, constructing a [`MatchingEngine`], and inspecting
//! the [`thing_matcher::MatchResult`] / [`thing_matcher::MatchBreakdown`].
//!
//! Naming follows the AGENTS guide: `test_<thing>_<expected>`.

use thing_matcher::{
    Confidence, Identifier, MatchConfig, MatchingEngine, Normalizer, SimilarityAlgorithm, Thing,
};

// ============================================================
// Perfect / near-perfect matches
// ============================================================

#[test]
fn test_perfect_match_all_fields() {
    let id = Identifier::new("wikidata", "Q243").unwrap();
    let p1 = Thing::builder()
        .name("Eiffel Tower")
        .add_alternate_name("La Tour Eiffel")
        .description("Iron lattice tower on the Champ de Mars in Paris.")
        .url("https://www.toureiffel.paris/")
        .add_identifier(id)
        .add_same_as("https://www.wikidata.org/wiki/Q243")
        .add_additional_type("https://schema.org/Landmark")
        .build();
    let p2 = p1.clone();

    let result = MatchingEngine::default_config().match_things(&p1, &p2);
    assert!(result.is_match);
    assert!(result.score > 0.95);
    assert!(result.breakdown.name_score.unwrap() > 0.99);
    assert_eq!(result.breakdown.identifiers_score, Some(1.0));
    assert_eq!(result.breakdown.url_score, Some(1.0));
    assert_eq!(result.breakdown.same_as_score, Some(1.0));
}

#[test]
fn test_identical_minimal_record_scores_one() {
    let t = Thing::builder().name("Big Ben").build();
    let r = MatchingEngine::default_config().match_things(&t, &t.clone());
    assert!(r.is_match);
    assert!(r.score > 0.99);
}

// ============================================================
// Name + alternate-name matching (best of cartesian product)
// ============================================================

#[test]
fn test_name_match_picks_best_across_alternates() {
    let t1 = Thing::builder().name("Eiffel Tower").build();
    let t2 = Thing::builder()
        .name("La Tour Eiffel")
        .add_alternate_name("Eiffel Tower")
        .build();
    let r = MatchingEngine::default_config().match_things(&t1, &t2);
    let s = r.breakdown.name_score.expect("scored");
    assert!(
        s > 0.99,
        "expected best-of cartesian product to pick exact, got {s}"
    );
}

#[test]
fn test_name_match_returns_none_if_either_side_empty() {
    let t1 = Thing::builder().build();
    let t2 = Thing::builder().name("Big Ben").build();
    let r = MatchingEngine::default_config().match_things(&t1, &t2);
    assert!(r.breakdown.name_score.is_none());
}

// ============================================================
// Description scoring
// ============================================================

#[test]
fn test_description_identical_scores_one() {
    let p1 = Thing::builder()
        .name("X")
        .description("Iron tower in Paris.")
        .build();
    let p2 = Thing::builder()
        .name("X")
        .description("Iron tower in Paris.")
        .build();
    let r = MatchingEngine::default_config().match_things(&p1, &p2);
    assert!(r.breakdown.description_score.unwrap() > 0.99);
}

#[test]
fn test_description_score_diacritic_insensitive() {
    let p1 = Thing::builder()
        .name("X")
        .description("Café in the Marais.")
        .build();
    let p2 = Thing::builder()
        .name("X")
        .description("Cafe in the Marais.")
        .build();
    let r = MatchingEngine::default_config().match_things(&p1, &p2);
    assert!(r.breakdown.description_score.unwrap() > 0.99);
}

// ============================================================
// Identifier scoring (replaces place_ids)
// ============================================================

#[test]
fn test_identifiers_shared_returns_one() {
    let id = Identifier::new("wikidata", "Q243").unwrap();
    let a = Thing::builder()
        .name("Eiffel Tower")
        .add_identifier(id.clone())
        .build();
    let b = Thing::builder()
        .name("Tour Eiffel")
        .add_identifier(id)
        .build();
    let r = MatchingEngine::default_config().match_things(&a, &b);
    assert_eq!(r.breakdown.identifiers_score, Some(1.0));
}

#[test]
fn test_identifiers_property_scoped_equality() {
    let a = Thing::builder()
        .name("X")
        .add_identifier(Identifier::new("google", "X").unwrap())
        .build();
    let b = Thing::builder()
        .name("X")
        .add_identifier(Identifier::new("wikidata", "X").unwrap())
        .build();
    let r = MatchingEngine::default_config().match_things(&a, &b);
    assert_eq!(r.breakdown.identifiers_score, Some(0.0));
}

// ============================================================
// URL scoring
// ============================================================

#[test]
fn test_url_normalised_equality() {
    let a = Thing::builder()
        .name("X")
        .url("HTTPS://Example.ORG/")
        .build();
    let b = Thing::builder()
        .name("X")
        .url("https://example.org")
        .build();
    let r = MatchingEngine::default_config().match_things(&a, &b);
    assert_eq!(r.breakdown.url_score, Some(1.0));
}

#[test]
fn test_url_mismatch_scores_zero() {
    let a = Thing::builder().name("X").url("https://a.org").build();
    let b = Thing::builder().name("X").url("https://b.org").build();
    let r = MatchingEngine::default_config().match_things(&a, &b);
    assert_eq!(r.breakdown.url_score, Some(0.0));
}

// ============================================================
// sameAs Jaccard
// ============================================================

#[test]
fn test_same_as_full_overlap_is_one() {
    let a = Thing::builder()
        .name("X")
        .add_same_as("https://www.wikidata.org/wiki/Q243")
        .build();
    let b = Thing::builder()
        .name("X")
        .add_same_as("https://www.wikidata.org/wiki/Q243")
        .build();
    let r = MatchingEngine::default_config().match_things(&a, &b);
    assert_eq!(r.breakdown.same_as_score, Some(1.0));
}

#[test]
fn test_same_as_disjoint_is_zero() {
    let a = Thing::builder()
        .name("X")
        .add_same_as("https://a.org/1")
        .build();
    let b = Thing::builder()
        .name("X")
        .add_same_as("https://b.org/2")
        .build();
    let r = MatchingEngine::default_config().match_things(&a, &b);
    assert_eq!(r.breakdown.same_as_score, Some(0.0));
}

// ============================================================
// additionalType Jaccard
// ============================================================

#[test]
fn test_additional_types_shared_scores_one() {
    let a = Thing::builder()
        .name("X")
        .add_additional_type("https://schema.org/Landmark")
        .build();
    let b = Thing::builder()
        .name("X")
        .add_additional_type("https://schema.org/Landmark")
        .build();
    let r = MatchingEngine::default_config().match_things(&a, &b);
    assert_eq!(r.breakdown.additional_types_score, Some(1.0));
}

// ============================================================
// Deterministic match
// ============================================================

#[test]
fn test_deterministic_via_shared_identifier_returns_true() {
    let id = Identifier::new("wikidata", "Q243").unwrap();
    let a = Thing::builder()
        .name("Anything")
        .add_identifier(id.clone())
        .build();
    let b = Thing::builder()
        .name("Other Name")
        .add_identifier(id)
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
}

#[test]
fn test_deterministic_via_shared_same_as_returns_true() {
    let a = Thing::builder()
        .name("Eiffel Tower")
        .add_same_as("https://www.wikidata.org/wiki/Q243")
        .build();
    let b = Thing::builder()
        .name("Tour Eiffel")
        .add_same_as("https://www.wikidata.org/wiki/Q243")
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
}

#[test]
fn test_deterministic_via_canonical_url_returns_true() {
    let a = Thing::builder()
        .name("X")
        .url("https://example.org/")
        .build();
    let b = Thing::builder()
        .name("Y")
        .url("https://example.org")
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
}

#[test]
fn test_deterministic_no_signal_returns_false() {
    let a = Thing::builder().name("X").build();
    let b = Thing::builder().name("X").build();
    assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
}

// ============================================================
// Image / mainEntityOfPage
// ============================================================

#[test]
fn test_image_url_match() {
    let a = Thing::builder()
        .name("X")
        .image("https://example.org/eiffel.jpg")
        .build();
    let b = Thing::builder()
        .name("X")
        .image("HTTPS://example.org/eiffel.jpg")
        .build();
    let r = MatchingEngine::default_config().match_things(&a, &b);
    assert_eq!(r.breakdown.image_score, Some(1.0));
}

#[test]
fn test_main_entity_of_page_match() {
    let a = Thing::builder()
        .name("X")
        .main_entity_of_page("https://en.wikipedia.org/wiki/Eiffel_Tower")
        .build();
    let b = Thing::builder()
        .name("X")
        .main_entity_of_page("https://en.wikipedia.org/wiki/Eiffel_Tower")
        .build();
    let r = MatchingEngine::default_config().match_things(&a, &b);
    assert_eq!(r.breakdown.main_entity_of_page_score, Some(1.0));
}

// ============================================================
// Config presets and serde
// ============================================================

#[test]
fn test_default_config_round_trips_through_json() {
    let cfg = MatchConfig::default();
    let json = serde_json::to_string(&cfg).expect("serialise");
    let back: MatchConfig = serde_json::from_str(&json).expect("deserialise");
    assert!((cfg.match_threshold - back.match_threshold).abs() < 1e-12);
    assert!((cfg.name_weight - back.name_weight).abs() < 1e-12);
    assert!((cfg.identifiers_weight - back.identifiers_weight).abs() < 1e-12);
}

#[test]
fn test_strict_preset_requires_deterministic_match() {
    let strict = MatchingEngine::new(MatchConfig::strict());
    let id = Identifier::new("wikidata", "Q243").unwrap();
    let p1 = Thing::builder()
        .name("Eiffel Tower")
        .url("https://www.toureiffel.paris/")
        .add_identifier(id.clone())
        .build();
    let p2 = p1.clone();
    assert!(strict.match_things(&p1, &p2).is_match);
}

#[test]
fn test_lenient_preset_lowers_threshold() {
    let lenient = MatchingEngine::new(MatchConfig::lenient());
    let p1 = Thing::builder().name("Cafe Centrale").build();
    let p2 = Thing::builder().name("Cafe Central").build();
    let r = lenient.match_things(&p1, &p2);
    assert!(r.score >= 0.65);
}

#[test]
fn test_similarity_algorithm_selectable() {
    let cfg = MatchConfig {
        name_algorithm: SimilarityAlgorithm::Exact,
        ..MatchConfig::default()
    };
    let engine = MatchingEngine::new(cfg);
    let p1 = Thing::builder().name("Stephen").build();
    let p2 = Thing::builder().name("Steven").build();
    let r = engine.match_things(&p1, &p2);
    assert!(r.breakdown.name_score.unwrap() < 0.9);
}

// ============================================================
// JSON round-trip of public types
// ============================================================

#[test]
fn test_thing_with_all_fields_round_trips_through_json() {
    let t = Thing::builder()
        .name("Eiffel Tower")
        .add_alternate_name("La Tour Eiffel")
        .description("Iron tower in Paris.")
        .disambiguating_description("The one in Paris, France.")
        .url("https://www.toureiffel.paris/")
        .image("https://example.org/eiffel.jpg")
        .add_identifier(Identifier::new("wikidata", "Q243").unwrap())
        .add_same_as("https://www.wikidata.org/wiki/Q243")
        .main_entity_of_page("https://en.wikipedia.org/wiki/Eiffel_Tower")
        .add_additional_type("https://schema.org/Landmark")
        .add_subject_of("https://example.org/article")
        .owner("Société d'Exploitation de la Tour Eiffel")
        .build();
    let json = serde_json::to_string(&t).expect("serialise");
    let back: Thing = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(t, back);
}

#[test]
fn test_identifier_round_trips() {
    let id = Identifier::new("custom-scheme", "abc-123").unwrap();
    let json = serde_json::to_string(&id).expect("serialise");
    let back: Identifier = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(id, back);
}

// ============================================================
// Thing::validate
// ============================================================

#[test]
fn test_validate_requires_a_name() {
    assert!(Thing::builder().name("Big Ben").build().validate().is_ok());
    assert!(Thing::builder().build().validate().is_err());
}

// ============================================================
// Confidence band
// ============================================================

#[test]
fn test_confidence_band_high_for_exact_clone() {
    let t = Thing::builder()
        .name("Eiffel Tower")
        .url("https://www.toureiffel.paris/")
        .build();
    let r = MatchingEngine::default_config().match_things(&t, &t.clone());
    assert_eq!(r.confidence, Confidence::High);
}

// ============================================================
// Missing-field tolerance
// ============================================================

#[test]
fn test_missing_optional_fields_yield_none_in_breakdown() {
    let p1 = Thing::builder().name("X").build();
    let p2 = Thing::builder().name("X").build();
    let r = MatchingEngine::default_config().match_things(&p1, &p2);
    assert!(r.breakdown.description_score.is_none());
    assert!(r.breakdown.disambiguating_description_score.is_none());
    assert!(r.breakdown.identifiers_score.is_none());
    assert!(r.breakdown.url_score.is_none());
    assert!(r.breakdown.same_as_score.is_none());
    assert!(r.breakdown.image_score.is_none());
    assert!(r.breakdown.main_entity_of_page_score.is_none());
    assert!(r.breakdown.additional_types_score.is_none());
}

// ============================================================
// Normalizer surface
// ============================================================

#[test]
fn test_normalizer_name_strips_diacritics_and_punctuation() {
    assert_eq!(Normalizer::normalize_name("Siân"), "sian");
    assert_eq!(Normalizer::normalize_name("O'Brien"), "obrien");
}

#[test]
fn test_normalizer_url_canonicalises_scheme_and_host() {
    assert_eq!(
        Normalizer::normalize_url("HTTPS://Example.ORG/"),
        "https://example.org",
    );
}

// ============================================================
// Batch APIs
// ============================================================

#[test]
fn test_match_one_to_many_returns_one_result_per_candidate() {
    let engine = MatchingEngine::default_config();
    let q = Thing::builder().name("Eiffel Tower").build();
    let candidates = vec![
        Thing::builder().name("Eiffel Tower").build(),
        Thing::builder().name("Big Ben").build(),
    ];
    let r = engine.match_one_to_many(&q, &candidates);
    assert_eq!(r.len(), 2);
    assert!(r[0].is_match);
    assert!(!r[1].is_match);
}

#[test]
fn test_rank_one_to_many_orders_by_descending_score() {
    let engine = MatchingEngine::default_config();
    let q = Thing::builder().name("Eiffel Tower").build();
    let candidates = vec![
        Thing::builder().name("Big Ben").build(),
        q.clone(),
        Thing::builder().name("Statue of Liberty").build(),
    ];
    let ranked = engine.rank_one_to_many(&q, &candidates);
    assert_eq!(ranked[0].0, 1);
    for w in ranked.windows(2) {
        assert!(w[0].1.score >= w[1].1.score);
    }
}
