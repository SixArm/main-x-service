use thing_service::models::identifier::ThingIdentifier;
use thing_service::models::thing::Thing;
use thing_service::validation::{normalize_thing, validate_thing};

#[test]
fn test_validate_then_normalize_workflow() {
    let mut thing = Thing::new("  pride and prejudice  ");
    thing.url = Some("https://www.gutenberg.org/ebooks/1342".into());
    thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")];

    let errors = validate_thing(&thing);
    assert!(errors.is_empty(), "errors: {errors:?}");

    normalize_thing(&mut thing);
    assert_eq!(thing.name, "pride and prejudice");
}

#[test]
fn test_invalid_thing_does_not_normalize() {
    let mut thing = Thing::new("");
    thing.url = Some("not-a-url".into());
    thing.additional_type = Some("Book".into());

    let errors = validate_thing(&thing);
    assert!(errors.len() >= 3, "errors: {errors:?}");

    // Normalize still runs without crashing
    normalize_thing(&mut thing);
}

#[test]
fn test_full_thing_lifecycle_validation() {
    let mut thing = Thing::new("Pride and Prejudice");
    thing.description = Some("A novel of manners by Jane Austen.".into());
    thing.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
    thing.additional_type = Some("https://schema.org/Book".into());
    thing.images = vec!["https://example.com/cover.jpg".into()];
    thing.same_as = vec![
        "https://www.wikidata.org/wiki/Q170583".into(),
        "https://openlibrary.org/works/OL1394865W".into(),
    ];
    thing.identifiers = vec![
        ThingIdentifier::isbn("9780141439518"),
        ThingIdentifier::doi("10.1093/owc/9780199535569.001.0001"),
    ];

    assert!(validate_thing(&thing).is_empty());
    normalize_thing(&mut thing);
    assert!(validate_thing(&thing).is_empty());
}
