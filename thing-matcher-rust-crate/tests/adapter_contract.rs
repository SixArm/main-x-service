//! Adapter contract test for the `thing-matcher` public API.
//!
//! Pins the public surface that downstream `thing-service` depends on via
//! its `to_matcher_thing` adapter.

use thing_matcher::{Confidence, Identifier, MatchConfig, MatchingEngine, Thing, ThingBuilder};

// =============================================================================
// 1. ThingBuilder surface
// =============================================================================

#[test]
fn thing_builder_full_surface() {
    let isbn = Identifier::new("isbn", "9780141439518").unwrap();

    let t: Thing = Thing::builder()
        .name("Pride and Prejudice")
        .add_alternate_name("First Impressions")
        .alternate_names(vec!["A Novel".into()])
        .description("A novel of manners")
        .disambiguating_description("Jane Austen, 1813")
        .add_identifier(isbn.clone())
        .identifiers(vec![isbn])
        .url("https://example.com")
        .image("https://example.com/cover.jpg")
        .same_as(vec!["https://wikidata.org/Q170583".into()])
        .add_same_as("https://openlibrary.org/works/OL1394865W")
        .main_entity_of_page("https://en.wikipedia.org/wiki/Pride_and_Prejudice")
        .additional_types(vec!["https://schema.org/Book".into()])
        .add_additional_type("https://schema.org/CreativeWork")
        .subject_of(vec!["https://example.com/about".into()])
        .add_subject_of("https://example.com/review")
        .owner("Penguin")
        .local_id("LOCAL-1")
        .build();

    assert_eq!(t.name.as_deref(), Some("Pride and Prejudice"));
    assert_eq!(t.identifiers.len(), 1);
    assert_eq!(t.identifiers[0].property_id, "isbn");
    assert!(!t.same_as.is_empty());
    assert!(!t.additional_types.is_empty());
}

// =============================================================================
// 2. Identifier::new validation surface
// =============================================================================

#[test]
fn identifier_constructor_surface() {
    // Empty value → None (trimmed).
    assert!(Identifier::new("isbn", "").is_none());
    assert!(Identifier::new("isbn", "   ").is_none());
    // Non-empty → Some.
    let id = Identifier::new("doi", "10.1038/nature12373").unwrap();
    assert_eq!(id.property_id, "doi");
    assert_eq!(id.value, "10.1038/nature12373");
}

// =============================================================================
// 3. MatchingEngine entry points
// =============================================================================

#[test]
fn matching_engine_constructor_surface() {
    let _: MatchingEngine = MatchingEngine::default_config();
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::default());
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::strict());
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::lenient());
}

#[test]
fn matching_engine_match_things_returns_match_result() {
    let a = Thing::builder().name("X").build();
    let b = a.clone();
    let result = MatchingEngine::default_config().match_things(&a, &b);
    let _: f64 = result.score;
    let _: bool = result.is_match;
    let _: Confidence = result.confidence;
    let _ = result.breakdown.name_score;
    let _ = result.breakdown.name_phonetic_score;
    let _ = result.breakdown.description_score;
    let _ = result.breakdown.disambiguating_description_score;
    let _ = result.breakdown.identifiers_score;
    let _ = result.breakdown.url_score;
    let _ = result.breakdown.same_as_score;
    let _ = result.breakdown.image_score;
    let _ = result.breakdown.main_entity_of_page_score;
    let _ = result.breakdown.additional_types_score;
}

#[test]
fn matching_engine_deterministic_match_returns_bool() {
    let id = Identifier::new("isbn", "9780141439518").unwrap();
    let a = Thing::builder().name("A").add_identifier(id.clone()).build();
    let b = Thing::builder().name("B").add_identifier(id).build();
    let res: bool = MatchingEngine::default_config().deterministic_match(&a, &b);
    assert!(res, "shared identifier must trigger deterministic match");
}

#[test]
fn matching_engine_match_one_to_many_returns_vec() {
    let query = Thing::builder().name("Q").build();
    let candidates = vec![query.clone(), query.clone()];
    let results = MatchingEngine::default_config().match_one_to_many(&query, &candidates);
    assert_eq!(results.len(), candidates.len());
}

// =============================================================================
// 4. Confidence + config + round-trip
// =============================================================================

#[test]
fn confidence_variants_exist() {
    let _ = [Confidence::High, Confidence::Medium, Confidence::Low];
}

#[test]
fn match_config_preset_scores_form_monotonic_threshold_ladder() {
    let strict = MatchConfig::strict().match_threshold;
    let default = MatchConfig::default().match_threshold;
    let lenient = MatchConfig::lenient().match_threshold;
    assert!(strict >= default && default >= lenient);
}

#[test]
fn match_result_round_trips_through_json() {
    let a = Thing::builder().name("X").build();
    let result = MatchingEngine::default_config().match_things(&a, &a);
    let json = serde_json::to_string(&result).expect("serialize");
    let _: thing_matcher::MatchResult = serde_json::from_str(&json).expect("deserialize");
}

#[test]
fn thing_builder_is_value_type() {
    fn _check(b: ThingBuilder) -> ThingBuilder {
        b.name("ok")
    }
}
