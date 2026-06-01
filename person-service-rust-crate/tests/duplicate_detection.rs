//! End-to-end duplicate-detection integration tests for the
//! person-service ↔ person-matcher bridge.
//!
//! Every test:
//!   1. Builds one or two service-side `Person` records (FHIR-shaped:
//!      `HumanName`, `Vec<Identifier>`, `Vec<Address>`, `Vec<ContactPoint>`,
//!      `Vec<IdentityDocument>`).
//!   2. Projects them through [`adapter::to_matcher_person`].
//!   3. Runs the canonical `person_matcher::MatchingEngine` and asserts on
//!      the resulting `MatchResult { score, is_match, confidence, breakdown }`.
//!
//! The bridge is the contract under test — these assertions pin both the
//! adapter's field-routing rules **and** the matcher's scoring behaviour
//! against the service's domain model. If either side breaks the contract,
//! a test here will fire.

use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use person_service::matching::adapter::to_matcher_person;
use person_service::matching::matcher_lib::{Confidence, MatchConfig, MatchingEngine};
use person_service::models::{
    Address, AddressUse, ContactPoint, ContactPointSystem, ContactPointUse, DocumentType, Gender,
    HumanName, Identifier, IdentifierType, IdentityDocument, Person,
};

// -------- builders -----------------------------------------------------------

fn human_name(family: &str, given: &str) -> HumanName {
    HumanName {
        use_type: None,
        family: family.into(),
        given: vec![given.into()],
        prefix: vec![],
        suffix: vec![],
    }
}

fn person(family: &str, given: &str) -> Person {
    let mut p = Person::new(human_name(family, given), Gender::Female);
    p.id = Uuid::new_v4();
    p.created_at = Utc::now();
    p.updated_at = p.created_at;
    p
}

fn person_with_dob(family: &str, given: &str, dob: NaiveDate) -> Person {
    let mut p = person(family, given);
    p.birth_date = Some(dob);
    p
}

fn telecom(system: ContactPointSystem, value: &str) -> ContactPoint {
    ContactPoint {
        system,
        value: value.into(),
        use_type: Some(ContactPointUse::Home),
    }
}

fn address(line1: &str, city: &str, state: &str, postal: &str, country: &str) -> Address {
    Address {
        use_type: Some(AddressUse::Home),
        line1: Some(line1.into()),
        line2: None,
        city: Some(city.into()),
        state: Some(state.into()),
        postal_code: Some(postal.into()),
        country: Some(country.into()),
    }
}

fn nhs_identifier(value: &str) -> Identifier {
    Identifier::new(
        IdentifierType::Other,
        "https://fhir.nhs.uk/Id/nhs-number".into(),
        value.into(),
    )
}

fn passport(country: &str, number: &str) -> IdentityDocument {
    IdentityDocument {
        document_type: DocumentType::Passport,
        number: number.into(),
        issuing_country: Some(country.into()),
        issuing_authority: None,
        issue_date: None,
        expiry_date: None,
        verified: true,
    }
}

fn engine() -> MatchingEngine {
    MatchingEngine::default_config()
}

// =============================================================================
// Identical / near-duplicate cases
// =============================================================================

#[test]
fn identical_clones_score_near_one_high_confidence() {
    let dob = NaiveDate::from_ymd_opt(1980, 5, 15).unwrap();
    let a = person_with_dob("Williams", "Alice", dob);
    let b = a.clone();

    let result = engine().match_persons(&to_matcher_person(&a), &to_matcher_person(&b));

    assert!(result.score >= 0.95, "identical clones should score ≥ 0.95, got {}", result.score);
    assert_eq!(result.confidence, Confidence::High);
    assert!(result.is_match, "identical clones must classify as match");
}

#[test]
fn typo_in_given_name_still_matches_fuzzy() {
    let dob = NaiveDate::from_ymd_opt(1980, 5, 15).unwrap();
    let alice = person_with_dob("Williams", "Alice", dob);
    let alyce = person_with_dob("Williams", "Alyce", dob); // single-letter Jaro-Winkler typo

    let result = engine().match_persons(&to_matcher_person(&alice), &to_matcher_person(&alyce));

    assert!(
        result.score >= 0.85,
        "Alice/Alyce one-letter typo should fuzzy-match at ≥ 0.85, got {}",
        result.score
    );
    assert!(result.is_match, "fuzzy typo match should classify as match");
}

#[test]
fn off_by_one_day_dob_softly_penalised_not_dropped() {
    let alice_a = person_with_dob("Williams", "Alice", NaiveDate::from_ymd_opt(1980, 5, 15).unwrap());
    let alice_b = person_with_dob("Williams", "Alice", NaiveDate::from_ymd_opt(1980, 5, 16).unwrap());

    let exact = engine().match_persons(
        &to_matcher_person(&alice_a),
        &to_matcher_person(&alice_a),
    );
    let off_by_one =
        engine().match_persons(&to_matcher_person(&alice_a), &to_matcher_person(&alice_b));

    // Off-by-one is a typical data-entry slip; the matcher applies a real
    // penalty (off-by-one isn't a perfect match) but should still flag the
    // pair as worth reviewing. The exact-DOB case should score strictly
    // higher than the off-by-one case.
    assert!(
        off_by_one.score >= 0.50,
        "off-by-one day should still be a review-worthy candidate (≥ 0.50), got {}",
        off_by_one.score
    );
    assert!(
        exact.score > off_by_one.score,
        "exact DOB must score higher than off-by-one DOB: exact={} off_by_one={}",
        exact.score,
        off_by_one.score
    );
}

// =============================================================================
// Deterministic short-circuits — national identifiers
// =============================================================================

#[test]
fn shared_nhs_number_with_different_formatting_is_deterministic_match() {
    // Two records with the same UK NHS number but in different textual layouts.
    // The matcher canonicalises the number before equality so the
    // deterministic-match path should fire.
    let mut a = person("Smith", "John");
    a.identifiers.push(nhs_identifier("943 476 5919"));
    let mut b = person("Smyth", "Jon"); // intentionally different names
    b.identifiers.push(nhs_identifier("9434765919")); // no spaces

    let ma = to_matcher_person(&a);
    let mb = to_matcher_person(&b);

    assert!(
        engine().deterministic_match(&ma, &mb),
        "shared NHS number must trigger deterministic_match regardless of name divergence"
    );

    let result = engine().match_persons(&ma, &mb);
    assert_eq!(
        result.breakdown.uk_nhs_number_score,
        Some(1.0),
        "NHS number breakdown should score 1.0"
    );
    assert!(
        result.score >= 0.80,
        "shared NHS number should drive overall score ≥ 0.80, got {}",
        result.score
    );
}

#[test]
fn shared_tax_id_default_routes_to_us_ssn() {
    // Service-side `tax_id` is the shortcut field; the adapter routes it to
    // the matcher's `us_ssn` slot by default.
    let mut a = person("Smith", "John");
    a.tax_id = Some("123-45-6789".into());
    let mut b = person("Smith", "John");
    b.tax_id = Some("123456789".into()); // different formatting, same digits

    let result = engine().match_persons(&to_matcher_person(&a), &to_matcher_person(&b));

    assert!(
        result.score >= 0.90,
        "shared tax_id (US SSN) should drive overall score ≥ 0.90, got {}",
        result.score
    );
    assert_eq!(result.confidence, Confidence::High);
}

#[test]
fn matching_passport_books_short_circuit() {
    // Passport identity is identified by (issuing_country, number); same
    // (country, number) pair on each side is a hard duplicate even when
    // demographic data drifts.
    let mut a = person("Smith", "Jonathan");
    a.documents.push(passport("US", "X12345678"));
    let mut b = person("Smith", "Jon"); // nickname divergence
    b.documents.push(passport("US", "X12345678"));

    let ma = to_matcher_person(&a);
    let mb = to_matcher_person(&b);

    assert_eq!(ma.passport_books.len(), 1);
    assert_eq!(ma.passport_books[0].country, "US");

    let result = engine().match_persons(&ma, &mb);
    assert!(
        result.score >= 0.90,
        "shared passport book should produce ≥ 0.90, got {}",
        result.score
    );
    assert!(result.is_match);
}

// =============================================================================
// Negative cases
// =============================================================================

#[test]
fn completely_different_persons_score_low_and_do_not_match() {
    let a = person_with_dob(
        "Williams",
        "Alice",
        NaiveDate::from_ymd_opt(1980, 5, 15).unwrap(),
    );
    let b = person_with_dob(
        "Tanaka",
        "Hiroshi",
        NaiveDate::from_ymd_opt(1953, 11, 2).unwrap(),
    );

    let result = engine().match_persons(&to_matcher_person(&a), &to_matcher_person(&b));

    assert!(
        result.score < 0.70,
        "unrelated persons should score < 0.70, got {}",
        result.score
    );
    assert!(!result.is_match);
    assert_ne!(result.confidence, Confidence::High);
}

#[test]
fn same_name_different_dob_and_no_shared_identifier_does_not_short_circuit() {
    // Common name like John Smith with no shared ID and different DOB must
    // not auto-match — false-positive prevention.
    let a = person_with_dob("Smith", "John", NaiveDate::from_ymd_opt(1955, 1, 1).unwrap());
    let b = person_with_dob("Smith", "John", NaiveDate::from_ymd_opt(1990, 12, 31).unwrap());

    let result = engine().match_persons(&to_matcher_person(&a), &to_matcher_person(&b));

    // Names match, gender matches, but DOB is wildly different — score must
    // not reach the strict-match band.
    assert!(
        result.score < 0.90,
        "same name + far-apart DOB should not hit High band, got {}",
        result.score
    );
}

// =============================================================================
// Field-routing pinning (regression guards for the adapter)
// =============================================================================

#[test]
fn first_phone_email_telecom_make_it_into_matcher() {
    let mut a = person("Smith", "John");
    a.telecom.push(telecom(ContactPointSystem::Phone, "+44 20 7946 0958"));
    a.telecom.push(telecom(ContactPointSystem::Email, "john@example.com"));
    a.telecom.push(telecom(ContactPointSystem::Phone, "+44 20 7946 0001")); // second phone ignored

    let m = to_matcher_person(&a);
    assert_eq!(m.phone.as_deref(), Some("+44 20 7946 0958"));
    assert_eq!(m.email.as_deref(), Some("john@example.com"));
}

#[test]
fn first_address_becomes_address_rest_become_previous() {
    let mut a = person("Smith", "John");
    a.addresses.push(address("1 Old Lane", "York", "ENG", "YO1 7HG", "GB"));
    a.addresses.push(address("99 New St", "Leeds", "ENG", "LS1 4DT", "GB"));

    let m = to_matcher_person(&a);
    assert!(m.address.is_some());
    assert_eq!(m.address.as_ref().unwrap().city.as_deref(), Some("York"));
    assert_eq!(m.previous_addresses.len(), 1);
    assert_eq!(
        m.previous_addresses[0].city.as_deref(),
        Some("Leeds"),
        "second address should land in previous_addresses"
    );
}

#[test]
fn state_renames_to_county_in_matcher_address() {
    // Service uses FHIR's "state"; matcher uses British "county". The adapter
    // must rename rather than drop.
    let mut a = person("Smith", "John");
    a.addresses.push(address(
        "10 Downing St",
        "London",
        "Greater London",
        "SW1A 2AA",
        "GB",
    ));
    let m = to_matcher_person(&a);
    assert_eq!(
        m.address.as_ref().unwrap().county.as_deref(),
        Some("Greater London"),
        "service `state` must map to matcher `county`"
    );
    assert_eq!(
        m.address.as_ref().unwrap().postcode.as_deref(),
        Some("SW1A 2AA"),
        "service `postal_code` must map to matcher `postcode`"
    );
}

// =============================================================================
// Edge cases — sparse records, type-routed identifiers
// =============================================================================

#[test]
fn sparse_records_do_not_panic_and_score_in_range() {
    // Sparse records (one nearly empty, one filled, no shared identifier or
    // name) must not panic. The matcher's missing-field renormalization can
    // legitimately produce a high score when *only* the gender field
    // participates and matches — that is a known limitation, not a bug —
    // so we assert on safety properties (no panic, score ∈ [0, 1]) rather
    // than on the match outcome.
    let mut a = person("", "");
    a.gender = Gender::Male;
    let mut b = person("Brown", "Charlie");
    b.gender = Gender::Female;

    let result = engine().match_persons(&to_matcher_person(&a), &to_matcher_person(&b));
    assert!(result.score >= 0.0 && result.score <= 1.0);
    // With mismatched gender and no overlapping identity fields, the pair
    // should not classify as a duplicate.
    assert!(
        !result.is_match,
        "sparse pair with mismatched gender should not match, got score {}",
        result.score
    );
}

#[test]
fn typed_ssn_identifier_with_us_system_uri_routes_to_us_ssn() {
    // When the service-side identifier carries the FHIR-style US SSN system
    // URI, the adapter must route to `us_ssn` regardless of tax_id.
    let mut a = person("Smith", "John");
    a.identifiers.push(Identifier::new(
        IdentifierType::SSN,
        "http://hl7.org/fhir/sid/us-ssn".into(),
        "111-22-3333".into(),
    ));
    let mut b = person("Smyth", "Jon");
    b.identifiers.push(Identifier::new(
        IdentifierType::SSN,
        "http://hl7.org/fhir/sid/us-ssn".into(),
        "111-22-3333".into(),
    ));

    let result = engine().match_persons(&to_matcher_person(&a), &to_matcher_person(&b));
    assert_eq!(
        result.breakdown.us_ssn_score,
        Some(1.0),
        "matching SSNs should produce us_ssn_score = 1.0"
    );
}

// =============================================================================
// Config presets — strict vs lenient threshold behaviour
// =============================================================================

#[test]
fn strict_config_demands_more_evidence_than_lenient_for_same_pair() {
    // A near-duplicate that hits the default match threshold may or may not
    // hit the strict threshold; either way, strict ≥ lenient on the
    // `is_match` boolean transition.
    let a = person_with_dob(
        "Garcia",
        "Maria",
        NaiveDate::from_ymd_opt(1972, 3, 8).unwrap(),
    );
    let mut b = a.clone();
    b.name.given[0] = "Mária".into(); // diacritic variation
    b.id = Uuid::new_v4(); // distinct record id

    let ma = to_matcher_person(&a);
    let mb = to_matcher_person(&b);

    let lenient = MatchingEngine::new(MatchConfig::lenient()).match_persons(&ma, &mb);
    let strict = MatchingEngine::new(MatchConfig::strict()).match_persons(&ma, &mb);

    // The raw probabilistic score is the same — only the threshold differs.
    assert!((lenient.score - strict.score).abs() < 1e-9);

    // Lenient classifies the diacritic variant as a match; strict may or may
    // not depending on the threshold, but never classifies *more* than
    // lenient.
    if strict.is_match {
        assert!(
            lenient.is_match,
            "strict matches must be a subset of lenient matches"
        );
    }
    // Confirm lenient definitely catches the near-clone.
    assert!(lenient.is_match, "lenient should match a diacritic variant");
}
