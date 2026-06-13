#![warn(clippy::pedantic)]

//! Adapter contract test for the `worker-matcher` public API.
//!
//! Pins the public surface that downstream `worker-service` depends on via
//! its `to_matcher_worker` adapter. Renaming any symbol here breaks this
//! test, failing the matcher's own CI before a publish would silently
//! break services.
//!
//! Notable worker-vs-person divergence: the UK NHS Number slot is the
//! short-form `uk_nhs_number` (not the person matcher's
//! `united_kingdom_national_health_service_number`). The contract test
//! pins the worker form so a rename in either direction trips CI.

use worker_matcher::{
    Address, Confidence, Gender, MatchConfig, MatchingEngine, PassportBook, Worker, WorkerBuilder,
};

// =============================================================================
// 1. WorkerBuilder surface
// =============================================================================

#[test]
fn worker_builder_demographic_and_contact_surface() {
    let dob = jiff::civil::date(1970, 4, 1);
    let death = jiff::civil::date(2060, 1, 1);
    let addr = Address::new().with_line1("1 Test St").with_city("Town");

    let w: Worker = Worker::builder()
        .given_name("Asha")
        .middle_name("R")
        .family_name("Patel")
        .date_of_birth(dob)
        .death_date(death)
        .gender(Gender::Female)
        .multiple_birth(1)
        .address(addr.clone())
        .birth_place(addr.clone())
        .death_place(addr.clone())
        .previous_addresses(vec![addr.clone()])
        .phone("+1-415-555-0100")
        .mobile("+1-415-555-0101")
        .email("asha@example.com")
        .local_id("LOCAL-1")
        .build();

    assert_eq!(w.given_name.as_deref(), Some("Asha"));
    assert_eq!(w.family_name.as_deref(), Some("Patel"));
    assert_eq!(w.date_of_birth, Some(dob));
    assert_eq!(w.gender, Some(Gender::Female));
}

#[test]
fn worker_builder_national_identifier_surface() {
    let _: Worker = Worker::builder()
        .uk_nhs_number("943 476 5919") // worker-matcher uses the SHORT form
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

#[test]
fn worker_builder_passport_book_surface() {
    assert!(PassportBook::new("", "X").is_none());
    assert!(PassportBook::new("US", "").is_none());

    let pb = PassportBook::new("US", "X12345678")
        .expect("non-empty country + number must construct")
        .with_issued(jiff::civil::date(2020, 1, 1))
        .with_expires(jiff::civil::date(2030, 1, 1));
    assert_eq!(pb.country, "US");
    assert_eq!(pb.number, "X12345678");

    let w1 = Worker::builder().add_passport_book(pb.clone()).build();
    assert_eq!(w1.passport_books.len(), 1);
    let w2 = Worker::builder().passport_books(vec![pb]).build();
    assert_eq!(w2.passport_books.len(), 1);
}

// =============================================================================
// 2. Address builder surface
// =============================================================================

#[test]
fn address_builder_surface() {
    let a = Address::new()
        .with_line1("Line 1")
        .with_line2("Line 2")
        .with_city("Town")
        .with_county("Region")
        .with_postcode("AB1 2CD")
        .with_country("GB");
    assert_eq!(a.line1.as_deref(), Some("Line 1"));
    assert_eq!(a.city.as_deref(), Some("Town"));
    assert_eq!(a.county.as_deref(), Some("Region"));
    assert_eq!(a.postcode.as_deref(), Some("AB1 2CD"));
    assert_eq!(a.country.as_deref(), Some("GB"));
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
fn matching_engine_match_workers_returns_match_result() {
    let a = Worker::builder().given_name("A").family_name("X").build();
    let b = a.clone();
    let result = MatchingEngine::default_config().match_workers(&a, &b);
    let _: f64 = result.score;
    let _: bool = result.is_match;
    let _: Confidence = result.confidence;
    let _ = result.breakdown.given_name_score;
    let _ = result.breakdown.family_name_score;
    let _ = result.breakdown.date_of_birth_score;
    let _ = result.breakdown.gender_score;
    let _ = result.breakdown.address_score;
    let _ = result.breakdown.phone_score;
    let _ = result.breakdown.email_score;
    let _ = result.breakdown.uk_nhs_number_score; // short form
    let _ = result.breakdown.us_ssn_score;
    let _ = result.breakdown.passport_book_score;
}

#[test]
fn matching_engine_deterministic_match_returns_bool() {
    let a = Worker::builder().us_ssn("111-22-3333").build();
    let b = Worker::builder().us_ssn("111223333").build();
    let res: bool = MatchingEngine::default_config().deterministic_match(&a, &b);
    assert!(res);
}

#[test]
fn matching_engine_match_one_to_many_returns_vec() {
    let query = Worker::builder().given_name("Q").build();
    let candidates = vec![query.clone(), query.clone()];
    let results = MatchingEngine::default_config().match_one_to_many(&query, &candidates);
    assert_eq!(results.len(), candidates.len());
}

// =============================================================================
// 4. Enum + config-preset variants
// =============================================================================

#[test]
fn confidence_variants_exist() {
    let _ = [Confidence::High, Confidence::Medium, Confidence::Low];
    assert_eq!(Confidence::from_score(0.95), Confidence::High);
}

#[test]
fn gender_variants_exist() {
    let _ = [Gender::Male, Gender::Female, Gender::Other, Gender::Unknown];
}

#[test]
fn match_config_preset_scores_form_monotonic_threshold_ladder() {
    let strict = MatchConfig::strict().match_threshold;
    let default = MatchConfig::default().match_threshold;
    let lenient = MatchConfig::lenient().match_threshold;
    assert!(strict >= default && default >= lenient);
}

// =============================================================================
// 5. Round-trip + value-type guards
// =============================================================================

#[test]
fn match_result_round_trips_through_json() {
    let a = Worker::builder().given_name("A").family_name("B").build();
    let b = a.clone();
    let result = MatchingEngine::default_config().match_workers(&a, &b);
    let json = serde_json::to_string(&result).expect("serialize");
    let back: worker_matcher::MatchResult = serde_json::from_str(&json).expect("deserialize");
    assert!((result.score - back.score).abs() < 1e-12);
}

#[test]
fn worker_builder_is_value_type() {
    fn _check(b: WorkerBuilder) -> WorkerBuilder {
        b.given_name("ok")
    }
}
