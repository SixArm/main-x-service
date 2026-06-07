//! End-to-end duplicate-detection integration tests for the
//! worker-service ↔ worker-matcher bridge.
//!
//! Mirrors the person-service test suite (the two crates share an adapter
//! shape) but exercises worker-specific routing: NPI as a typed identifier,
//! ODS organisation code passthrough, the matcher's shorter `uk_nhs_number`
//! method name.

use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use worker_service::matching::adapter::to_matcher_worker;
use worker_service::matching::matcher_lib::{Confidence, MatchConfig, MatchingEngine};
use worker_service::models::{
    Address, AddressUse, ContactPoint, ContactPointSystem, ContactPointUse, DocumentType, Gender,
    HumanName, Identifier, IdentifierType, IdentityDocument, Worker,
};

// -------- builders -----------------------------------------------------------

/// Builds a minimal [`HumanName`] from a family and single given name.
fn human_name(family: &str, given: &str) -> HumanName {
    HumanName {
        use_type: None,
        family: family.into(),
        given: vec![given.into()],
        prefix: vec![],
        suffix: vec![],
    }
}

/// Builds a [`Worker`] with a fresh UUID and timestamps, defaulting to
/// [`Gender::Female`]; the base used by the other builders.
fn worker(family: &str, given: &str) -> Worker {
    let mut w = Worker::new(human_name(family, given), Gender::Female);
    w.id = Uuid::new_v4();
    w.created_at = Utc::now();
    w.updated_at = w.created_at;
    w
}

/// Like [`worker`], but also sets the birth date (most match tests need one).
fn worker_with_dob(family: &str, given: &str, dob: NaiveDate) -> Worker {
    let mut w = worker(family, given);
    w.birth_date = Some(dob);
    w
}

/// Builds a work-use [`ContactPoint`] of the given system and value.
fn telecom(system: ContactPointSystem, value: &str) -> ContactPoint {
    ContactPoint {
        system,
        value: value.into(),
        use_type: Some(ContactPointUse::Work),
    }
}

/// Builds a work-use [`Address`] with line1/city/state/postal/country set.
fn address(line1: &str, city: &str, state: &str, postal: &str, country: &str) -> Address {
    Address {
        use_type: Some(AddressUse::Work),
        line1: Some(line1.into()),
        line2: None,
        city: Some(city.into()),
        state: Some(state.into()),
        postal_code: Some(postal.into()),
        country: Some(country.into()),
    }
}

/// Builds an [`Identifier`] carrying the FHIR NHS-number system URI, used to
/// pin the adapter's routing into the matcher's `uk_nhs_number` slot.
fn nhs_identifier(value: &str) -> Identifier {
    Identifier::new(
        IdentifierType::Other,
        "https://fhir.nhs.uk/Id/nhs-number".into(),
        value.into(),
    )
}

/// Builds a verified passport [`IdentityDocument`] for the given country.
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

/// The default-config [`MatchingEngine`] shared by most tests.
fn engine() -> MatchingEngine {
    MatchingEngine::default_config()
}

// =============================================================================
// Identical / near-duplicate cases
// =============================================================================

/// Identical records score ≥ 0.95 with High confidence and match.
#[test]
fn identical_clones_score_near_one_high_confidence() {
    let dob = NaiveDate::from_ymd_opt(1970, 4, 1).unwrap();
    let a = worker_with_dob("Patel", "Asha", dob);
    let b = a.clone();

    let result = engine().match_workers(&to_matcher_worker(&a), &to_matcher_worker(&b));
    assert!(
        result.score >= 0.95,
        "identical clones should score ≥ 0.95, got {}",
        result.score
    );
    assert_eq!(result.confidence, Confidence::High);
    assert!(result.is_match);
}

/// A one-character given-name typo still fuzzy-matches (≥ 0.85).
#[test]
fn typo_in_given_name_still_matches_fuzzy() {
    let dob = NaiveDate::from_ymd_opt(1970, 4, 1).unwrap();
    let a = worker_with_dob("Patel", "Asha", dob);
    let b = worker_with_dob("Patel", "Ashaa", dob); // one-char insertion

    let result = engine().match_workers(&to_matcher_worker(&a), &to_matcher_worker(&b));
    assert!(
        result.score >= 0.85,
        "one-char Jaro-Winkler typo should fuzzy-match ≥ 0.85, got {}",
        result.score
    );
    assert!(result.is_match);
}

// =============================================================================
// Deterministic short-circuits — national identifiers
// =============================================================================

/// A shared NHS number routes to `uk_nhs_number` and deterministically matches.
#[test]
fn shared_nhs_number_drives_match_via_uk_nhs_number_slot() {
    // Service-side `Identifier` with FHIR NHS system URI must route to the
    // matcher's `uk_nhs_number` slot — different method name from the
    // person matcher, same algorithm.
    let mut a = worker("Smith", "John");
    a.identifiers.push(nhs_identifier("943 476 5919"));
    let mut b = worker("Smyth", "Jon");
    b.identifiers.push(nhs_identifier("9434765919"));

    let ma = to_matcher_worker(&a);
    let mb = to_matcher_worker(&b);
    assert!(ma.uk_nhs_number.is_some(), "adapter must populate uk_nhs_number");

    assert!(
        engine().deterministic_match(&ma, &mb),
        "shared NHS number must trigger deterministic_match"
    );
    let result = engine().match_workers(&ma, &mb);
    assert_eq!(result.breakdown.uk_nhs_number_score, Some(1.0));
}

/// A shared tax id (default-routed to US SSN) drives the score ≥ 0.90.
#[test]
fn shared_tax_id_default_routes_to_us_ssn() {
    let mut a = worker("Smith", "John");
    a.tax_id = Some("123-45-6789".into());
    let mut b = worker("Smith", "John");
    b.tax_id = Some("123456789".into());

    let result = engine().match_workers(&to_matcher_worker(&a), &to_matcher_worker(&b));
    assert!(
        result.score >= 0.90,
        "shared tax_id should drive overall score ≥ 0.90, got {}",
        result.score
    );
}

/// A shared passport (type + number) short-circuits to a high score.
#[test]
fn matching_passport_books_short_circuit() {
    let mut a = worker("Smith", "Jonathan");
    a.documents.push(passport("US", "X12345678"));
    let mut b = worker("Smith", "Jon");
    b.documents.push(passport("US", "X12345678"));

    let result = engine().match_workers(&to_matcher_worker(&a), &to_matcher_worker(&b));
    assert!(
        result.score >= 0.90,
        "shared passport book should produce ≥ 0.90, got {}",
        result.score
    );
}

// =============================================================================
// Worker-specific: NPI and ODS routing
// =============================================================================

/// NPI has no matcher slot, so it is dropped; the pair still matches on
/// demographics rather than via a spurious national-id signal.
#[test]
fn npi_typed_identifier_does_not_short_circuit_through_country_slots() {
    // NPI is a US worker-professional identifier with no country-slot
    // counterpart in the matcher (yet). The adapter falls through unmapped,
    // so an NPI match should NOT trigger an NHS / SSN / etc. signal — the
    // pair must still be evaluated on demographics alone.
    let dob = NaiveDate::from_ymd_opt(1965, 8, 12).unwrap();
    let mut a = worker_with_dob("Garcia", "Maria", dob);
    a.identifiers.push(Identifier::new(
        IdentifierType::NPI,
        "http://hl7.org/fhir/sid/us-npi".into(),
        "1234567893".into(),
    ));
    let mut b = worker_with_dob("Garcia", "Maria", dob);
    b.identifiers.push(Identifier::new(
        IdentifierType::NPI,
        "http://hl7.org/fhir/sid/us-npi".into(),
        "1234567893".into(),
    ));

    let ma = to_matcher_worker(&a);
    let mb = to_matcher_worker(&b);
    // The adapter doesn't fill any matcher country slot for NPI; both
    // matcher Workers should still match strongly on demographics alone.
    let result = engine().match_workers(&ma, &mb);
    assert!(
        result.score >= 0.90,
        "identical demographics should match ≥ 0.90 even when NPI is dropped, got {}",
        result.score
    );
}

/// An ODS organisation code is dropped without panicking; matching continues
/// on the remaining fields.
#[test]
fn ods_organisation_code_falls_through_unmapped() {
    // Worker-specific ODS code has no matcher equivalent; adapter must drop
    // it silently without panicking, and matching must continue on the
    // remaining fields.
    let mut a = worker("Jones", "Sara");
    a.identifiers.push(Identifier::new(
        IdentifierType::ODS,
        "https://fhir.nhs.uk/Id/ods-organization-code".into(),
        "RXX01".into(),
    ));
    let b = worker("Jones", "Sara");

    let ma = to_matcher_worker(&a);
    let result = engine().match_workers(&ma, &to_matcher_worker(&b));
    assert!(
        result.score >= 0.80,
        "demographics-only match should still score ≥ 0.80, got {}",
        result.score
    );
}

// =============================================================================
// Negative cases
// =============================================================================

/// Unrelated workers score < 0.70 and are not flagged as a match.
#[test]
fn completely_different_workers_score_low_and_do_not_match() {
    let a = worker_with_dob("Patel", "Asha", NaiveDate::from_ymd_opt(1970, 4, 1).unwrap());
    let b = worker_with_dob(
        "Olsen",
        "Sven",
        NaiveDate::from_ymd_opt(1992, 12, 24).unwrap(),
    );

    let result = engine().match_workers(&to_matcher_worker(&a), &to_matcher_worker(&b));
    assert!(
        result.score < 0.70,
        "unrelated workers should score < 0.70, got {}",
        result.score
    );
    assert!(!result.is_match);
}

/// Same name but far-apart birth dates stays out of the High band (< 0.90).
#[test]
fn same_name_different_dob_does_not_short_circuit() {
    let a = worker_with_dob("Smith", "John", NaiveDate::from_ymd_opt(1960, 1, 1).unwrap());
    let b = worker_with_dob(
        "Smith",
        "John",
        NaiveDate::from_ymd_opt(1995, 12, 31).unwrap(),
    );

    let result = engine().match_workers(&to_matcher_worker(&a), &to_matcher_worker(&b));
    assert!(
        result.score < 0.90,
        "same name + far-apart DOB must not hit High band, got {}",
        result.score
    );
}

// =============================================================================
// Field-routing pinning
// =============================================================================

/// The adapter routes telecom entries to the matcher's `phone`/`email` slots.
#[test]
fn telecom_phone_email_extraction() {
    let mut a = worker("Smith", "John");
    a.telecom
        .push(telecom(ContactPointSystem::Phone, "+1-415-555-0100"));
    a.telecom
        .push(telecom(ContactPointSystem::Email, "john@example.com"));

    let m = to_matcher_worker(&a);
    assert_eq!(m.phone.as_deref(), Some("+1-415-555-0100"));
    assert_eq!(m.email.as_deref(), Some("john@example.com"));
}

/// The first address maps to `address`; the rest become `previous_addresses`.
#[test]
fn first_address_becomes_address_rest_become_previous() {
    let mut a = worker("Smith", "John");
    a.addresses
        .push(address("1 First St", "Boston", "MA", "02108", "US"));
    a.addresses
        .push(address("2 Second St", "Cambridge", "MA", "02139", "US"));

    let m = to_matcher_worker(&a);
    assert_eq!(m.address.as_ref().unwrap().city.as_deref(), Some("Boston"));
    assert_eq!(m.previous_addresses.len(), 1);
    assert_eq!(
        m.previous_addresses[0].city.as_deref(),
        Some("Cambridge")
    );
}

// =============================================================================
// Edge cases & config presets
// =============================================================================

/// Sparse/empty records score within `[0.0, 1.0]` and do not panic; a gender
/// mismatch with no overlap does not match.
#[test]
fn sparse_records_do_not_panic_and_stay_in_range() {
    let mut a = worker("", "");
    a.gender = Gender::Male;
    let mut b = worker("Doe", "Jane");
    b.gender = Gender::Female;

    let result = engine().match_workers(&to_matcher_worker(&a), &to_matcher_worker(&b));
    assert!(result.score >= 0.0 && result.score <= 1.0);
    assert!(!result.is_match, "mismatched gender + no overlap must not match");
}

/// Strict-config matches are a subset of lenient-config matches (same score,
/// stricter threshold).
#[test]
fn strict_config_matches_subset_of_lenient_config() {
    let dob = NaiveDate::from_ymd_opt(1972, 3, 8).unwrap();
    let a = worker_with_dob("Garcia", "Maria", dob);
    let mut b = a.clone();
    b.name.given[0] = "Mária".into(); // diacritic
    b.id = Uuid::new_v4();

    let ma = to_matcher_worker(&a);
    let mb = to_matcher_worker(&b);

    let lenient = MatchingEngine::new(MatchConfig::lenient()).match_workers(&ma, &mb);
    let strict = MatchingEngine::new(MatchConfig::strict()).match_workers(&ma, &mb);

    assert!((lenient.score - strict.score).abs() < 1e-9);
    if strict.is_match {
        assert!(lenient.is_match, "strict matches must be a subset of lenient matches");
    }
    assert!(lenient.is_match, "lenient should match a diacritic variant");
}
