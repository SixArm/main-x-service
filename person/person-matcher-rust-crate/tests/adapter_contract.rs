#![warn(clippy::pedantic)]

//! Adapter contract test for the `person-matcher` public API.
//!
//! This test pins the exact public surface that downstream `person-service`
//! (and any other consumer) depends on via its `to_matcher_person` adapter.
//! Renames to any of the symbols touched here break this test, failing the
//! matcher's own CI **before** a publish would silently break services.
//!
//! Scope:
//! - Every `PersonBuilder` method the service adapter calls.
//! - Every national-identifier slot (40+).
//! - `MatchingEngine` entry points (`default_config`, `new`,
//!   `match_persons`, `deterministic_match`, `match_one_to_many`).
//! - `MatchResult` field set (`score`, `is_match`, `confidence`,
//!   `breakdown`).
//! - `MatchBreakdown` per-identifier fields the adapter inspects.
//! - `MatchConfig` presets (`default`, `strict`, `lenient`).
//! - `Confidence` enum variants (`High`, `Medium`, `Low`).
//! - `Gender` enum variants.
//! - `Address` builder and `PassportBook::new` constructor.
//!
//! If a public symbol must change, update *this test in the same PR* — the
//! purpose is to make every breaking API change deliberate.

use person_matcher::{
    Address, Confidence, Gender, MatchConfig, MatchingEngine, PassportBook, Person, PersonBuilder,
};

// =============================================================================
// 1. PersonBuilder surface — every method the service adapter touches.
// =============================================================================

/// Pins every fluent builder method we know downstream callers invoke.
/// This is intentionally a compile-time-mostly test: if any method is renamed
/// or removed, this fails to build.
#[test]
fn person_builder_demographic_and_contact_surface() {
    let dob = jiff::civil::date(1980, 5, 15);
    let death = jiff::civil::date(2070, 1, 1);
    let addr = Address::new().with_line1("1 Test St").with_city("Town");

    let p: Person = Person::builder()
        .given_name("Alice")
        .middle_name("Q")
        .family_name("Williams")
        .date_of_birth(dob)
        .death_date(death)
        .gender(Gender::Female)
        .multiple_birth(1)
        .address(addr.clone())
        .birth_place(addr.clone())
        .death_place(addr.clone())
        .previous_addresses(vec![addr.clone()])
        .phone("+44 20 7946 0958")
        .mobile("+44 7700 900000")
        .email("alice@example.com")
        .local_id("LOCAL-1")
        .build();

    assert_eq!(p.given_name.as_deref(), Some("Alice"));
    assert_eq!(p.middle_name.as_deref(), Some("Q"));
    assert_eq!(p.family_name.as_deref(), Some("Williams"));
    assert_eq!(p.date_of_birth, Some(dob));
    assert_eq!(p.gender, Some(Gender::Female));
    assert_eq!(p.phone.as_deref(), Some("+44 20 7946 0958"));
    assert_eq!(p.email.as_deref(), Some("alice@example.com"));
    assert!(p.address.is_some());
    assert_eq!(p.previous_addresses.len(), 1);
}

/// Pins every national-identifier builder slot the adapter routes to.
/// The list MUST stay stable across `SemVer` minor releases.
#[test]
fn person_builder_national_identifier_surface() {
    // We don't care about the parsed value here — the point is the method
    // exists, takes `impl Into<String>`, and returns `PersonBuilder`.
    let _: Person = Person::builder()
        .united_kingdom_national_health_service_number("943 476 5919")
        .uk_chi_number("123")
        .uk_hc_number("123")
        .uk_nino("AB123456C")
        .us_ssn("123-45-6789")
        .fr_nir("180 06 31 075 040 71")
        .es_tsi("12345678")
        .ie_ihi("1234567")
        .au_ihi("8003600000000000")
        .de_kvnr("A123456789")
        .it_cf("RSSMRA80A01H501U")
        .nl_bsn("111222333")
        .se_personnummer("19121212-1212")
        .be_nn("12345678901")
        .bg_egn("8001010001")
        .cz_rc("8001010001")
        .dk_cpr("0101800001")
        .ee_ik("38001010001")
        .es_dni("12345678Z")
        .fi_hetu("010180-1230")
        .hr_oib("12345678903")
        .is_kt("0101801239")
        .lt_ak("38001010001")
        .lv_pk("010180-12345")
        .mt_id("0123456M")
        .no_fnr("01018012345")
        .pl_pesel("80010100015")
        .ro_cnp("1800101000001")
        .si_emso("0101980500001")
        .sk_rc("8001010001")
        .gr_dss("12345")
        .li_id("00")
        .nl_id("123")
        .pl_nip("0")
        .pt_nif("0")
        .br_cpf("12345678909")
        .cn_rrn("123")
        .in_aadhaar("234123412346")
        .jp_my_number("123456789012")
        .mx_curp("HEGJ560219HJCRRR03")
        .nz_nhi("ABC1234")
        .za_id("8001015009087")
        .build();
}

/// Pins the passport-book identity surface.
#[test]
fn person_builder_passport_book_surface() {
    // PassportBook::new is fallible; an empty country or number must yield None.
    assert!(PassportBook::new("", "X").is_none());
    assert!(PassportBook::new("US", "").is_none());

    let pb = PassportBook::new("US", "X12345678")
        .expect("non-empty country + number must construct")
        .with_issued(jiff::civil::date(2020, 1, 1))
        .with_expires(jiff::civil::date(2030, 1, 1));
    assert_eq!(pb.country, "US");
    assert_eq!(pb.number, "X12345678");

    // Builder must accept both add-one and replace-all paths.
    let p1 = Person::builder().add_passport_book(pb.clone()).build();
    assert_eq!(p1.passport_books.len(), 1);

    let p2 = Person::builder().passport_books(vec![pb]).build();
    assert_eq!(p2.passport_books.len(), 1);
}

// =============================================================================
// 2. Address builder surface — service adapter renames service fields here.
// =============================================================================

/// Pins every `Address` builder method the service adapter calls, including
/// the field-rename routes (`with_county` ← service `state`, `with_postcode`
/// ← service `postal_code`). A rename here breaks the adapter at compile time.
#[test]
fn address_builder_surface() {
    let a = Address::new()
        .with_line1("Line 1")
        .with_line2("Line 2")
        .with_city("Town")
        .with_county("Region") // service-side "state" routes here
        .with_postcode("AB1 2CD") // service-side "postal_code" routes here
        .with_country("GB");
    assert_eq!(a.line1.as_deref(), Some("Line 1"));
    assert_eq!(a.city.as_deref(), Some("Town"));
    assert_eq!(a.county.as_deref(), Some("Region"));
    assert_eq!(a.postcode.as_deref(), Some("AB1 2CD"));
    assert_eq!(a.country.as_deref(), Some("GB"));
}

// =============================================================================
// 3. MatchingEngine entry points — every constructor / match method.
// =============================================================================

/// Pins the four `MatchingEngine` construction paths the service relies on:
/// `default_config` plus `new` with the `default`/`strict`/`lenient` presets.
#[test]
fn matching_engine_constructor_surface() {
    let _: MatchingEngine = MatchingEngine::default_config();
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::default());
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::strict());
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::lenient());
}

/// Pins the `MatchResult` / `MatchBreakdown` shape the adapter reads:
/// `score` (`f64`), `is_match` (`bool`), `confidence` (`Confidence`), and the
/// per-field `Option<f64>` breakdown fields it surfaces to callers.
#[test]
fn matching_engine_match_persons_returns_match_result() {
    let a = Person::builder().given_name("A").family_name("X").build();
    let b = a.clone();
    let result = MatchingEngine::default_config().match_persons(&a, &b);
    // Field access pins the MatchResult shape.
    let _: f64 = result.score;
    let _: bool = result.is_match;
    let _: Confidence = result.confidence;
    let _ = result.breakdown.given_name_score; // Option<f64>
    let _ = result.breakdown.family_name_score;
    let _ = result.breakdown.date_of_birth_score;
    let _ = result.breakdown.gender_score;
    let _ = result.breakdown.address_score;
    let _ = result.breakdown.phone_score;
    let _ = result.breakdown.email_score;
    let _ = result
        .breakdown
        .united_kingdom_national_health_service_number_score;
    let _ = result.breakdown.us_ssn_score;
    let _ = result.breakdown.passport_book_score;
}

/// Pins that `deterministic_match` returns `bool` and that a shared `US SSN`
/// across two textual layouts triggers a deterministic hit.
#[test]
fn matching_engine_deterministic_match_returns_bool() {
    let a = Person::builder().us_ssn("111-22-3333").build();
    let b = Person::builder().us_ssn("111223333").build();
    let res: bool = MatchingEngine::default_config().deterministic_match(&a, &b);
    assert!(res, "shared US SSN must trigger deterministic match");
}

/// Pins the batch entry point `match_one_to_many`: returns one `MatchResult`
/// per candidate (length-preserving) with `f64` scores the adapter ranks on.
#[test]
fn matching_engine_match_one_to_many_returns_vec() {
    let query = Person::builder().given_name("Q").build();
    let candidates = vec![query.clone(), query.clone()];
    let results = MatchingEngine::default_config().match_one_to_many(&query, &candidates);
    assert_eq!(results.len(), candidates.len());
    let _: f64 = results[0].score;
}

// =============================================================================
// 4. Enum + config-preset variants — Confidence + MatchConfig presets.
// =============================================================================

/// Pins the `Confidence` variant set (`High`/`Medium`/`Low`) and the
/// `from_score` band boundaries (`0.95` → `High`, `0.80` → `Medium`,
/// `0.10` → `Low`) the adapter maps onto its own labels.
#[test]
fn confidence_variants_exist() {
    let _ = Confidence::High;
    let _ = Confidence::Medium;
    let _ = Confidence::Low;
    assert_eq!(Confidence::from_score(0.95), Confidence::High);
    assert_eq!(Confidence::from_score(0.80), Confidence::Medium);
    assert_eq!(Confidence::from_score(0.10), Confidence::Low);
}

/// Pins the full `Gender` variant set (`Male`/`Female`/`Other`/`Unknown`)
/// the adapter constructs — adding/removing a variant must be deliberate.
#[test]
fn gender_variants_exist() {
    // Adapter constructs all four; this pins the variant set.
    let _ = [Gender::Male, Gender::Female, Gender::Other, Gender::Unknown];
}

/// Pins that the preset thresholds form a monotonic ladder
/// (`strict >= default >= lenient`) — the service's strict-⊆-lenient
/// invariant test depends on this ordering.
#[test]
fn match_config_preset_scores_form_monotonic_threshold_ladder() {
    // Strict threshold ≥ default ≥ lenient. The adapter's strict-⊆-lenient
    // invariant test in the service depends on this.
    let strict = MatchConfig::strict().match_threshold;
    let default = MatchConfig::default().match_threshold;
    let lenient = MatchConfig::lenient().match_threshold;
    assert!(
        strict >= default && default >= lenient,
        "thresholds must form a monotonic ladder strict ≥ default ≥ lenient \
         (got strict={strict} default={default} lenient={lenient})"
    );
}

// =============================================================================
// 5. Round-trip: MatchResult must serialize + deserialize (services persist).
// =============================================================================

/// Pins that `MatchResult` survives a serde JSON round-trip without value
/// drift — services persist results as JSON, so the serialised shape is
/// part of the contract.
#[test]
fn match_result_round_trips_through_json() {
    let a = Person::builder().given_name("A").family_name("B").build();
    let b = a.clone();
    let result = MatchingEngine::default_config().match_persons(&a, &b);
    let json = serde_json::to_string(&result).expect("serialize");
    let back: person_matcher::MatchResult = serde_json::from_str(&json).expect("deserialize");
    assert!((result.score - back.score).abs() < 1e-12);
    assert_eq!(result.is_match, back.is_match);
    assert_eq!(result.confidence, back.confidence);
}

// =============================================================================
// 6. Compile-time guard: PersonBuilder is `Sized` and returnable by value.
// =============================================================================

/// Compile-time guard that `PersonBuilder` stays a `Sized` value type the
/// adapter can move through `let mut b = b.method(...)` and return by value.
#[test]
fn person_builder_is_value_type() {
    // The adapter passes builders through `let mut b = b.method(...);` and
    // returns them by value. If `PersonBuilder` ever became `!Sized` or
    // required heap allocation, this would fail to compile.
    fn _check(b: PersonBuilder) -> PersonBuilder {
        b.given_name("ok")
    }
}
