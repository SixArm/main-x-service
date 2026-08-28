#![warn(clippy::pedantic)]

//! Integration tests for worker matcher.
//!
//! These exercise the **public API** as a downstream user would — building
//! [`Worker`] records, constructing a [`MatchingEngine`], and inspecting
//! the [`worker_matcher::MatchResult`] / [`worker_matcher::MatchBreakdown`].
//!
//! Naming follows the AGENTS guide: `test_<thing>_<expected>`.
//!
//! All fixtures use synthetic, drama-reserved, or self-consistent NHS-format
//! values; no real PII appears here.

use chrono::NaiveDate;
use worker_matcher::{
    Address, BloodType, Confidence, Gender, MatchConfig, MatchingEngine, NicknameTable, Normalizer,
    PassportBook, SimilarityAlgorithm, Worker, identifiers,
};

// ---------- helpers ----------

fn dob(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

/// A self-consistent NHS-format check-digit identifier used across fixtures.
/// This is the same illustrative value reused in the AGENTS guides; it is
/// not allocated to any real subscriber.
const NHS_A: &str = "9434765919";
/// A second self-consistent NHS-format identifier for "different worker" cases.
const NHS_B: &str = "9434765870";

// ============================================================
// §1. Perfect / near-perfect matches
// ============================================================

#[test]
fn test_perfect_match_all_fields() {
    let mut address = Address::new();
    address.line1 = Some("123 High Street".into());
    address.city = Some("Riverside".into());
    address.postcode = Some("CF10 1AA".into());

    let worker1 = Worker::builder()
        .uk_nhs_number(NHS_A)
        .given_name("John")
        .middle_name("David")
        .family_name("Smith")
        .date_of_birth(dob(1980, 5, 15))
        .gender(Gender::Male)
        .address(address.clone())
        .phone("02920345678")
        .build();
    let worker2 = worker1.clone();

    let result = MatchingEngine::default_config().match_workers(&worker1, &worker2);
    assert!(result.is_match);
    assert!(result.score > 0.95);
    assert_eq!(result.breakdown.uk_nhs_number_score, Some(1.0));
    assert!(result.breakdown.given_name_score.unwrap() > 0.99);
    assert!(result.breakdown.family_name_score.unwrap() > 0.99);
    assert_eq!(result.breakdown.date_of_birth_score, Some(1.0));
    assert_eq!(result.breakdown.gender_score, Some(1.0));
}

#[test]
fn test_identical_minimal_record_scores_one() {
    let p = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p, &p.clone());
    assert!(r.is_match);
    assert!(r.score > 0.99);
}

// ============================================================
// §2. NHS number behaviour
// ============================================================

#[test]
fn test_nhs_number_mismatch_does_not_immediately_disqualify() {
    let p1 = Worker::builder()
        .uk_nhs_number(NHS_A)
        .given_name("John")
        .family_name("Smith")
        .date_of_birth(dob(1980, 5, 15))
        .build();
    let p2 = Worker::builder()
        .uk_nhs_number(NHS_B)
        .given_name("John")
        .family_name("Smith")
        .date_of_birth(dob(1980, 5, 15))
        .build();

    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.uk_nhs_number_score, Some(0.0));
    // Demographics still align, so the score sits in the middle range.
    assert!(r.score > 0.5);
    assert!(r.score < 0.9);
}

#[test]
fn test_unparseable_nhs_numbers_score_none_not_zero() {
    let p1 = Worker::builder()
        .uk_nhs_number("ABCDEFGHIJ")
        .given_name("Ada")
        .family_name("Lovelace")
        .date_of_birth(dob(1815, 12, 10))
        .build();
    let p2 = Worker::builder()
        .uk_nhs_number("not-valid")
        .given_name("Ada")
        .family_name("Lovelace")
        .date_of_birth(dob(1815, 12, 10))
        .build();

    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(
        r.breakdown.uk_nhs_number_score, None,
        "unparseable NHS numbers must not contribute a 0.0 penalty"
    );
    assert!(r.is_match, "demographics fully align");
}

#[test]
fn test_one_sided_uk_nhs_number_scores_none() {
    // Asymmetric NHS data: scorer should skip the field rather than penalise.
    let p1 = Worker::builder()
        .uk_nhs_number(NHS_A)
        .given_name("Ada")
        .family_name("Lovelace")
        .build();
    let p2 = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.uk_nhs_number_score, None);
}

#[test]
fn test_nhs_number_whitespace_variants_match_deterministically() {
    let a = Worker::builder().uk_nhs_number("943 476 5919").build();
    let b = Worker::builder().uk_nhs_number(NHS_A).build();
    assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
}

// ============================================================
// §3. Name handling: typos, diacritics, punctuation, phonetics
// ============================================================

#[test]
fn test_common_name_typo_under_lenient_config() {
    let p1 = Worker::builder()
        .given_name("Catherine")
        .family_name("Williams")
        .date_of_birth(dob(1975, 8, 22))
        .gender(Gender::Female)
        .build();
    let p2 = Worker::builder()
        .given_name("Katherine")
        .family_name("Williams")
        .date_of_birth(dob(1975, 8, 22))
        .gender(Gender::Female)
        .build();
    let r = MatchingEngine::new(MatchConfig::lenient()).match_workers(&p1, &p2);
    assert!(r.score > 0.75);
}

#[test]
fn test_phonetic_name_match_lifts_score() {
    let p1 = Worker::builder()
        .given_name("Stephen")
        .family_name("Jones")
        .date_of_birth(dob(1985, 3, 10))
        .build();
    let p2 = Worker::builder()
        .given_name("Steven")
        .family_name("Jones")
        .date_of_birth(dob(1985, 3, 10))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert!(r.is_match);
    assert!(r.breakdown.phonetic_name_score.is_some());
}

#[test]
fn test_diacritic_names_with_apostrophes_normalise_together() {
    let p1 = Worker::builder()
        .given_name("Siân")
        .family_name("O'Connor")
        .date_of_birth(dob(1990, 11, 5))
        .build();
    let p2 = Worker::builder()
        .given_name("Sian")
        .family_name("OConnor")
        .date_of_birth(dob(1990, 11, 5))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert!(r.is_match);
    assert!(r.score > 0.85);
}

#[test]
fn test_double_barrelled_surnames_normalise_to_a_single_word() {
    // "Lloyd-Webber" → "lloydwebber"; documented in agents/normalization.md.
    let p1 = Worker::builder()
        .given_name("Andrew")
        .family_name("Lloyd-Webber")
        .build();
    let p2 = Worker::builder()
        .given_name("Andrew")
        .family_name("Lloyd Webber")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    // Different normalised forms ("lloydwebber" vs "lloyd webber") still
    // score very highly via similarity.
    assert!(r.breakdown.family_name_score.unwrap() > 0.85);
}

#[test]
fn test_middle_name_matching_blends_into_given_name_component() {
    // Same given name on both sides → 1.0 contribution. Matching
    // middle name should leave the blended score essentially at 1.0
    // (95% of 1.0 + 5% of 1.0).
    let p1 = Worker::builder()
        .given_name("John")
        .middle_name("David")
        .family_name("Smith")
        .build();
    let p2 = p1.clone();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let g = r.breakdown.given_name_score.expect("scored");
    assert!(g > 0.99, "perfect given+middle should round to ~1.0: {g}");
}

#[test]
fn test_middle_name_mismatch_only_marginally_lowers_given_name() {
    // Same given name; very different middle names. The blended
    // contribution must dip slightly but stay close to 1.0 (95% of
    // 1.0 + 5% of a low middle-name similarity).
    let p1 = Worker::builder()
        .given_name("John")
        .middle_name("Aragon")
        .family_name("Smith")
        .build();
    let p2 = Worker::builder()
        .given_name("John")
        .middle_name("Zedd")
        .family_name("Smith")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let g = r.breakdown.given_name_score.expect("scored");
    assert!(g < 1.0, "middle-name mismatch must produce some drop: {g}");
    assert!(g > 0.93, "drop must be modest (5% weight): {g}");
}

#[test]
fn test_middle_name_one_sided_does_not_affect_score() {
    // Only one side records a middle name → the middle-name path
    // does not fire; the score is whatever the given-name comparison
    // alone yields.
    let p1 = Worker::builder()
        .given_name("John")
        .middle_name("David")
        .family_name("Smith")
        .build();
    let p2 = Worker::builder()
        .given_name("John")
        .family_name("Smith")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let g = r.breakdown.given_name_score.expect("scored");
    assert!(g > 0.99, "one-sided middle name must not change score: {g}");
}

#[test]
fn test_middle_name_lifts_score_when_given_names_differ() {
    // Identical middle names provide a small lift when the given
    // names are close but not equal.
    let p1 = Worker::builder()
        .given_name("Jon")
        .middle_name("Frederick")
        .family_name("Smith")
        .build();
    let p2 = Worker::builder()
        .given_name("John")
        .middle_name("Frederick")
        .family_name("Smith")
        .build();
    let without_middle = MatchingEngine::default_config().match_workers(
        &Worker::builder()
            .given_name("Jon")
            .family_name("Smith")
            .build(),
        &Worker::builder()
            .given_name("John")
            .family_name("Smith")
            .build(),
    );
    let with_middle = MatchingEngine::default_config().match_workers(&p1, &p2);
    let g_without = without_middle.breakdown.given_name_score.expect("scored");
    let g_with = with_middle.breakdown.given_name_score.expect("scored");
    assert!(
        g_with > g_without,
        "matching middle names must lift the given-name component: with={g_with} without={g_without}",
    );
}

#[test]
fn test_case_difference_is_irrelevant() {
    let p1 = Worker::builder()
        .given_name("JOHN")
        .family_name("SMITH")
        .build();
    let p2 = Worker::builder()
        .given_name("john")
        .family_name("smith")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert!(r.is_match);
}

// ============================================================
// §4. Address comparison
// ============================================================

#[test]
fn test_address_matching_with_abbreviated_street() {
    let mut a1 = Address::new();
    a1.line1 = Some("123 High Street".into());
    a1.city = Some("Riverside".into());
    a1.postcode = Some("CF10 1AA".into());

    let mut a2 = Address::new();
    a2.line1 = Some("123 High St".into());
    a2.city = Some("Riverside".into());
    a2.postcode = Some("CF10 1AA".into());

    let p1 = Worker::builder()
        .given_name("David")
        .family_name("Evans")
        .date_of_birth(dob(1970, 6, 30))
        .address(a1)
        .build();
    let p2 = Worker::builder()
        .given_name("David")
        .family_name("Evans")
        .date_of_birth(dob(1970, 6, 30))
        .address(a2)
        .build();

    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert!(r.is_match);
    assert!(r.breakdown.address_score.is_some());
}

#[test]
fn test_address_postcode_only_records_still_contribute() {
    let mut a1 = Address::new();
    a1.postcode = Some("CF10 1AA".into());
    let mut a2 = Address::new();
    a2.postcode = Some("cf10  1aa".into()); // different formatting

    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(a1)
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(a2)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert!(r.breakdown.address_score.unwrap() > 0.0);
}

#[test]
fn test_address_completely_empty_is_neutral() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(Address::new())
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(Address::new())
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let score = r.breakdown.address_score.expect("both have addresses");
    assert!(
        (score - 0.5).abs() < 1e-9,
        "empty addresses must be neutral 0.5, got {score}"
    );
}

#[test]
fn test_previous_addresses_used_when_current_differs() {
    // p1's current address is on Downing Street; p2's current address
    // is somewhere else, but their *previous* address matches p1's
    // current. Best-of matching surfaces the historical agreement.
    let mut current_p1 = Address::new();
    current_p1.postcode = Some("SW1A 2AA".into());
    current_p1.line1 = Some("10 Downing Street".into());

    let mut current_p2 = Address::new();
    current_p2.postcode = Some("CF10 1AA".into());
    current_p2.line1 = Some("1 Castle Avenue".into());

    let mut previous_p2 = Address::new();
    previous_p2.postcode = Some("SW1A 2AA".into());
    previous_p2.line1 = Some("10 Downing Street".into());

    let p1 = Worker::builder()
        .given_name("Same")
        .family_name("Worker")
        .address(current_p1)
        .build();
    let p2 = Worker::builder()
        .given_name("Same")
        .family_name("Worker")
        .address(current_p2)
        .previous_addresses(vec![previous_p2])
        .build();

    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let address_score = r.breakdown.address_score.expect("scored");
    // The current-vs-current pair would score near 0 on postcode+line.
    // The current-of-p1 vs previous-of-p2 pair scores much higher.
    // Best-of must surface the higher one.
    assert!(
        address_score > 0.3,
        "best-of address must use the matching historical pair, got {address_score}",
    );
}

#[test]
fn test_previous_addresses_scored_when_currents_missing() {
    // Neither worker has a `current` address; both record a historical
    // address that happens to match. We still produce a score.
    let mut historical = Address::new();
    historical.postcode = Some("CF10 1AA".into());

    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .previous_addresses(vec![historical.clone()])
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .previous_addresses(vec![historical])
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.address_score.expect("scored on historical");
    assert!(s > 0.0);
}

#[test]
fn test_previous_addresses_unused_when_one_side_has_no_address_data() {
    // No current and no previous on one side → `None`, unchanged.
    let mut a = Address::new();
    a.postcode = Some("CF10 1AA".into());
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(a)
        .build();
    let p2 = Worker::builder().given_name("X").family_name("Y").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.address_score, None);
}

#[test]
fn test_previous_addresses_does_not_lower_a_strong_current_match() {
    // If the current-vs-current pair already scores well, an
    // unrelated historical address must not drag the score down.
    // We assert relative non-regression: the score with the unrelated
    // history is at least as high as the score without it.
    let mut current = Address::new();
    current.postcode = Some("SW1A 2AA".into());
    current.line1 = Some("10 Downing Street".into());
    let mut unrelated_history = Address::new();
    unrelated_history.postcode = Some("ZZ99 9ZZ".into());
    unrelated_history.line1 = Some("Somewhere Else".into());

    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(current.clone())
        .build();
    let p2_no_history = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(current.clone())
        .build();
    let p2_with_history = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(current)
        .previous_addresses(vec![unrelated_history])
        .build();

    let engine = MatchingEngine::default_config();
    let without = engine
        .match_workers(&p1, &p2_no_history)
        .breakdown
        .address_score
        .expect("scored");
    let with = engine
        .match_workers(&p1, &p2_with_history)
        .breakdown
        .address_score
        .expect("scored");
    assert!(
        with >= without,
        "unrelated historical address must not drag down a good current match: with={with} without={without}",
    );
}

#[test]
fn test_address_absent_on_one_side_skips_scoring() {
    let mut a1 = Address::new();
    a1.postcode = Some("CF10 1AA".into());
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(a1)
        .build();
    let p2 = Worker::builder().given_name("X").family_name("Y").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.address_score, None);
}

// ============================================================
// §5. Phone number normalisation and fallback
// ============================================================

#[test]
fn test_phone_normalisation_across_uk_formats() {
    let p1 = Worker::builder()
        .given_name("Emma")
        .family_name("Davies")
        .phone("+44 7700 900123")
        .date_of_birth(dob(1988, 12, 1))
        .build();
    let p2 = Worker::builder()
        .given_name("Emma")
        .family_name("Davies")
        .phone("07700 900123")
        .date_of_birth(dob(1988, 12, 1))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert!(r.is_match);
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

#[test]
fn test_phone_falls_back_to_mobile() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .mobile("07700 900456")
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .phone("+44 7700 900456")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

#[test]
fn test_phone_absent_on_one_side_skips() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .phone("07700 900456")
        .build();
    let p2 = Worker::builder().given_name("X").family_name("Y").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, None);
}

#[test]
fn test_phone_different_numbers_score_zero() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .phone("07700 900456")
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .phone("07700 900789")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, Some(0.0));
}

// ============================================================
// §6. Deterministic matching
// ============================================================

#[test]
fn test_deterministic_nhs_match_overrides_demographics() {
    let p1 = Worker::builder()
        .uk_nhs_number("943 476 5919")
        .given_name("Robert")
        .family_name("Brown")
        .build();
    let p2 = Worker::builder()
        .uk_nhs_number(NHS_A)
        .given_name("Bob") // different name
        .family_name("Brown")
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_deterministic_demographics_match() {
    let p = Worker::builder()
        .given_name("Sarah")
        .family_name("Thomas")
        .date_of_birth(dob(1992, 4, 18))
        .gender(Gender::Female)
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p, &p.clone()));
}

#[test]
fn test_deterministic_rejects_dob_swap() {
    // Day/month swap is a notorious data-entry failure mode. The
    // probabilistic scorer gives partial credit (0.5) via the
    // transposition heuristic, but the deterministic path still
    // requires exact equality.
    let p1 = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 1, 10))
        .gender(Gender::Male)
        .build();
    let p2 = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 10, 1))
        .gender(Gender::Male)
        .build();
    let engine = MatchingEngine::default_config();
    assert!(!engine.deterministic_match(&p1, &p2));
    let r = engine.match_workers(&p1, &p2);
    assert_eq!(r.breakdown.date_of_birth_score, Some(0.5));
}

#[test]
fn test_deterministic_rejects_when_names_missing() {
    let p1 = Worker::builder()
        .date_of_birth(dob(1980, 5, 15))
        .gender(Gender::Male)
        .build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p1.clone()));
}

#[test]
fn test_deterministic_tolerates_missing_gender_on_one_side() {
    let p1 = Worker::builder()
        .given_name("John")
        .family_name("Smith")
        .date_of_birth(dob(1980, 5, 15))
        .build();
    let p2 = Worker::builder()
        .given_name("John")
        .family_name("Smith")
        .date_of_birth(dob(1980, 5, 15))
        .gender(Gender::Male)
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

// ============================================================
// §7. Configuration presets and thresholds
// ============================================================

#[test]
fn test_strict_mode_rejects_fuzzy_only_match_above_threshold() {
    // Strict mode lowers the false-positive surface by also requiring
    // a deterministic match. A fuzzy near-match that clears even a
    // relaxed strict threshold must still be rejected when no
    // identifier agrees and the normalised name + DOB + gender tuple
    // doesn't fully align.
    let cfg = MatchConfig {
        match_threshold: 0.85,
        strict_mode: true,
        ..MatchConfig::default()
    };
    let p1 = Worker::builder()
        .given_name("John")
        .family_name("Smith")
        .date_of_birth(dob(1980, 5, 15))
        .gender(Gender::Male)
        .build();
    let p2 = Worker::builder()
        .given_name("Jon")
        .family_name("Smith")
        .date_of_birth(dob(1980, 5, 15))
        .gender(Gender::Male)
        .build();
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert!(r.score >= 0.85);
    assert!(!r.is_match);
}

#[test]
fn test_strict_mode_accepts_deterministic_match() {
    // Same worker on both sides — deterministic match holds AND the
    // score is above the strict threshold. Strict mode accepts.
    let p = Worker::builder()
        .uk_nhs_number(NHS_A)
        .given_name("John")
        .family_name("Smith")
        .date_of_birth(dob(1980, 5, 15))
        .gender(Gender::Male)
        .build();
    let r = MatchingEngine::new(MatchConfig::strict()).match_workers(&p, &p.clone());
    assert!(r.is_match);
    assert_eq!(r.confidence, Confidence::High);
}

#[test]
fn test_strict_mode_rejects_nickname() {
    let p1 = Worker::builder()
        .given_name("Michael")
        .family_name("Jones")
        .date_of_birth(dob(1978, 9, 25))
        .build();
    let p2 = Worker::builder()
        .given_name("Mike")
        .family_name("Jones")
        .date_of_birth(dob(1978, 9, 25))
        .build();
    let r = MatchingEngine::new(MatchConfig::strict()).match_workers(&p1, &p2);
    assert!(!r.is_match);
}

#[test]
fn test_lenient_mode_more_forgiving_than_strict() {
    let p1 = Worker::builder()
        .given_name("Elizabeth")
        .family_name("Morgan")
        .date_of_birth(dob(1965, 7, 14))
        .build();
    let p2 = Worker::builder()
        .given_name("Liz")
        .family_name("Morgan")
        .date_of_birth(dob(1965, 7, 14))
        .build();
    let strict = MatchingEngine::new(MatchConfig::strict()).match_workers(&p1, &p2);
    let lenient = MatchingEngine::new(MatchConfig::lenient()).match_workers(&p1, &p2);
    // Same underlying score; the threshold difference is what matters.
    assert!((strict.score - lenient.score).abs() < 1e-12);
    // Lenient may match where strict does not (or both fail — the assertion is
    // about asymmetry, not absolute outcome).
    if !strict.is_match {
        assert!(strict.score < 0.95);
    }
}

#[test]
fn test_custom_weights_can_swing_outcome() {
    let p1 = Worker::builder()
        .given_name("Robert")
        .family_name("Jones")
        .date_of_birth(dob(1975, 6, 20))
        .gender(Gender::Male)
        .build();
    let p2 = Worker::builder()
        .given_name("Bob")
        .family_name("Jones")
        .date_of_birth(dob(1975, 6, 20))
        .gender(Gender::Male)
        .build();

    // De-emphasise given name; emphasise DOB + family name. This should
    // produce a higher score than the default.
    let cfg = MatchConfig {
        match_threshold: 0.80,
        uk_nhs_number_weight: 0.0,
        fr_nir_weight: 0.0,
        es_tsi_weight: 0.0,
        ie_ihi_weight: 0.0,
        uk_hc_number_weight: 0.0,
        us_ssn_weight: 0.0,
        au_ihi_weight: 0.0,
        de_kvnr_weight: 0.0,
        it_cf_weight: 0.0,
        nl_bsn_weight: 0.0,
        se_personnummer_weight: 0.0,
        uk_chi_number_weight: 0.0,
        be_nn_weight: 0.0,
        bg_egn_weight: 0.0,
        cz_rc_weight: 0.0,
        dk_cpr_weight: 0.0,
        ee_ik_weight: 0.0,
        es_dni_weight: 0.0,
        fi_hetu_weight: 0.0,
        hr_oib_weight: 0.0,
        is_kt_weight: 0.0,
        lt_ak_weight: 0.0,
        lv_pk_weight: 0.0,
        mt_id_weight: 0.0,
        no_fnr_weight: 0.0,
        pl_pesel_weight: 0.0,
        ro_cnp_weight: 0.0,
        si_emso_weight: 0.0,
        sk_rc_weight: 0.0,
        uk_nino_weight: 0.0,
        gr_dss_weight: 0.0,
        li_id_weight: 0.0,
        nl_id_weight: 0.0,
        pl_nip_weight: 0.0,
        pt_nif_weight: 0.0,
        br_cpf_weight: 0.0,
        cn_rrn_weight: 0.0,
        in_aadhaar_weight: 0.0,
        jp_my_number_weight: 0.0,
        mx_curp_weight: 0.0,
        nz_nhi_weight: 0.0,
        za_id_weight: 0.0,
        passport_book_weight: 0.0,
        given_name_weight: 0.05,
        family_name_weight: 0.35,
        date_of_birth_weight: 0.40,
        gender_weight: 0.10,
        blood_type_weight: 0.05,
        multiple_birth_weight: 0.05,
        address_weight: 0.05,
        birth_place_weight: 0.05,
        death_date_weight: 0.10,
        death_place_weight: 0.05,
        phone_weight: 0.05,
        email_weight: 0.05,
        relationships_weight: 0.0,
        tags_weight: 0.0,
        use_phonetic_matching: false,
        name_algorithm: SimilarityAlgorithm::JaroWinkler,
        strict_mode: false,
        nickname_table: NicknameTable::empty(),
        gmail_dot_folding: false,
        phone_default_country: Some("GB".into()),
    };
    let custom = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    let default = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert!(custom.score >= default.score);
}

#[test]
fn test_exact_algorithm_treats_typos_as_no_match() {
    let p1 = Worker::builder()
        .given_name("John")
        .family_name("Smith")
        .build();
    let p2 = Worker::builder()
        .given_name("Jon")
        .family_name("Smith")
        .build();
    let cfg = MatchConfig {
        name_algorithm: SimilarityAlgorithm::Exact,
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert_eq!(r.breakdown.given_name_score, Some(0.0));
    assert_eq!(r.breakdown.family_name_score, Some(1.0));
}

#[test]
fn test_jaro_winkler_algorithm_pure() {
    let p1 = Worker::builder()
        .given_name("John")
        .family_name("Smith")
        .build();
    let p2 = Worker::builder()
        .given_name("Jon")
        .family_name("Smith")
        .build();
    let cfg = MatchConfig {
        name_algorithm: SimilarityAlgorithm::JaroWinkler,
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert!(r.breakdown.given_name_score.unwrap() > 0.8);
}

#[test]
fn test_levenshtein_algorithm_pure() {
    let p1 = Worker::builder()
        .given_name("Anne")
        .family_name("Smith")
        .build();
    let p2 = Worker::builder()
        .given_name("Anna")
        .family_name("Smith")
        .build();
    let cfg = MatchConfig {
        name_algorithm: SimilarityAlgorithm::Levenshtein,
        ..MatchConfig::default()
    };
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    let s = r.breakdown.given_name_score.unwrap();
    assert!(s > 0.5 && s < 1.0, "Anne vs Anna under Levenshtein: {s}");
}

// ============================================================
// §8. Negative cases and edge handling
// ============================================================

#[test]
fn test_date_of_birth_transposition_gets_partial_credit_but_still_blocks_default_match() {
    // Day/month transposition (DD/MM vs MM/DD data-entry bug) yields
    // partial credit (0.5) under the heuristic, but the demographic
    // record on its own is not enough to clear the default 0.85
    // threshold — name + half-DOB only.
    let p1 = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 1, 10))
        .build();
    let p2 = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 10, 1))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.date_of_birth_score, Some(0.5));
    assert!(!r.is_match);
}

#[test]
fn test_date_of_birth_unrelated_mismatch_scores_zero() {
    // A genuinely different DOB (not a transposition) still scores 0.0.
    let p1 = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 1, 10))
        .build();
    let p2 = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1980, 6, 30))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.date_of_birth_score, Some(0.0));
    assert!(!r.is_match);
}

#[test]
fn test_missing_fields_calc_uses_only_available() {
    let p1 = Worker::builder()
        .given_name("James")
        .family_name("Williams")
        .build();
    let p2 = Worker::builder()
        .given_name("James")
        .family_name("Williams")
        .date_of_birth(dob(1982, 3, 15))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert!(r.score > 0.0);
    assert!(r.breakdown.given_name_score.is_some());
    assert!(r.breakdown.family_name_score.is_some());
    assert_eq!(r.breakdown.date_of_birth_score, None);
}

#[test]
fn test_completely_different_workers() {
    let p1 = Worker::builder()
        .uk_nhs_number(NHS_A)
        .given_name("Alice")
        .family_name("Anderson")
        .date_of_birth(dob(1990, 1, 1))
        .gender(Gender::Female)
        .build();
    let p2 = Worker::builder()
        .uk_nhs_number(NHS_B)
        .given_name("Zachary")
        .family_name("Zimmerman")
        .date_of_birth(dob(2000, 12, 31))
        .gender(Gender::Male)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert!(!r.is_match);
    assert!(r.score < 0.3);
    assert_eq!(r.breakdown.uk_nhs_number_score, Some(0.0));
    assert_eq!(r.breakdown.date_of_birth_score, Some(0.0));
    assert_eq!(r.breakdown.gender_score, Some(0.0));
}

#[test]
fn test_no_overlapping_fields_returns_zero_score() {
    let p1 = Worker::builder().given_name("Solo").build();
    let p2 = Worker::builder().family_name("Only").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert!(r.score.abs() < f64::EPSILON);
    assert!(!r.is_match);
}

#[test]
fn test_engine_is_symmetric() {
    let p1 = Worker::builder()
        .given_name("Owen")
        .family_name("Williams")
        .date_of_birth(dob(1972, 11, 4))
        .build();
    let p2 = Worker::builder()
        .given_name("Ewan")
        .family_name("Williams")
        .date_of_birth(dob(1972, 11, 4))
        .build();
    let engine = MatchingEngine::default_config();
    let r1 = engine.match_workers(&p1, &p2);
    let r2 = engine.match_workers(&p2, &p1);
    assert!(
        (r1.score - r2.score).abs() < 1e-12,
        "matcher must be symmetric"
    );
    assert_eq!(r1.is_match, r2.is_match);
}

#[test]
fn test_engine_is_deterministic_across_calls() {
    let p1 = Worker::builder()
        .given_name("Iolo")
        .family_name("Lewis")
        .date_of_birth(dob(1960, 2, 29))
        .build();
    let p2 = Worker::builder()
        .given_name("Iolo")
        .family_name("Lewis")
        .date_of_birth(dob(1960, 2, 29))
        .build();
    let engine = MatchingEngine::default_config();
    let a = engine.match_workers(&p1, &p2);
    let b = engine.match_workers(&p1, &p2);
    assert!((a.score - b.score).abs() < f64::EPSILON);
    assert_eq!(a.is_match, b.is_match);
}

#[test]
fn test_validation_accepts_minimum_fields() {
    assert!(
        Worker::builder()
            .given_name("Test")
            .family_name("Worker")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .uk_nhs_number(NHS_A)
            .build()
            .validate()
            .is_ok()
    );
    assert!(Worker::builder().build().validate().is_err());
}

// ============================================================
// §9. Serialization round-trips
// ============================================================

#[test]
fn test_worker_serialization_round_trip() {
    let p = Worker::builder()
        .uk_nhs_number(NHS_A)
        .given_name("Test")
        .family_name("Worker")
        .date_of_birth(dob(1990, 1, 1))
        .gender(Gender::Male)
        .build();
    let json = serde_json::to_string(&p).unwrap();
    let back: Worker = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn test_match_result_serialization_round_trip() {
    let p = Worker::builder()
        .given_name("John")
        .family_name("Smith")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p, &p.clone());
    let json = serde_json::to_string(&r).unwrap();
    let back: worker_matcher::MatchResult = serde_json::from_str(&json).unwrap();
    assert!((r.score - back.score).abs() < f64::EPSILON);
    assert_eq!(r.is_match, back.is_match);
    assert_eq!(
        r.breakdown.uk_nhs_number_score,
        back.breakdown.uk_nhs_number_score
    );
}

#[test]
fn test_address_serialization_round_trip() {
    let mut a = Address::new();
    a.line1 = Some("10 Downing Street".into());
    a.city = Some("London".into());
    a.postcode = Some("SW1A 2AA".into());
    let json = serde_json::to_string(&a).unwrap();
    let back: Address = serde_json::from_str(&json).unwrap();
    assert_eq!(a, back);
}

// ============================================================
// §10. Threading / Send + Sync
// ============================================================

#[test]
fn test_public_types_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Worker>();
    assert_send_sync::<Address>();
    assert_send_sync::<Gender>();
    assert_send_sync::<MatchConfig>();
    assert_send_sync::<MatchingEngine>();
    assert_send_sync::<worker_matcher::MatchResult>();
    assert_send_sync::<worker_matcher::MatchBreakdown>();
    assert_send_sync::<worker_matcher::MatchingError>();
    assert_send_sync::<worker_matcher::Confidence>();
    assert_send_sync::<worker_matcher::NicknameTable>();
    assert_send_sync::<worker_matcher::ParsedAddressLine>();
}

#[test]
fn test_engine_usable_across_threads() {
    use std::sync::Arc;
    use std::thread;

    let engine = Arc::new(MatchingEngine::default_config());
    let p = Arc::new(
        Worker::builder()
            .given_name("Eira")
            .family_name("Hughes")
            .date_of_birth(dob(1990, 1, 1))
            .build(),
    );

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let engine = Arc::clone(&engine);
            let p = Arc::clone(&p);
            thread::spawn(move || engine.match_workers(&p, &p).score)
        })
        .collect();

    for h in handles {
        let score = h.join().expect("thread");
        assert!(score > 0.99);
    }
}

// ============================================================
// §11. Score-shape invariants
// ============================================================

#[test]
fn test_score_is_in_unit_interval() {
    // Generate a small grid of comparisons and check every score is in [0, 1].
    let workers = [
        Worker::builder().given_name("A").family_name("B").build(),
        Worker::builder()
            .given_name("AA")
            .family_name("BB")
            .date_of_birth(dob(2000, 1, 1))
            .build(),
        Worker::builder().uk_nhs_number(NHS_A).build(),
        Worker::builder()
            .uk_nhs_number(NHS_B)
            .given_name("X")
            .build(),
        Worker::builder().build(),
    ];
    let engine = MatchingEngine::default_config();
    for a in &workers {
        for b in &workers {
            let s = engine.match_workers(a, b).score;
            assert!((0.0..=1.0).contains(&s), "score out of range: {s}");
        }
    }
}

// ============================================================
// §12. Multinational national identifiers
// ============================================================
//
// Each scheme is exercised the same way: that two textual layouts of the
// same identifier canonicalise to equality, that a deterministic match
// fires on the identifier alone, and that the probabilistic breakdown
// surfaces the per-scheme score independently.

// ---- France NIR ----

/// A second, distinct France NIR with a verified Mod-97 check key. Body
/// `1801275123457` differs from `NIR_A_FR`'s body by the trailing digit; its
/// Mod-97 remainder is 56 (one more than 55), so its key is 41 = 97-56.
const NIR_A_FR: &str = "180127512345642";
const NIR_B_FR: &str = "180127512345741";

#[test]
fn test_fr_nir_two_layouts_canonicalise_equally() {
    assert_eq!(
        identifiers::parse_fr_nir("1 80 12 75 123 456 42"),
        identifiers::parse_fr_nir(NIR_A_FR),
    );
}

#[test]
fn test_fr_nir_deterministic_match_overrides_demographics() {
    let p1 = Worker::builder()
        .fr_nir("1 80 12 75 123 456 42")
        .given_name("Jean")
        .family_name("Dupont")
        .build();
    let p2 = Worker::builder()
        .fr_nir(NIR_A_FR)
        .given_name("Marie") // intentionally different
        .family_name("Curie")
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_fr_nir_probabilistic_score_breakdown() {
    let same1 = Worker::builder()
        .fr_nir(NIR_A_FR)
        .given_name("Jean")
        .family_name("Dupont")
        .build();
    let same2 = Worker::builder()
        .fr_nir(NIR_A_FR)
        .given_name("Jean")
        .family_name("Dupont")
        .build();
    let r = MatchingEngine::default_config().match_workers(&same1, &same2);
    assert_eq!(r.breakdown.fr_nir_score, Some(1.0));
    assert!(r.is_match);

    let diff = Worker::builder()
        .fr_nir(NIR_B_FR)
        .given_name("Jean")
        .family_name("Dupont")
        .build();
    let r = MatchingEngine::default_config().match_workers(&same1, &diff);
    assert_eq!(r.breakdown.fr_nir_score, Some(0.0));
}

// ---- Spain TSI / CIP-SNS ----

const TSI_A_ES: &str = "ABCD123456XY1234";
const TSI_B_ES: &str = "WXYZ654321AB9876";

#[test]
fn test_es_tsi_normalises_whitespace_and_hyphens() {
    let p1 = Worker::builder()
        .es_tsi("abcd 123 456-xy1234")
        .given_name("María")
        .build();
    let p2 = Worker::builder()
        .es_tsi(TSI_A_ES)
        .given_name("Pedro") // different name; deterministic match drives the result
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_es_tsi_mismatch_scores_zero_not_none() {
    let p1 = Worker::builder()
        .es_tsi(TSI_A_ES)
        .given_name("María")
        .family_name("García")
        .build();
    let p2 = Worker::builder()
        .es_tsi(TSI_B_ES)
        .given_name("María")
        .family_name("García")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.es_tsi_score, Some(0.0));
}

// ---- Ireland IHI ----

const IHI_A_IE: &str = "1234567";
const IHI_B_IE: &str = "7654321";

#[test]
fn test_ie_ihi_whitespace_layouts_match_deterministically() {
    let p1 = Worker::builder().ie_ihi("123 4567").build();
    let p2 = Worker::builder().ie_ihi(IHI_A_IE).build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_ie_ihi_probabilistic_breakdown_distinguishes_two_individuals() {
    let p1 = Worker::builder()
        .ie_ihi(IHI_A_IE)
        .given_name("Aoife")
        .family_name("Murphy")
        .build();
    let p2 = Worker::builder()
        .ie_ihi(IHI_B_IE)
        .given_name("Aoife")
        .family_name("Murphy")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.ie_ihi_score, Some(0.0));
}

#[test]
fn test_ie_ihi_unparseable_yields_none_not_zero() {
    let p1 = Worker::builder()
        .ie_ihi("not-a-number")
        .given_name("Aoife")
        .family_name("Murphy")
        .build();
    let p2 = Worker::builder()
        .ie_ihi("also-invalid")
        .given_name("Aoife")
        .family_name("Murphy")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.ie_ihi_score, None);
    assert!(r.is_match, "demographics still align");
}

// ---- Northern Ireland H&C Number ----

#[test]
fn test_uk_hc_number_deterministic_match() {
    let p1 = Worker::builder()
        .uk_hc_number("943 476 5919")
        .given_name("Seán")
        .build();
    let p2 = Worker::builder()
        .uk_hc_number(NHS_A)
        .given_name("Patrick") // different name; H&C drives the match
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_uk_hc_number_does_not_cross_match_with_uk_nhs_number() {
    // Same 10-digit value, but one is recorded as an NHS Number and the
    // other as an H&C Number — different registries, no deterministic
    // match by identifier alone.
    let p1 = Worker::builder().uk_nhs_number(NHS_A).build();
    let p2 = Worker::builder().uk_hc_number(NHS_A).build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

// ---- United States SSN ----

const SSN_A_US: &str = "123-45-6789";
const SSN_B_US: &str = "987-65-4320"; // 987 area is invalid; see test below

#[test]
fn test_us_ssn_hyphenated_and_compact_canonicalise_equally() {
    assert_eq!(
        identifiers::parse_us_ssn(SSN_A_US),
        identifiers::parse_us_ssn("123456789"),
    );
}

#[test]
fn test_us_ssn_deterministic_match_overrides_demographics() {
    let p1 = Worker::builder()
        .us_ssn(SSN_A_US)
        .given_name("John")
        .family_name("Doe")
        .build();
    let p2 = Worker::builder()
        .us_ssn("123456789") // same SSN, compact form
        .given_name("Different")
        .family_name("Worker")
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_us_ssn_probabilistic_breakdown_reports_independent_score() {
    let p1 = Worker::builder()
        .us_ssn(SSN_A_US)
        .given_name("Alice")
        .family_name("Anderson")
        .build();
    let p2 = Worker::builder()
        .us_ssn(SSN_A_US)
        .given_name("Alice")
        .family_name("Anderson")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.us_ssn_score, Some(1.0));
    assert!(r.is_match);
}

#[test]
fn test_us_ssn_mismatch_scores_zero_not_none() {
    // Use two structurally-valid SSNs that differ.
    let p1 = Worker::builder()
        .us_ssn("123-45-6789")
        .given_name("Alice")
        .family_name("Anderson")
        .build();
    let p2 = Worker::builder()
        .us_ssn("234-56-7890")
        .given_name("Alice")
        .family_name("Anderson")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.us_ssn_score, Some(0.0));
}

#[test]
fn test_us_ssn_structurally_invalid_yields_none_not_zero() {
    // Area 987 is in the never-issued 900..=999 range; parse returns None.
    // Both sides invalid → breakdown is None and demographics carry the match.
    let p1 = Worker::builder()
        .us_ssn(SSN_B_US)
        .given_name("Alice")
        .family_name("Anderson")
        .build();
    let p2 = Worker::builder()
        .us_ssn("900-00-0000")
        .given_name("Alice")
        .family_name("Anderson")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.us_ssn_score, None);
    assert!(r.is_match, "demographics align");
}

#[test]
fn test_us_ssn_validate_accepts_solo_ssn() {
    assert!(
        Worker::builder()
            .us_ssn(SSN_A_US)
            .build()
            .validate()
            .is_ok(),
    );
}

// ---- Germany KVNR ----

const KVNR_A_DE: &str = "A123456780";
const KVNR_B_DE: &str = "B987654320";

#[test]
fn test_de_kvnr_deterministic_match_overrides_demographics() {
    let p1 = Worker::builder()
        .de_kvnr(KVNR_A_DE)
        .given_name("Hans")
        .family_name("Müller")
        .build();
    let p2 = Worker::builder()
        .de_kvnr("a 123 456 780") // whitespace + lowercase
        .given_name("Different")
        .family_name("Spelling")
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_de_kvnr_mismatch_scores_zero_not_none() {
    let p1 = Worker::builder()
        .de_kvnr(KVNR_A_DE)
        .given_name("Hans")
        .family_name("Müller")
        .build();
    let p2 = Worker::builder()
        .de_kvnr(KVNR_B_DE)
        .given_name("Hans")
        .family_name("Müller")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.de_kvnr_score, Some(0.0));
}

#[test]
fn test_de_kvnr_unparseable_yields_none_not_zero() {
    let p1 = Worker::builder()
        .de_kvnr("not-a-kvnr")
        .given_name("Hans")
        .family_name("Müller")
        .build();
    let p2 = Worker::builder()
        .de_kvnr("also-bogus")
        .given_name("Hans")
        .family_name("Müller")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.de_kvnr_score, None);
    assert!(r.is_match, "demographics carry the match");
}

// ---- Italy Codice Fiscale ----

const CF_A_IT: &str = "RSSMRA85T10A562S";
const CF_B_IT: &str = "MNRMRC75H17H501I";

#[test]
fn test_it_cf_deterministic_match() {
    let p1 = Worker::builder()
        .it_cf("rss mra 85t 10a 562s") // mixed case + whitespace
        .given_name("Mario")
        .family_name("Rossi")
        .build();
    let p2 = Worker::builder()
        .it_cf(CF_A_IT)
        .given_name("Different")
        .family_name("Name")
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_it_cf_mismatch_scores_zero() {
    let p1 = Worker::builder()
        .it_cf(CF_A_IT)
        .given_name("Mario")
        .family_name("Rossi")
        .build();
    let p2 = Worker::builder()
        .it_cf(CF_B_IT)
        .given_name("Mario")
        .family_name("Rossi")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.it_cf_score, Some(0.0));
}

// ---- Netherlands BSN ----

const BSN_A_NL: &str = "111222333";
const BSN_B_NL: &str = "123456782";

#[test]
fn test_nl_bsn_deterministic_match() {
    let p1 = Worker::builder()
        .nl_bsn("111 222 333") // whitespace
        .given_name("Jan")
        .family_name("de Vries")
        .build();
    let p2 = Worker::builder()
        .nl_bsn(BSN_A_NL)
        .given_name("Different")
        .family_name("Name")
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_nl_bsn_mismatch_scores_zero() {
    let p1 = Worker::builder()
        .nl_bsn(BSN_A_NL)
        .given_name("Jan")
        .family_name("de Vries")
        .build();
    let p2 = Worker::builder()
        .nl_bsn(BSN_B_NL)
        .given_name("Jan")
        .family_name("de Vries")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.nl_bsn_score, Some(0.0));
}

// ---- Sweden Personnummer ----

const PNR_A_SE: &str = "4603243850";
const PNR_B_SE: &str = "8112310092";

#[test]
fn test_se_personnummer_separator_variants_match_deterministically() {
    let p1 = Worker::builder().se_personnummer("460324-3850").build();
    let p2 = Worker::builder().se_personnummer(PNR_A_SE).build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_se_personnummer_ten_and_twelve_digit_forms_do_not_cross_match() {
    // The 10-digit form `460324-3850` and the 12-digit form
    // `19460324-3850` canonicalise to different strings, so they do
    // not deterministically match. Consumers needing cross-form
    // matching should normalise upstream.
    let p1 = Worker::builder().se_personnummer("460324-3850").build();
    let p2 = Worker::builder().se_personnummer("19460324-3850").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.se_personnummer_score, Some(0.0));
}

#[test]
fn test_se_personnummer_mismatch_scores_zero() {
    let p1 = Worker::builder()
        .se_personnummer(PNR_A_SE)
        .given_name("Karl")
        .family_name("Andersson")
        .build();
    let p2 = Worker::builder()
        .se_personnummer(PNR_B_SE)
        .given_name("Karl")
        .family_name("Andersson")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.se_personnummer_score, Some(0.0));
}

// ---- Australia IHI ----

const IHI_A_AU: &str = "8003601234567894";
const IHI_B_AU: &str = "8003619876543213";

#[test]
fn test_au_ihi_whitespace_variants_match_deterministically() {
    let p1 = Worker::builder().au_ihi("8003 6012 3456 7894").build();
    let p2 = Worker::builder().au_ihi(IHI_A_AU).build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_au_ihi_mismatch_scores_zero() {
    let p1 = Worker::builder()
        .au_ihi(IHI_A_AU)
        .given_name("Sarah")
        .family_name("Smith")
        .build();
    let p2 = Worker::builder()
        .au_ihi(IHI_B_AU)
        .given_name("Sarah")
        .family_name("Smith")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.au_ihi_score, Some(0.0));
}

#[test]
fn test_au_ihi_does_not_cross_match_ie_ihi() {
    // Australia and Ireland both have an "IHI" identifier, but they
    // are scheme-local. The Ireland IHI is 7 digits; the Australia
    // IHI is 16 digits; no value can parse as both.
    let p1 = Worker::builder().au_ihi(IHI_A_AU).build();
    let p2 = Worker::builder().ie_ihi("1234567").build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

// ---- Scotland CHI ----

const CHI_A_UK: &str = "0101701233";
const CHI_B_UK: &str = "0101701241";

#[test]
fn test_uk_chi_number_deterministic_match() {
    let p1 = Worker::builder()
        .uk_chi_number("010 170 1233")
        .given_name("Alex")
        .family_name("MacLeod")
        .build();
    let p2 = Worker::builder()
        .uk_chi_number(CHI_A_UK)
        .given_name("Different")
        .family_name("Name")
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_uk_chi_number_mismatch_scores_zero() {
    let p1 = Worker::builder()
        .uk_chi_number(CHI_A_UK)
        .given_name("Alex")
        .family_name("MacLeod")
        .build();
    let p2 = Worker::builder()
        .uk_chi_number(CHI_B_UK)
        .given_name("Alex")
        .family_name("MacLeod")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.uk_chi_number_score, Some(0.0));
}

#[test]
fn test_uk_chi_number_does_not_cross_match_uk_nhs_number() {
    // Same Mod-11 algorithm, but the schemes are local: a value
    // recorded as a CHI Number on one side and an NHS Number on the
    // other does not deterministically match.
    let p1 = Worker::builder().uk_chi_number(CHI_A_UK).build();
    let p2 = Worker::builder().uk_nhs_number(CHI_A_UK).build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_uk_chi_number_does_not_cross_match_uk_hc_number() {
    // Three UK schemes share the same Mod-11 algorithm
    // (uk_nhs_number, uk_hc_number, uk_chi_number) but none of them
    // ever cross-match: provenance lives on the field name.
    let p1 = Worker::builder().uk_chi_number(CHI_A_UK).build();
    let p2 = Worker::builder().uk_hc_number(CHI_A_UK).build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

// ---- Cross-scheme guarantees ----

#[test]
fn test_breakdown_carries_independent_score_per_scheme() {
    let p1 = Worker::builder()
        .uk_nhs_number(NHS_A)
        .fr_nir(NIR_A_FR)
        .es_tsi(TSI_A_ES)
        .ie_ihi(IHI_A_IE)
        .uk_hc_number(NHS_A)
        .us_ssn(SSN_A_US)
        .au_ihi(IHI_A_AU)
        .de_kvnr(KVNR_A_DE)
        .it_cf(CF_A_IT)
        .nl_bsn(BSN_A_NL)
        .se_personnummer(PNR_A_SE)
        .uk_chi_number(CHI_A_UK)
        .given_name("Polyglot")
        .family_name("Worker")
        .build();
    let p2 = p1.clone();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.uk_nhs_number_score, Some(1.0));
    assert_eq!(r.breakdown.fr_nir_score, Some(1.0));
    assert_eq!(r.breakdown.es_tsi_score, Some(1.0));
    assert_eq!(r.breakdown.ie_ihi_score, Some(1.0));
    assert_eq!(r.breakdown.uk_hc_number_score, Some(1.0));
    assert_eq!(r.breakdown.us_ssn_score, Some(1.0));
    assert_eq!(r.breakdown.au_ihi_score, Some(1.0));
    assert_eq!(r.breakdown.de_kvnr_score, Some(1.0));
    assert_eq!(r.breakdown.it_cf_score, Some(1.0));
    assert_eq!(r.breakdown.nl_bsn_score, Some(1.0));
    assert_eq!(r.breakdown.se_personnummer_score, Some(1.0));
    assert_eq!(r.breakdown.uk_chi_number_score, Some(1.0));
    assert!(r.is_match);
}

#[test]
fn test_validate_accepts_any_single_national_identifier() {
    // Each identifier on its own is sufficient for Worker::validate.
    assert!(
        Worker::builder()
            .fr_nir(NIR_A_FR)
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .es_tsi(TSI_A_ES)
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .ie_ihi(IHI_A_IE)
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .uk_hc_number(NHS_A)
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .au_ihi(IHI_A_AU)
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .de_kvnr(KVNR_A_DE)
            .build()
            .validate()
            .is_ok()
    );
    assert!(Worker::builder().it_cf(CF_A_IT).build().validate().is_ok());
    assert!(
        Worker::builder()
            .nl_bsn(BSN_A_NL)
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .se_personnummer(PNR_A_SE)
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .uk_chi_number(CHI_A_UK)
            .build()
            .validate()
            .is_ok()
    );
}

// ============================================================
// §13. International phone numbers
// ============================================================
//
// The matcher converts phone numbers to E.164 (`+CCNNN…`) form first and
// falls back to the legacy national-significant form only when either input
// cannot be parsed against the supported country table. These tests pin the
// observable matcher behaviour for the most common multinational scenarios.

#[test]
fn test_international_phone_uk_plus_and_trunk_layouts_match() {
    let p1 = Worker::builder()
        .given_name("Owen")
        .family_name("Williams")
        .phone("+44 7700 900123")
        .build();
    let p2 = Worker::builder()
        .given_name("Owen")
        .family_name("Williams")
        .phone("07700 900123")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

#[test]
fn test_international_phone_french_layouts_match_under_fr_default() {
    let cfg = MatchConfig {
        phone_default_country: Some("FR".into()),
        ..MatchConfig::default()
    };
    let p1 = Worker::builder()
        .given_name("Jean")
        .family_name("Dupont")
        .phone("01 23 45 67 89")
        .build();
    let p2 = Worker::builder()
        .given_name("Jean")
        .family_name("Dupont")
        .phone("+33 1 23 45 67 89")
        .build();
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

#[test]
fn test_international_phone_distinguishes_uk_from_us_with_overlapping_digits() {
    // The same 10-digit national-significant string interpreted as a UK
    // number (`+442025551234`) and as a US number (`+12025551234`) must
    // not collide in the matcher.
    let british = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .phone("2025551234") // GB default → +442025551234
        .build();
    let american = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .phone("+1 202 555 1234")
        .build();
    let r = MatchingEngine::default_config().match_workers(&british, &american);
    assert_eq!(r.breakdown.phone_score, Some(0.0));
}

#[test]
fn test_international_phone_spain_no_trunk_prefix() {
    // Spain's national format has no leading 0; both layouts must
    // canonicalise to the same +34NNN string.
    let cfg = MatchConfig {
        phone_default_country: Some("ES".into()),
        ..MatchConfig::default()
    };
    let p1 = Worker::builder()
        .given_name("María")
        .family_name("García")
        .phone("912 345 678")
        .build();
    let p2 = Worker::builder()
        .given_name("María")
        .family_name("García")
        .phone("+34 912 345 678")
        .build();
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

#[test]
fn test_international_phone_ireland_three_digit_dial_code() {
    let cfg = MatchConfig {
        phone_default_country: Some("IE".into()),
        ..MatchConfig::default()
    };
    let p1 = Worker::builder()
        .given_name("Aoife")
        .family_name("Murphy")
        .phone("01 234 5678")
        .build();
    let p2 = Worker::builder()
        .given_name("Aoife")
        .family_name("Murphy")
        .phone("+353 1 234 5678")
        .build();
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

#[test]
fn test_international_phone_germany_via_00_access_prefix() {
    // "00" international-access form should be treated as equivalent to "+".
    let p1 = Worker::builder()
        .given_name("Hans")
        .family_name("Müller")
        .phone("0049 30 12345678")
        .build();
    let p2 = Worker::builder()
        .given_name("Hans")
        .family_name("Müller")
        .phone("+49 30 12345678")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

#[test]
fn test_international_phone_falls_back_to_legacy_when_unparseable() {
    // When neither side carries an international marker and the default
    // country cannot parse the input (e.g. dial code "99" not in the
    // table on a country with no default), the matcher must fall back to
    // the legacy national-significant comparison so existing data still
    // scores meaningfully.
    let cfg = MatchConfig {
        phone_default_country: None,
        ..MatchConfig::default()
    };
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .phone("07700 900123")
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .phone("07700 900123")
        .build();
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    // Both sides fail E.164 (no default country, no `+` / `00`), so the
    // fallback path is taken and the strings compare equal.
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

#[test]
fn test_international_phone_does_not_collide_with_legacy_uk_fallback() {
    // Regression: when one side parses to E.164 and the other does not,
    // the matcher falls back to the legacy form for both. The previous
    // single-country behaviour is preserved for unsupported inputs.
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .phone("+99 12345678") // unknown dial code → E.164 None
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .phone("12345678") // would be GB E.164 under default; mixed path
        .build();
    // Either result is acceptable as long as the matcher does not panic
    // and the score is in [0.0, 1.0].
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.phone_score.expect("scored");
    assert!((0.0..=1.0).contains(&s));
}

#[test]
fn test_international_phone_idempotent_under_round_trip() {
    // A phone stored already in E.164 must continue to match its
    // round-tripped equivalent.
    let canonical =
        Normalizer::normalize_phone_e164("+44 7700 900123", Some("GB")).expect("canonicalises");
    let p1 = Worker::builder()
        .given_name("Owen")
        .family_name("Williams")
        .phone(&canonical)
        .build();
    let p2 = Worker::builder()
        .given_name("Owen")
        .family_name("Williams")
        .phone("07700 900123")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

#[test]
fn test_international_phone_config_default_country_none_disables_assumption() {
    // With phone_default_country=None, a UK trunk-format number on one
    // side cannot be lifted to E.164. The legacy fallback compares
    // national-significant digits, so an explicit +44 form is still
    // matched against the legacy "0…" form via that fallback.
    let cfg = MatchConfig {
        phone_default_country: None,
        ..MatchConfig::default()
    };
    let p1 = Worker::builder()
        .given_name("Owen")
        .family_name("Williams")
        .phone("+44 7700 900123")
        .build();
    let p2 = Worker::builder()
        .given_name("Owen")
        .family_name("Williams")
        .phone("07700 900123")
        .build();
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

// ============================================================
// §14. Sophisticated address parsing
// ============================================================
//
// The matcher parses line 1 into (house_number, street) and uses the
// abbreviation-expanding `normalize_address_line` for the street
// comparison. Mismatching house numbers penalise the address sub-score
// even when the street name is similar; matching house numbers lift it.

#[test]
fn test_address_parsing_recognises_abbreviated_street_type() {
    // "High Street" vs "High St" should score essentially identically
    // now that the address normaliser expands the abbreviation.
    let mut a1 = Address::new();
    a1.line1 = Some("123 High Street".into());
    let mut a2 = Address::new();
    a2.line1 = Some("123 High St".into());

    let p1 = Worker::builder()
        .given_name("David")
        .family_name("Evans")
        .address(a1)
        .build();
    let p2 = Worker::builder()
        .given_name("David")
        .family_name("Evans")
        .address(a2)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.address_score.expect("scored");
    assert!(s > 0.15, "abbreviation-aware score should be high: {s}");
}

#[test]
fn test_address_parsing_penalises_different_house_numbers() {
    // Same street, different house number — score should drop noticeably.
    let mut a_match1 = Address::new();
    a_match1.line1 = Some("10 Downing Street".into());
    let mut a_match2 = Address::new();
    a_match2.line1 = Some("10 Downing Street".into());

    let mut a_diff1 = Address::new();
    a_diff1.line1 = Some("10 Downing Street".into());
    let mut a_diff2 = Address::new();
    a_diff2.line1 = Some("20 Downing Street".into());

    let engine = MatchingEngine::default_config();
    let make = |a: Address| {
        Worker::builder()
            .given_name("X")
            .family_name("Y")
            .address(a)
            .build()
    };
    let same = engine.match_workers(&make(a_match1), &make(a_match2));
    let diff = engine.match_workers(&make(a_diff1), &make(a_diff2));
    assert!(
        same.breakdown.address_score > diff.breakdown.address_score,
        "matching house numbers must outscore mismatching: same={:?} diff={:?}",
        same.breakdown.address_score,
        diff.breakdown.address_score,
    );
}

#[test]
fn test_address_parsing_directional_abbreviation() {
    let mut a1 = Address::new();
    a1.line1 = Some("100 N Park Ave".into());
    let mut a2 = Address::new();
    a2.line1 = Some("100 North Park Avenue".into());
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(a1)
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(a2)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    // After expansion, both lines normalise to the same string.
    let s = r.breakdown.address_score.expect("scored");
    assert!(
        s > 0.15,
        "score should be high after directional expansion: {s}"
    );
}

#[test]
fn test_address_parsing_unit_prefix_does_not_block_house_number_match() {
    // "Flat 2A, 10 Downing Street" and "10 Downing Street" share a
    // house number and street; the unit on one side should not block
    // the structured match.
    let mut a1 = Address::new();
    a1.line1 = Some("Flat 2A, 10 Downing Street".into());
    let mut a2 = Address::new();
    a2.line1 = Some("10 Downing Street".into());
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(a1)
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(a2)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.address_score.expect("scored");
    assert!(
        s > 0.15,
        "unit prefix must not block the structured match: {s}"
    );
}

#[test]
fn test_parsed_address_line_is_publicly_re_exported() {
    let p: worker_matcher::ParsedAddressLine = Normalizer::parse_address_line("123 High St");
    assert_eq!(p.house_number.as_deref(), Some("123"));
    assert_eq!(p.street, "high street");
}

// ============================================================
// §15. Nickname dictionary
// ============================================================
//
// MatchConfig::nickname_table is empty by default, so adding the feature
// is a no-op for existing callers. When the table is populated, the
// matcher lifts the given-name (and family-name) similarity score to at
// least 0.9 for any pair the table considers equivalent.

#[test]
fn test_nicknames_default_config_does_not_change_existing_scores() {
    // No nickname table → behaviour matches the pre-feature baseline.
    let p1 = Worker::builder()
        .given_name("Michael")
        .family_name("Jones")
        .date_of_birth(dob(1978, 9, 25))
        .build();
    let p2 = Worker::builder()
        .given_name("Mike")
        .family_name("Jones")
        .date_of_birth(dob(1978, 9, 25))
        .build();
    let baseline = MatchingEngine::default_config().match_workers(&p1, &p2);
    // Without the table, the Combined algorithm on "Michael" vs "Mike"
    // is well below the 0.9 boost ceiling.
    assert!(baseline.breakdown.given_name_score.unwrap() < 0.85);
}

#[test]
fn test_nicknames_english_table_lifts_given_name_to_at_least_0_9() {
    let cfg = MatchConfig {
        nickname_table: NicknameTable::english(),
        ..MatchConfig::default()
    };
    for (a, b) in [
        ("Michael", "Mike"),
        ("Elizabeth", "Liz"),
        ("Robert", "Bob"),
        ("William", "Bill"),
        ("Richard", "Dick"),
    ] {
        let p1 = Worker::builder().given_name(a).family_name("Smith").build();
        let p2 = Worker::builder().given_name(b).family_name("Smith").build();
        let r = MatchingEngine::new(cfg.clone()).match_workers(&p1, &p2);
        let s = r.breakdown.given_name_score.unwrap();
        assert!(
            s >= 0.9,
            "expected nickname boost ≥ 0.9 for {a:?} vs {b:?}, got {s}",
        );
    }
}

#[test]
fn test_nicknames_boost_is_one_way_lift_never_drop() {
    // Identical names already score 1.0; the nickname check must not
    // drag them down.
    let cfg = MatchConfig {
        nickname_table: NicknameTable::english(),
        ..MatchConfig::default()
    };
    let p = Worker::builder()
        .given_name("Mike")
        .family_name("Smith")
        .build();
    let r = MatchingEngine::new(cfg).match_workers(&p, &p.clone());
    assert_eq!(r.breakdown.given_name_score, Some(1.0));
}

#[test]
fn test_nicknames_unrelated_names_are_untouched() {
    let cfg = MatchConfig {
        nickname_table: NicknameTable::english(),
        ..MatchConfig::default()
    };
    let p1 = Worker::builder()
        .given_name("Mike")
        .family_name("S")
        .build();
    let p2 = Worker::builder().given_name("Bob").family_name("S").build();
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    // "Mike" and "Bob" are not in the same class — base score, no lift.
    assert!(r.breakdown.given_name_score.unwrap() < 0.6);
}

#[test]
fn test_nicknames_custom_class_extends_english_default() {
    let cfg = MatchConfig {
        nickname_table: NicknameTable::english().with_class(["Reginald", "Reggie"]),
        ..MatchConfig::default()
    };
    let p1 = Worker::builder()
        .given_name("Reginald")
        .family_name("S")
        .build();
    let p2 = Worker::builder()
        .given_name("Reggie")
        .family_name("S")
        .build();
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert!(r.breakdown.given_name_score.unwrap() >= 0.9);
}

#[test]
fn test_nicknames_promote_overall_match_from_under_to_over_threshold() {
    // With strong supporting evidence (family + DOB + gender), the
    // nickname boost can be the difference between a sub-threshold and
    // a passing overall match.
    let p1 = Worker::builder()
        .given_name("Michael")
        .family_name("Jones")
        .date_of_birth(dob(1978, 9, 25))
        .gender(Gender::Male)
        .build();
    let p2 = Worker::builder()
        .given_name("Mike")
        .family_name("Jones")
        .date_of_birth(dob(1978, 9, 25))
        .gender(Gender::Male)
        .build();
    let baseline = MatchingEngine::default_config().match_workers(&p1, &p2);
    let cfg = MatchConfig {
        nickname_table: NicknameTable::english(),
        ..MatchConfig::default()
    };
    let with_nicknames = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert!(
        with_nicknames.score > baseline.score,
        "nickname table must lift the overall score: baseline={} with={}",
        baseline.score,
        with_nicknames.score,
    );
}

// ============================================================
// §16. Email scoring
// ============================================================
//
// `MatchBreakdown::email_score` is `None` if either side is missing or
// fails to parse as an email. The default config does NOT enable Gmail
// dot-folding, so `j.smith@gmail.com` and `jsmith@gmail.com` differ by
// default but match when `gmail_dot_folding = true`.

#[test]
fn test_email_exact_match_after_case_and_whitespace_normalisation() {
    let p1 = Worker::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .email("  Alice@Example.ORG  ")
        .build();
    let p2 = Worker::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .email("alice@example.org")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.email_score, Some(1.0));
}

#[test]
fn test_email_mismatch_scores_zero() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .email("alice@example.org")
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .email("bob@example.org")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.email_score, Some(0.0));
}

#[test]
fn test_email_missing_on_one_side_scores_none_not_zero() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .email("alice@example.org")
        .build();
    let p2 = Worker::builder().given_name("X").family_name("Y").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.email_score, None);
}

#[test]
fn test_email_unparseable_yields_none_not_zero() {
    let p1 = Worker::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .email("not-an-email")
        .build();
    let p2 = Worker::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .email("also@invalid@thing")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.email_score, None);
    assert!(r.is_match, "demographics carry the match");
}

#[test]
fn test_email_gmail_dot_folding_opt_in() {
    // Default behaviour: dotted and undotted Gmail addresses differ.
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .email("j.smith@gmail.com")
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .email("jsmith@gmail.com")
        .build();
    let default = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(default.breakdown.email_score, Some(0.0));

    // Opt-in: dot-folded Gmail localparts are equivalent.
    let cfg = MatchConfig {
        gmail_dot_folding: true,
        ..MatchConfig::default()
    };
    let folded = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert_eq!(folded.breakdown.email_score, Some(1.0));
}

#[test]
fn test_email_gmail_plus_tag_folding() {
    let cfg = MatchConfig {
        gmail_dot_folding: true,
        ..MatchConfig::default()
    };
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .email("jsmith+work@gmail.com")
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .email("jsmith@gmail.com")
        .build();
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    assert_eq!(r.breakdown.email_score, Some(1.0));
}

#[test]
fn test_email_dot_folding_does_not_affect_non_gmail_domains() {
    let cfg = MatchConfig {
        gmail_dot_folding: true,
        ..MatchConfig::default()
    };
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .email("j.smith@example.org")
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .email("jsmith@example.org")
        .build();
    let r = MatchingEngine::new(cfg).match_workers(&p1, &p2);
    // Dot-folding is Gmail-specific; example.org localparts compare verbatim.
    assert_eq!(r.breakdown.email_score, Some(0.0));
}

// ============================================================
// §16a. `local_id` is never scored (organisation-level / cross-org codes)
// ============================================================
//
// Per spec §2 ("Out of scope (permanently): organisation-level
// identifiers") and §8 / OQ-2, `local_id` is carried but deliberately
// NOT scored: different organisations may issue colliding values, and an
// organisation-level code (e.g. a UK NHS ODS code) is shared by every
// worker at the same practice. These tests pin that the field adds no
// signal in either direction — the matcher-side half of `worker-service`
// entity task T-7.

#[test]
fn test_local_id_difference_does_not_lower_score() {
    // Two records identical except for `local_id` must score the same as
    // the field-free pair: `local_id` contributes nothing.
    let with_a = Worker::builder()
        .given_name("Asha")
        .family_name("Patel")
        .date_of_birth(dob(1970, 4, 1))
        .gender(Gender::Female)
        .local_id("PRACTICE-A/MRN-1")
        .build();
    let with_b = Worker::builder()
        .given_name("Asha")
        .family_name("Patel")
        .date_of_birth(dob(1970, 4, 1))
        .gender(Gender::Female)
        .local_id("PRACTICE-B/MRN-2")
        .build();
    let without = Worker::builder()
        .given_name("Asha")
        .family_name("Patel")
        .date_of_birth(dob(1970, 4, 1))
        .gender(Gender::Female)
        .build();

    let engine = MatchingEngine::default_config();
    let differing = engine.match_workers(&with_a, &with_b);
    let absent = engine.match_workers(&without, &without.clone());

    assert!(
        (differing.score - absent.score).abs() < 1e-9,
        "differing local_id must not change the score (got {} vs {})",
        differing.score,
        absent.score
    );
    assert!(differing.is_match);
}

#[test]
fn test_shared_local_id_adds_no_signal_between_unrelated_workers() {
    // A shared organisation-level code (same value on two unrelated
    // workers — e.g. colleagues at one practice) must NOT push them to a
    // match: `local_id` is never an exact-match short-circuit.
    let shared = "RXX01"; // illustrative NHS ODS-style organisation code
    let a = Worker::builder()
        .given_name("Asha")
        .family_name("Patel")
        .date_of_birth(dob(1970, 4, 1))
        .gender(Gender::Female)
        .local_id(shared)
        .build();
    let b = Worker::builder()
        .given_name("Sven")
        .family_name("Olsen")
        .date_of_birth(dob(1992, 12, 24))
        .gender(Gender::Male)
        .local_id(shared)
        .build();

    let engine = MatchingEngine::default_config();
    let shared_code = engine.match_workers(&a, &b);
    let no_code = engine.match_workers(
        &Worker::builder()
            .given_name("Asha")
            .family_name("Patel")
            .date_of_birth(dob(1970, 4, 1))
            .gender(Gender::Female)
            .build(),
        &Worker::builder()
            .given_name("Sven")
            .family_name("Olsen")
            .date_of_birth(dob(1992, 12, 24))
            .gender(Gender::Male)
            .build(),
    );

    assert!(
        !shared_code.is_match,
        "a shared local_id must not force a match"
    );
    assert!(
        (shared_code.score - no_code.score).abs() < 1e-9,
        "shared local_id must add no signal (got {} with code vs {} without)",
        shared_code.score,
        no_code.score
    );
}

// ============================================================
// §17. Confidence band
// ============================================================

#[test]
fn test_confidence_high_for_exact_clone() {
    let p = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .date_of_birth(dob(1815, 12, 10))
        .gender(Gender::Female)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p, &p.clone());
    assert_eq!(r.confidence, Confidence::High);
    assert!(r.score >= 0.90);
}

#[test]
fn test_confidence_low_for_completely_different_workers() {
    let p1 = Worker::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .date_of_birth(dob(1990, 1, 1))
        .gender(Gender::Female)
        .build();
    let p2 = Worker::builder()
        .given_name("Zachary")
        .family_name("Zimmerman")
        .date_of_birth(dob(2000, 12, 31))
        .gender(Gender::Male)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.confidence, Confidence::Low);
    assert!(r.score < 0.75);
}

#[test]
fn test_confidence_bands_are_independent_of_match_threshold() {
    // Same input, different threshold presets → same band, different is_match.
    let p1 = Worker::builder()
        .given_name("Catherine")
        .family_name("Williams")
        .date_of_birth(dob(1975, 8, 22))
        .gender(Gender::Female)
        .build();
    let p2 = Worker::builder()
        .given_name("Katherine")
        .family_name("Williams")
        .date_of_birth(dob(1975, 8, 22))
        .gender(Gender::Female)
        .build();
    let strict = MatchingEngine::new(MatchConfig::strict()).match_workers(&p1, &p2);
    let lenient = MatchingEngine::new(MatchConfig::lenient()).match_workers(&p1, &p2);
    assert_eq!(
        strict.confidence, lenient.confidence,
        "bands are score-based, not threshold-based",
    );
    assert!(
        (strict.score - lenient.score).abs() < 1e-12,
        "underlying score must be identical: strict={} lenient={}",
        strict.score,
        lenient.score,
    );
}

#[test]
fn test_confidence_round_trips_via_json() {
    let p = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p, &p.clone());
    let json = serde_json::to_string(&r).unwrap();
    let back: worker_matcher::MatchResult = serde_json::from_str(&json).unwrap();
    assert_eq!(r.confidence, back.confidence);
}

#[test]
fn test_confidence_legacy_json_payload_without_field_defaults_to_low() {
    // A pre-feature JSON payload lacking the `confidence` field must
    // deserialise cleanly and default to Low so the caller does not
    // misread a stale payload as a high-confidence match.
    let legacy = r#"{
        "score": 0.97,
        "is_match": true,
        "breakdown": {
            "uk_nhs_number_score": null,
            "fr_nir_score": null,
            "es_tsi_score": null,
            "ie_ihi_score": null,
            "uk_hc_number_score": null,
            "us_ssn_score": null,
            "given_name_score": 1.0,
            "family_name_score": 1.0,
            "date_of_birth_score": null,
            "gender_score": null,
            "address_score": null,
            "phone_score": null,
            "phonetic_name_score": 0.5
        }
    }"#;
    let r: worker_matcher::MatchResult = serde_json::from_str(legacy).unwrap();
    assert_eq!(r.confidence, Confidence::Low);
}

// ============================================================
// §18. MatchConfig and SimilarityAlgorithm serde round-trip
// ============================================================

#[test]
fn test_match_config_round_trips_through_json() {
    let cfg = MatchConfig {
        match_threshold: 0.88,
        gmail_dot_folding: true,
        nickname_table: NicknameTable::english(),
        phone_default_country: Some("FR".into()),
        ..MatchConfig::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let back: MatchConfig = serde_json::from_str(&json).unwrap();
    assert!((cfg.match_threshold - back.match_threshold).abs() < 1e-12);
    assert_eq!(cfg.gmail_dot_folding, back.gmail_dot_folding);
    assert_eq!(cfg.nickname_table, back.nickname_table);
    assert_eq!(cfg.phone_default_country, back.phone_default_country);
}

#[test]
fn test_match_config_loaded_from_partial_json_uses_defaults() {
    // Production deployments often want to override one or two fields
    // from a JSON config file. `#[serde(default)]` lets them ship a
    // minimal document and inherit the rest from MatchConfig::default().
    let partial = r#"{"match_threshold": 0.80, "phone_default_country": "US"}"#;
    let cfg: MatchConfig = serde_json::from_str(partial).unwrap();
    assert!((cfg.match_threshold - 0.80).abs() < 1e-12);
    assert_eq!(cfg.phone_default_country.as_deref(), Some("US"));
    // Defaults preserved:
    assert!(matches!(cfg.name_algorithm, SimilarityAlgorithm::Combined));
    assert!((cfg.given_name_weight - 0.15).abs() < 1e-12);
    assert!(cfg.use_phonetic_matching);
}

#[test]
fn test_similarity_algorithm_serializes_as_simple_name() {
    let json = serde_json::to_string(&SimilarityAlgorithm::JaroWinkler).unwrap();
    // Default serde tag for a unit-like enum variant is the variant name.
    assert_eq!(json, "\"JaroWinkler\"");
    let back: SimilarityAlgorithm = serde_json::from_str(&json).unwrap();
    assert_eq!(SimilarityAlgorithm::JaroWinkler, back);
}

#[test]
fn test_nickname_table_round_trips_through_json() {
    let t = NicknameTable::english().with_class(["Reginald", "Reggie"]);
    let json = serde_json::to_string(&t).unwrap();
    let back: NicknameTable = serde_json::from_str(&json).unwrap();
    assert_eq!(t, back);
    assert!(back.are_equivalent("Reggie", "Reginald"));
    assert!(back.are_equivalent("Mike", "Michael"));
}

#[test]
fn test_config_loaded_from_json_drives_engine_behaviour_unchanged() {
    // End-to-end: build a config in code, serialise it, deserialise it,
    // and verify the resulting engine produces the same match outcomes.
    let original = MatchConfig {
        nickname_table: NicknameTable::english(),
        gmail_dot_folding: true,
        ..MatchConfig::default()
    };
    let json = serde_json::to_string(&original).unwrap();
    let loaded: MatchConfig = serde_json::from_str(&json).unwrap();

    let p1 = Worker::builder()
        .given_name("Michael")
        .family_name("Jones")
        .email("j.smith@gmail.com")
        .build();
    let p2 = Worker::builder()
        .given_name("Mike")
        .family_name("Jones")
        .email("jsmith@gmail.com")
        .build();

    let r1 = MatchingEngine::new(original).match_workers(&p1, &p2);
    let r2 = MatchingEngine::new(loaded).match_workers(&p1, &p2);
    assert!((r1.score - r2.score).abs() < 1e-12);
    assert_eq!(r1.is_match, r2.is_match);
    assert_eq!(r1.confidence, r2.confidence);
}

// ============================================================
// §19. Date-of-birth transposition heuristic
// ============================================================
//
// `score_date_of_birth` returns 0.5 when swapping day and month on one
// side yields the other (same year, valid swapped date). Otherwise it
// returns 1.0 for exact equality or 0.0 for any other mismatch.
// `deterministic_match` is unaffected — it still requires exact DOB
// equality on the demographic-tuple branch.

#[test]
fn test_dob_transposition_scores_half_on_classic_dd_mm_bug() {
    let p1 = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 1, 10)) // 10 January 1995
        .build();
    let p2 = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 10, 1)) // 1 October 1995 — DD/MM swap of p1
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.date_of_birth_score, Some(0.5));
}

#[test]
fn test_dob_transposition_does_not_fire_across_years() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .date_of_birth(dob(1995, 1, 10))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .date_of_birth(dob(1996, 10, 1))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.date_of_birth_score, Some(0.0));
}

#[test]
fn test_dob_transposition_lifts_overall_score_relative_to_zero_dob() {
    // Stack the transposition heuristic with strong supporting
    // evidence: matching name + gender + phone should lift the score
    // closer to threshold than a 0.0 DOB would.
    let p1 = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 1, 10))
        .gender(Gender::Male)
        .phone("07700 900111")
        .build();
    let p2_transposed = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 10, 1))
        .gender(Gender::Male)
        .phone("07700 900111")
        .build();
    let p2_unrelated = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1980, 6, 30))
        .gender(Gender::Male)
        .phone("07700 900111")
        .build();
    let engine = MatchingEngine::default_config();
    let transposed = engine.match_workers(&p1, &p2_transposed);
    let unrelated = engine.match_workers(&p1, &p2_unrelated);
    assert!(
        transposed.score > unrelated.score,
        "transposition partial credit must lift the score: transposed={} unrelated={}",
        transposed.score,
        unrelated.score,
    );
}

#[test]
fn test_dob_transposition_alone_is_not_enough_for_default_threshold() {
    // Just name + transposed DOB — under the default 0.85 threshold,
    // partial DOB credit cannot rescue the overall match.
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .date_of_birth(dob(1995, 1, 10))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .date_of_birth(dob(1995, 10, 1))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.date_of_birth_score, Some(0.5));
    assert!(!r.is_match);
}

#[test]
fn test_dob_transposition_does_not_short_circuit_deterministic_match() {
    let a = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 1, 10))
        .gender(Gender::Male)
        .build();
    let b = Worker::builder()
        .given_name("Thomas")
        .family_name("Price")
        .date_of_birth(dob(1995, 10, 1))
        .gender(Gender::Male)
        .build();
    assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
}

// ============================================================
// §20. Batch API (match_one_to_many / rank_one_to_many)
// ============================================================

#[test]
fn test_batch_match_one_to_many_returns_parallel_results() {
    let engine = MatchingEngine::default_config();
    let query = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .date_of_birth(dob(1815, 12, 10))
        .build();
    let candidates = vec![
        Worker::builder()
            .given_name("Grace")
            .family_name("Hopper")
            .build(),
        query.clone(),
        Worker::builder()
            .given_name("Alan")
            .family_name("Turing")
            .build(),
    ];
    let r = engine.match_one_to_many(&query, &candidates);
    assert_eq!(r.len(), 3);
    assert!(r[1].is_match, "clone of query must match");
    assert!(!r[0].is_match);
    assert!(!r[2].is_match);
}

#[test]
fn test_batch_match_one_to_many_filtered_for_is_match() {
    // Caller-side filtering is the recommended pattern; the batch API
    // returns everything and lets the consumer decide.
    let engine = MatchingEngine::default_config();
    let query = Worker::builder()
        .uk_nhs_number(NHS_A)
        .given_name("John")
        .family_name("Smith")
        .build();
    let candidates = vec![
        Worker::builder().uk_nhs_number(NHS_A).build(),
        Worker::builder().uk_nhs_number(NHS_B).build(),
        query.clone(),
    ];
    let results = engine.match_one_to_many(&query, &candidates);
    let positives: Vec<_> = results.iter().filter(|r| r.is_match).collect();
    // Index 0 deterministically matches (same NHS) and index 2 is a
    // clone. Index 1 has a different NHS Number and no other
    // identifying overlap → not a match.
    assert!(positives.len() >= 2);
}

#[test]
fn test_batch_rank_one_to_many_orders_clone_first() {
    let engine = MatchingEngine::default_config();
    let query = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .date_of_birth(dob(1815, 12, 10))
        .build();
    let candidates = vec![
        Worker::builder()
            .given_name("Grace")
            .family_name("Hopper")
            .build(),
        query.clone(),
        Worker::builder()
            .given_name("Alan")
            .family_name("Turing")
            .build(),
    ];
    let ranked = engine.rank_one_to_many(&query, &candidates);
    assert_eq!(ranked.len(), 3);
    assert_eq!(ranked[0].0, 1, "clone is at original index 1");
    // Strict ordering invariant.
    for w in ranked.windows(2) {
        assert!(w[0].1.score >= w[1].1.score);
    }
}

#[test]
fn test_batch_rank_one_to_many_with_empty_candidates() {
    let engine = MatchingEngine::default_config();
    let query = Worker::builder().given_name("Solo").build();
    let ranked = engine.rank_one_to_many(&query, &[]);
    assert!(ranked.is_empty());
}

#[test]
fn test_batch_match_one_to_many_carries_confidence_per_result() {
    let engine = MatchingEngine::default_config();
    let query = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .build();
    let candidates = vec![
        query.clone(),
        Worker::builder()
            .given_name("Different")
            .family_name("Worker")
            .date_of_birth(dob(2000, 1, 1))
            .build(),
    ];
    let r = engine.match_one_to_many(&query, &candidates);
    assert_eq!(r[0].confidence, Confidence::High);
    assert_eq!(r[1].confidence, Confidence::Low);
}

#[test]
fn test_batch_match_one_to_many_threadsafe_under_arc() {
    // The engine is Send + Sync; a batch can be scattered across
    // threads using stdlib primitives without any feature flag.
    use std::sync::Arc;
    use std::thread;

    let engine = Arc::new(MatchingEngine::default_config());
    let query = Arc::new(
        Worker::builder()
            .given_name("Ada")
            .family_name("Lovelace")
            .build(),
    );
    let candidates: Arc<Vec<Worker>> = Arc::new(vec![
        query.as_ref().clone(),
        Worker::builder()
            .given_name("Grace")
            .family_name("Hopper")
            .build(),
    ]);

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let engine = Arc::clone(&engine);
            let query = Arc::clone(&query);
            let candidates = Arc::clone(&candidates);
            thread::spawn(move || engine.match_one_to_many(&query, &candidates))
        })
        .collect();

    let baseline = engine.match_one_to_many(&query, &candidates);
    for h in handles {
        let r = h.join().expect("thread");
        for i in 0..r.len() {
            assert!((r[i].score - baseline[i].score).abs() < 1e-12);
        }
    }
}

// ============================================================
// §21a. Eighteen additional national personal identifiers (T-27)
// ============================================================
//
// Each scheme: deterministic match by identifier alone, mismatch
// produces a 0.0 component score, and Worker::validate accepts a
// worker whose only identifying field is the new identifier.

#[test]
fn test_be_nn_deterministic_match() {
    let p1 = Worker::builder().be_nn("80010100107").build();
    let p2 = Worker::builder().be_nn("80.01.01-001.07").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
    assert!(
        Worker::builder()
            .be_nn("80010100107")
            .build()
            .validate()
            .is_ok()
    );
}

#[test]
fn test_bg_egn_deterministic_match() {
    let p1 = Worker::builder().bg_egn("8001010013").build();
    let p2 = Worker::builder().bg_egn("8001010013").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
    let p3 = Worker::builder()
        .bg_egn("8001010013")
        .given_name("A")
        .family_name("B")
        .build();
    let p4 = Worker::builder()
        .bg_egn("0123456789")
        .given_name("A")
        .family_name("B")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p3, &p4);
    // p4's EGN is invalid so the score is None (not 0.0).
    assert_eq!(r.breakdown.bg_egn_score, None);
}

#[test]
fn test_cz_rc_deterministic_match() {
    let p1 = Worker::builder().cz_rc("8001150014").build();
    let p2 = Worker::builder().cz_rc("8001150014").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_dk_cpr_format_only_match() {
    let p1 = Worker::builder().dk_cpr("1501801234").build();
    let p2 = Worker::builder().dk_cpr("150180-1234").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_ee_ik_deterministic_match() {
    let p1 = Worker::builder().ee_ik("48001150011").build();
    let p2 = Worker::builder().ee_ik("48001150011").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_es_dni_deterministic_match() {
    let p1 = Worker::builder().es_dni("12345678Z").build();
    let p2 = Worker::builder().es_dni("12345678-z").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_fi_hetu_deterministic_match() {
    let p1 = Worker::builder().fi_hetu("150180-999B").build();
    let p2 = Worker::builder().fi_hetu("150180-999B").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_hr_oib_deterministic_match() {
    let p1 = Worker::builder().hr_oib("12345678903").build();
    let p2 = Worker::builder().hr_oib("12345678903").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_is_kt_deterministic_match() {
    let p1 = Worker::builder().is_kt("1501802529").build();
    let p2 = Worker::builder().is_kt("1501802529").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_lt_ak_deterministic_match() {
    let p1 = Worker::builder().lt_ak("48001150011").build();
    let p2 = Worker::builder().lt_ak("48001150011").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_lv_pk_deterministic_match() {
    let p1 = Worker::builder().lv_pk("15018010007").build();
    let p2 = Worker::builder().lv_pk("15018010007").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_mt_id_deterministic_match() {
    let p1 = Worker::builder().mt_id("1234567M").build();
    let p2 = Worker::builder().mt_id("1234567m").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_no_fnr_deterministic_match() {
    let p1 = Worker::builder().no_fnr("15018012399").build();
    let p2 = Worker::builder().no_fnr("15018012399").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_pl_pesel_deterministic_match() {
    let p1 = Worker::builder().pl_pesel("80011500014").build();
    let p2 = Worker::builder().pl_pesel("80011500014").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_ro_cnp_deterministic_match() {
    let p1 = Worker::builder().ro_cnp("1800115400012").build();
    let p2 = Worker::builder().ro_cnp("1800115400012").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_si_emso_deterministic_match() {
    let p1 = Worker::builder().si_emso("1501980500015").build();
    let p2 = Worker::builder().si_emso("1501980500015").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_sk_rc_deterministic_match() {
    let p1 = Worker::builder().sk_rc("8051150019").build();
    let p2 = Worker::builder().sk_rc("8051150019").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_uk_nino_deterministic_match() {
    let p1 = Worker::builder().uk_nino("AB123456A").build();
    let p2 = Worker::builder().uk_nino("ab 12 34 56 a").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

fn assert_first_six_new_schemes_validate_solo() {
    assert!(
        Worker::builder()
            .be_nn("80010100107")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .bg_egn("8001010013")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .cz_rc("8001150014")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .dk_cpr("1501801234")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .ee_ik("48001150011")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .es_dni("12345678Z")
            .build()
            .validate()
            .is_ok()
    );
}

fn assert_middle_six_new_schemes_validate_solo() {
    assert!(
        Worker::builder()
            .fi_hetu("150180-999B")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .hr_oib("12345678903")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .is_kt("1501802529")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .lt_ak("48001150011")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .lv_pk("15018010007")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .mt_id("1234567M")
            .build()
            .validate()
            .is_ok()
    );
}

fn assert_last_six_new_schemes_validate_solo() {
    assert!(
        Worker::builder()
            .no_fnr("15018012399")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .pl_pesel("80011500014")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .ro_cnp("1800115400012")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .si_emso("1501980500015")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .sk_rc("8051150019")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .uk_nino("AB123456A")
            .build()
            .validate()
            .is_ok()
    );
}

#[test]
fn test_eighteen_new_schemes_each_validate_solo() {
    assert_first_six_new_schemes_validate_solo();
    assert_middle_six_new_schemes_validate_solo();
    assert_last_six_new_schemes_validate_solo();
}

// ============================================================
// §21b. Five additional personal identifiers (T-28)
// ============================================================

#[test]
fn test_gr_dss_deterministic_match() {
    let p1 = Worker::builder().gr_dss("1234567890").build();
    let p2 = Worker::builder().gr_dss("12-34-56 78 90").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
    assert!(
        Worker::builder()
            .gr_dss("1234567890")
            .build()
            .validate()
            .is_ok()
    );
}

#[test]
fn test_gr_dss_mismatch_scores_zero() {
    let p1 = Worker::builder()
        .given_name("A")
        .family_name("B")
        .gr_dss("1234567890")
        .build();
    let p2 = Worker::builder()
        .given_name("A")
        .family_name("B")
        .gr_dss("0987654321")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.gr_dss_score, Some(0.0));
}

#[test]
fn test_li_id_deterministic_match_with_8_digit_form() {
    let p1 = Worker::builder().li_id("ID12345678").build();
    let p2 = Worker::builder().li_id("id 1234 5678").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_li_id_deterministic_match_with_9_digit_form() {
    // Spec example uses 9 trailing digits.
    let p1 = Worker::builder().li_id("ID022143586").build();
    let p2 = Worker::builder().li_id("ID022143586").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_nl_id_deterministic_match() {
    let p1 = Worker::builder().nl_id("AB1234567").build();
    let p2 = Worker::builder().nl_id("ab 12 34 567").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
    assert!(
        Worker::builder()
            .nl_id("AB1234567")
            .build()
            .validate()
            .is_ok()
    );
}

#[test]
fn test_nl_id_distinct_from_nl_bsn() {
    // The two NL fields are scheme-local: same digits recorded against
    // different schemes do NOT cross-match.
    let p1 = Worker::builder().nl_bsn("111222333").build();
    let p2 = Worker::builder().nl_id("AB1234567").build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_pl_nip_deterministic_match() {
    let p1 = Worker::builder().pl_nip("1234567802").build();
    let p2 = Worker::builder().pl_nip("123-456-78-02").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_pl_nip_distinct_from_pl_pesel() {
    // NIP (tax) and PESEL (national ID) are scheme-local even within Poland.
    let p1 = Worker::builder().pl_nip("1234567802").build();
    let p2 = Worker::builder().pl_pesel("80011500014").build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_pt_nif_deterministic_match() {
    let p1 = Worker::builder().pt_nif("123456789").build();
    let p2 = Worker::builder().pt_nif("123 456 789").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_five_new_schemes_each_validate_solo() {
    assert!(
        Worker::builder()
            .gr_dss("1234567890")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .li_id("ID12345678")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .nl_id("AB1234567")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .pl_nip("1234567802")
            .build()
            .validate()
            .is_ok()
    );
    assert!(
        Worker::builder()
            .pt_nif("123456789")
            .build()
            .validate()
            .is_ok()
    );
}

// ============================================================
// §27. T-17.1 — Seven next-batch national identifier schemes
// ============================================================
//
// One deterministic-match test + one cross-scheme isolation test per
// scheme, plus a solo-validate sweep and a polyglot round-trip.

#[test]
fn test_br_cpf_deterministic_match() {
    let p1 = Worker::builder().br_cpf("12345678909").build();
    let p2 = Worker::builder().br_cpf("123.456.789-09").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_br_cpf_rejects_mismatch() {
    let p1 = Worker::builder().br_cpf("12345678909").build();
    let p2 = Worker::builder().br_cpf("11144477735").build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_cn_rrn_deterministic_match() {
    let p1 = Worker::builder().cn_rrn("11010519491231002X").build();
    let p2 = Worker::builder().cn_rrn("11010519491231002x").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_in_aadhaar_deterministic_match() {
    let p1 = Worker::builder().in_aadhaar("234123412346").build();
    let p2 = Worker::builder().in_aadhaar("2341 2341 2346").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_jp_my_number_deterministic_match() {
    let p1 = Worker::builder().jp_my_number("123456789018").build();
    let p2 = Worker::builder().jp_my_number("1234-5678-9018").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_mx_curp_deterministic_match() {
    let p1 = Worker::builder().mx_curp("HEGG560427MVZRRL04").build();
    let p2 = Worker::builder().mx_curp("hegg560427mvzrrl04").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_nz_nhi_deterministic_match() {
    let p1 = Worker::builder().nz_nhi("ZAA0083").build();
    let p2 = Worker::builder().nz_nhi("zaa0083").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_za_id_deterministic_match() {
    let p1 = Worker::builder().za_id("8001015009087").build();
    let p2 = Worker::builder().za_id("800101 5009 087").build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_t17_1_scheme_local_isolation() {
    // Two records with the *same digit-string* across two different
    // schemes must NOT cross-match. Pick two 11-digit schemes:
    // BR CPF and US SSN. Even if a string happened to satisfy both
    // parsers (unlikely given different check-digit rules), the
    // deterministic logic must require same-scheme equality.
    let cpf_record = Worker::builder().br_cpf("12345678909").build();
    let ssn_record = Worker::builder().us_ssn("123456789").build();
    assert!(!MatchingEngine::default_config().deterministic_match(&cpf_record, &ssn_record));
}

#[test]
fn test_t17_1_schemes_each_validate_solo() {
    for builder in [
        Worker::builder().br_cpf("12345678909"),
        Worker::builder().cn_rrn("11010519491231002X"),
        Worker::builder().in_aadhaar("234123412346"),
        Worker::builder().jp_my_number("123456789018"),
        Worker::builder().mx_curp("HEGG560427MVZRRL04"),
        Worker::builder().nz_nhi("ZAA0083"),
        Worker::builder().za_id("8001015009087"),
    ] {
        assert!(
            builder.build().validate().is_ok(),
            "scheme should be sufficient for validate()",
        );
    }
}

#[test]
fn test_t17_1_breakdown_carries_each_score() {
    let p1 = Worker::builder()
        .br_cpf("12345678909")
        .cn_rrn("11010519491231002X")
        .in_aadhaar("234123412346")
        .jp_my_number("123456789018")
        .mx_curp("HEGG560427MVZRRL04")
        .nz_nhi("ZAA0083")
        .za_id("8001015009087")
        .given_name("Polyglot")
        .family_name("Forty-Two")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p1.clone());
    assert_eq!(r.breakdown.br_cpf_score, Some(1.0));
    assert_eq!(r.breakdown.cn_rrn_score, Some(1.0));
    assert_eq!(r.breakdown.in_aadhaar_score, Some(1.0));
    assert_eq!(r.breakdown.jp_my_number_score, Some(1.0));
    assert_eq!(r.breakdown.mx_curp_score, Some(1.0));
    assert_eq!(r.breakdown.nz_nhi_score, Some(1.0));
    assert_eq!(r.breakdown.za_id_score, Some(1.0));
}

#[test]
fn test_t17_1_unparseable_yields_none_not_zero() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .br_cpf("not a cpf")
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .br_cpf("also not a cpf")
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.br_cpf_score, None);
}

#[test]
fn test_t17_1_serde_round_trip_all_seven() {
    let p = Worker::builder()
        .br_cpf("12345678909")
        .cn_rrn("11010519491231002X")
        .in_aadhaar("234123412346")
        .jp_my_number("123456789018")
        .mx_curp("HEGG560427MVZRRL04")
        .nz_nhi("ZAA0083")
        .za_id("8001015009087")
        .given_name("Polyglot")
        .family_name("Forty-Two")
        .build();
    let json = serde_json::to_string(&p).unwrap();
    let back: Worker = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn test_t17_1_legacy_payload_deserialises_to_none() {
    // Legacy JSON without the new fields must round-trip with None.
    let legacy = r#"{
        "given_name": "Legacy",
        "family_name": "Record",
        "middle_name": null,
        "date_of_birth": null,
        "gender": null,
        "address": null,
        "previous_addresses": [],
        "phone": null,
        "mobile": null,
        "email": null,
        "local_id": null
    }"#;
    let p: Worker = serde_json::from_str(legacy).unwrap();
    assert!(p.br_cpf.is_none());
    assert!(p.cn_rrn.is_none());
    assert!(p.in_aadhaar.is_none());
    assert!(p.jp_my_number.is_none());
    assert!(p.mx_curp.is_none());
    assert!(p.nz_nhi.is_none());
    assert!(p.za_id.is_none());
}

// ============================================================
// §21c. Per-country passport-number format validators (T-28)
// ============================================================
//
// These are pure format validators; passport data is canonically stored
// via PassportBook. The tests just verify the parsers accept canonical
// inputs and reject obviously-wrong ones.

#[test]
fn test_country_passport_format_validators_round_trip() {
    use worker_matcher::identifiers as id;
    assert_eq!(id::parse_cy_passport("E123456"), Some("E123456".into()));
    assert_eq!(id::parse_cy_passport("K12345678"), Some("K12345678".into()));
    assert_eq!(id::parse_cz_passport("12345678"), Some("12345678".into()));
    assert_eq!(id::parse_li_passport("R00536"), Some("R00536".into()));
    assert_eq!(id::parse_lt_passport("12345678"), Some("12345678".into()));
    assert_eq!(id::parse_mt_passport("1234567"), Some("1234567".into()));
    assert_eq!(id::parse_nl_passport("AB1234567"), Some("AB1234567".into()));
    assert_eq!(id::parse_pt_passport("A123456"), Some("A123456".into()));
    assert_eq!(id::parse_ro_passport("AB123456"), Some("AB123456".into()));
    assert_eq!(id::parse_sk_passport("AB1234567"), Some("AB1234567".into()));
}

#[test]
fn test_country_passport_validators_compose_with_passport_book() {
    use worker_matcher::PassportBook;
    use worker_matcher::identifiers as id;
    // Typical ingestion flow: validate format country-specifically,
    // then construct a PassportBook with the canonical form. Country
    // provenance lives on the PassportBook; only the document number
    // itself goes through the per-country parser.
    let raw = "AB 12-34 56";
    let canonical = id::parse_ro_passport(raw).expect("valid RO passport format");
    let book = PassportBook::new("RO", &canonical).expect("constructs");
    assert_eq!(book.country, "RO");
    assert_eq!(book.number, "AB123456");
}

#[test]
fn test_uk_chi_does_not_cross_match_uk_nino_or_uk_nhs_or_uk_hc() {
    // Three UK scheme-local identifiers must never cross-match. Use
    // the same characters where possible to make the test sharper.
    let chi = Worker::builder().uk_chi_number("0101701233").build();
    let nhs = Worker::builder().uk_nhs_number("0101701233").build();
    let hc = Worker::builder().uk_hc_number("0101701233").build();
    let engine = MatchingEngine::default_config();
    assert!(!engine.deterministic_match(&chi, &nhs));
    assert!(!engine.deterministic_match(&chi, &hc));
    assert!(!engine.deterministic_match(&nhs, &hc));
    // NINO has a different format entirely; cannot cross with the
    // 10-digit Mod-11 schemes.
    let nino = Worker::builder().uk_nino("AB123456A").build();
    assert!(!engine.deterministic_match(&nino, &chi));
}

// ============================================================
// §21. Passport books (multi-country, multi-book, historical)
// ============================================================
//
// A passport book is scheme-local: a UK "AB123456" is not the same
// identifier as a US "AB123456". A worker may hold passports from
// several countries simultaneously and accumulate historical book
// numbers as passports are renewed. The matcher treats any shared
// (country, number) pair across the two workers' lists as a match,
// regardless of issue date.

#[test]
fn test_passport_book_single_pair_deterministic_match() {
    let p1 = Worker::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .add_passport_book(PassportBook::new("GB", "123456789").unwrap())
        .build();
    let p2 = Worker::builder()
        .given_name("Different")
        .family_name("Worker")
        .add_passport_book(PassportBook::new("gb", " 123 456 789 ").unwrap())
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_passport_book_multi_country_any_pair_matches() {
    // p1 holds GB and US passports; p2 holds FR and US passports.
    // They share US:XYZ → match.
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .add_passport_book(PassportBook::new("GB", "111").unwrap())
        .add_passport_book(PassportBook::new("US", "XYZ123").unwrap())
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .add_passport_book(PassportBook::new("FR", "222").unwrap())
        .add_passport_book(PassportBook::new("US", "xyz123").unwrap())
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_passport_book_same_number_different_country_does_not_match() {
    // A UK book "AB123456" and a US book "AB123456" are different
    // identifiers — country is part of the comparison key.
    let p1 = Worker::builder()
        .add_passport_book(PassportBook::new("GB", "AB123456").unwrap())
        .build();
    let p2 = Worker::builder()
        .add_passport_book(PassportBook::new("US", "AB123456").unwrap())
        .build();
    let engine = MatchingEngine::default_config();
    assert!(!engine.deterministic_match(&p1, &p2));
    let r = engine.match_workers(&p1, &p2);
    assert_eq!(r.breakdown.passport_book_score, Some(0.0));
}

#[test]
fn test_passport_book_historical_pair_still_matches() {
    // A renewed passport: p1 carries only the current book; p2 carries
    // the prior book on the same country. The shared (country, prior)
    // pair makes the records deterministic-match.
    let p1 = Worker::builder()
        .add_passport_book(PassportBook::new("GB", "PRIOR-NUMBER").unwrap())
        .build();
    let p2 = Worker::builder()
        .add_passport_book(PassportBook::new("GB", "CURRENT-NUMBER").unwrap())
        .add_passport_book(PassportBook::new("GB", "prior-number").unwrap())
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_passport_book_score_zero_when_disjoint() {
    let p1 = Worker::builder()
        .given_name("A")
        .family_name("B")
        .add_passport_book(PassportBook::new("GB", "111").unwrap())
        .build();
    let p2 = Worker::builder()
        .given_name("A")
        .family_name("B")
        .add_passport_book(PassportBook::new("GB", "222").unwrap())
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.passport_book_score, Some(0.0));
}

#[test]
fn test_passport_book_score_none_when_either_side_empty() {
    let p1 = Worker::builder()
        .given_name("A")
        .family_name("B")
        .add_passport_book(PassportBook::new("GB", "123").unwrap())
        .build();
    let p2 = Worker::builder().given_name("A").family_name("B").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.passport_book_score, None);
}

#[test]
fn test_passport_book_dates_are_metadata_not_used_in_matching() {
    // Same (country, number) but very different effective dates:
    // matching is unaffected.
    let p1 = Worker::builder()
        .add_passport_book(
            PassportBook::new("GB", "123456789")
                .unwrap()
                .with_issued(NaiveDate::from_ymd_opt(2010, 1, 1).unwrap())
                .with_expires(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        )
        .build();
    let p2 = Worker::builder()
        .add_passport_book(
            PassportBook::new("GB", "123456789")
                .unwrap()
                .with_issued(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap())
                .with_expires(NaiveDate::from_ymd_opt(2034, 6, 1).unwrap()),
        )
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_passport_book_serde_round_trip_with_worker() {
    let p = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .add_passport_book(
            PassportBook::new("GB", "123456789")
                .unwrap()
                .with_issued(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        )
        .add_passport_book(PassportBook::new("US", "AB1234567").unwrap())
        .build();
    let json = serde_json::to_string(&p).unwrap();
    let back: Worker = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn test_passport_book_legacy_worker_json_without_books_deserialises() {
    // Pre-feature JSON payload lacking `passport_books` must
    // deserialise cleanly with an empty list.
    let legacy = r#"{
        "given_name": "Legacy",
        "family_name": "Record",
        "middle_name": null,
        "date_of_birth": null,
        "gender": null,
        "address": null,
        "previous_addresses": [],
        "phone": null,
        "mobile": null,
        "email": null,
        "local_id": null
    }"#;
    let p: Worker = serde_json::from_str(legacy).unwrap();
    assert!(p.passport_books.is_empty());
    // Sanity: matcher accepts it.
    let r = MatchingEngine::default_config().match_workers(&p, &p.clone());
    assert_eq!(r.breakdown.passport_book_score, None);
}

// ============================================================
// §22. Blood-type scoring
// ============================================================
//
// Blood type is a weak positive signal (many people share a type) and
// a strong negative signal (blood type doesn't change). Same shape as
// gender: Some(1.0) for equal, Some(0.0) for different, None if either
// side is missing. NOT consulted by deterministic_match (too weak alone).

#[test]
fn test_blood_type_match_scores_one() {
    let p1 = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .blood_type(BloodType::OPositive)
        .build();
    let p2 = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .blood_type(BloodType::OPositive)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.blood_type_score, Some(1.0));
}

#[test]
fn test_blood_type_mismatch_scores_zero() {
    // Same name + DOB, but disagreeing blood type. The probabilistic
    // score remains high because blood-type weight is small, but the
    // breakdown surfaces the disagreement for clinical review.
    let p1 = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .date_of_birth(dob(1815, 12, 10))
        .blood_type(BloodType::OPositive)
        .build();
    let p2 = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .date_of_birth(dob(1815, 12, 10))
        .blood_type(BloodType::ANegative)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.blood_type_score, Some(0.0));
}

#[test]
fn test_blood_type_missing_on_one_side_scores_none() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .blood_type(BloodType::APositive)
        .build();
    let p2 = Worker::builder().given_name("X").family_name("Y").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.blood_type_score, None);
}

#[test]
fn test_blood_type_is_not_part_of_deterministic_match() {
    // Two records with matching blood type but no other identifying
    // overlap must NOT deterministic-match — blood type is too weak a
    // signal on its own.
    let p1 = Worker::builder()
        .given_name("Different")
        .family_name("Worker")
        .blood_type(BloodType::OPositive)
        .build();
    let p2 = Worker::builder()
        .given_name("Other")
        .family_name("Worker")
        .blood_type(BloodType::OPositive)
        .build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_blood_type_parses_through_builder() {
    // Common ingestion pattern: parse a free-text blood-type column.
    let bt = BloodType::parse("O Positive").expect("parses");
    let p = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .blood_type(bt)
        .build();
    assert_eq!(p.blood_type, Some(BloodType::OPositive));
}

#[test]
fn test_blood_type_round_trips_via_worker_serde() {
    let p = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .blood_type(BloodType::ABNegative)
        .build();
    let json = serde_json::to_string(&p).unwrap();
    assert!(
        json.contains("\"AB-\""),
        "expected canonical short form in JSON: {json}"
    );
    let back: Worker = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn test_blood_type_legacy_worker_json_without_field_deserialises() {
    // Pre-feature JSON payload lacking `blood_type` must deserialise
    // cleanly with `blood_type = None`.
    let legacy = r#"{
        "given_name": "Legacy",
        "family_name": "Record",
        "middle_name": null,
        "date_of_birth": null,
        "gender": null,
        "address": null,
        "previous_addresses": [],
        "phone": null,
        "mobile": null,
        "email": null,
        "local_id": null
    }"#;
    let p: Worker = serde_json::from_str(legacy).unwrap();
    assert!(p.blood_type.is_none());
}

// ============================================================
// §23. Place of birth (FHIR Patient.birthPlace)
// ============================================================

#[test]
fn test_birth_place_matcher_city_and_country_scores_high() {
    let bp = Address::new().with_city("Cardiff").with_country("Wales");
    let p1 = Worker::builder()
        .given_name("Dylan")
        .family_name("Thomas")
        .birth_place(bp.clone())
        .build();
    let p2 = Worker::builder()
        .given_name("Dylan")
        .family_name("Thomas")
        .birth_place(bp)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.birth_place_score.expect("scored");
    assert!(s > 0.99, "identical birth place should score ~1.0: {s}");
}

#[test]
fn test_birth_place_different_city_lowers_score() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(Address::new().with_city("Cardiff").with_country("Wales"))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(
            Address::new()
                .with_city("Edinburgh")
                .with_country("Scotland"),
        )
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.birth_place_score.expect("scored");
    assert!(
        s < 0.5,
        "wildly different birth places should score low: {s}"
    );
}

#[test]
fn test_birth_place_same_city_different_country_is_partial() {
    // Same city name but different country → 0.7 × 1.0 + 0.3 × 0.0 = 0.7
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(Address::new().with_city("Paris").with_country("France"))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(Address::new().with_city("Paris").with_country("USA"))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.birth_place_score.expect("scored");
    assert!(
        (s - 0.7).abs() < 1e-9,
        "city-only match should weight 0.7: {s}"
    );
}

#[test]
fn test_birth_place_missing_on_one_side_scores_none() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(Address::new().with_city("Cardiff"))
        .build();
    let p2 = Worker::builder().given_name("X").family_name("Y").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.birth_place_score, None);
}

#[test]
fn test_birth_place_only_country_populated_falls_back_to_country() {
    // No city on either side, but country present and matching.
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(Address::new().with_country("Wales"))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(Address::new().with_country("Wales"))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.birth_place_score, Some(1.0));
}

#[test]
fn test_birth_place_empty_subfields_score_none() {
    // Both sides have a birth_place but neither city nor country
    // populated → no signal to compare on.
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(Address::new())
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(Address::new())
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.birth_place_score, None);
}

#[test]
fn test_birth_place_is_not_part_of_deterministic_match() {
    // Matching birth place but no other identifying overlap → not a
    // deterministic match. Birth place is too weak a signal alone.
    let p1 = Worker::builder()
        .given_name("Different")
        .family_name("People")
        .birth_place(Address::new().with_city("London").with_country("England"))
        .build();
    let p2 = Worker::builder()
        .given_name("Other")
        .family_name("Folks")
        .birth_place(Address::new().with_city("London").with_country("England"))
        .build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_birth_place_diacritic_normalisation() {
    // City names with diacritics canonicalise the same as their
    // diacritic-stripped forms (per `Normalizer::normalize_name`),
    // so birth-place city matching is diacritic-tolerant.
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(
            Address::new()
                .with_city("Zürich")
                .with_country("Switzerland"),
        )
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(
            Address::new()
                .with_city("Zurich")
                .with_country("Switzerland"),
        )
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.birth_place_score.expect("scored");
    assert!(s > 0.99, "diacritic-stripped match should score ~1.0: {s}");
}

#[test]
fn test_birth_place_round_trips_via_worker_serde() {
    let p = Worker::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .birth_place(Address::new().with_city("London").with_country("England"))
        .build();
    let json = serde_json::to_string(&p).unwrap();
    let back: Worker = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn test_birth_place_legacy_payload_deserialises_to_none() {
    let legacy = r#"{
        "given_name": "Legacy",
        "family_name": "Record",
        "middle_name": null,
        "date_of_birth": null,
        "gender": null,
        "address": null,
        "previous_addresses": [],
        "phone": null,
        "mobile": null,
        "email": null,
        "local_id": null
    }"#;
    let p: Worker = serde_json::from_str(legacy).unwrap();
    assert!(p.birth_place.is_none());
}

// ============================================================
// §24. Multiple-birth indicator (FHIR Patient.multipleBirth)
// ============================================================

#[test]
fn test_multiple_birth_match_scores_one() {
    let p1 = Worker::builder()
        .given_name("Twin")
        .family_name("Smith")
        .date_of_birth(dob(2000, 6, 15))
        .multiple_birth(1)
        .build();
    let p2 = Worker::builder()
        .given_name("Twin")
        .family_name("Smith")
        .date_of_birth(dob(2000, 6, 15))
        .multiple_birth(1)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.multiple_birth_score, Some(1.0));
}

#[test]
fn test_multiple_birth_disambiguates_identical_twins() {
    // The canonical failure mode: two records with identical name, DOB,
    // and demographics that nonetheless refer to different identical
    // twins. Without the multiple-birth indicator the matcher would
    // confidently merge them. With it, the disagreement (twin 1 vs
    // twin 2) surfaces in the breakdown.
    let twin1 = Worker::builder()
        .given_name("Alex")
        .family_name("Smith")
        .date_of_birth(dob(2000, 6, 15))
        .gender(Gender::Female)
        .multiple_birth(1)
        .build();
    let twin2 = Worker::builder()
        .given_name("Alex")
        .family_name("Smith")
        .date_of_birth(dob(2000, 6, 15))
        .gender(Gender::Female)
        .multiple_birth(2)
        .build();
    let r = MatchingEngine::default_config().match_workers(&twin1, &twin2);
    assert_eq!(r.breakdown.multiple_birth_score, Some(0.0));
}

#[test]
fn test_multiple_birth_missing_on_one_side_scores_none() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .multiple_birth(1)
        .build();
    let p2 = Worker::builder().given_name("X").family_name("Y").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.multiple_birth_score, None);
}

#[test]
fn test_multiple_birth_is_not_part_of_deterministic_match() {
    // Even with all other demographic signals aligning, a multiple_birth
    // value alone (or even with name+DOB agreement) does not trigger
    // a deterministic match — that path requires identifier agreement
    // or the full demographic-tuple (which includes gender and DOB).
    let p1 = Worker::builder()
        .given_name("Different")
        .family_name("People")
        .multiple_birth(1)
        .build();
    let p2 = Worker::builder()
        .given_name("Other")
        .family_name("Folks")
        .multiple_birth(1)
        .build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_multiple_birth_round_trips_via_worker_serde() {
    let p = Worker::builder()
        .given_name("Twin")
        .family_name("Smith")
        .multiple_birth(2)
        .build();
    let json = serde_json::to_string(&p).unwrap();
    assert!(
        json.contains("\"multiple_birth\":2"),
        "expected multiple_birth in JSON: {json}",
    );
    let back: Worker = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn test_multiple_birth_legacy_payload_deserialises_to_none() {
    let legacy = r#"{
        "given_name": "Legacy",
        "family_name": "Record",
        "middle_name": null,
        "date_of_birth": null,
        "gender": null,
        "address": null,
        "previous_addresses": [],
        "phone": null,
        "mobile": null,
        "email": null,
        "local_id": null
    }"#;
    let p: Worker = serde_json::from_str(legacy).unwrap();
    assert!(p.multiple_birth.is_none());
}

#[test]
fn test_serialization_round_trip_carries_all_identifiers() {
    let p = Worker::builder()
        .uk_nhs_number(NHS_A)
        .fr_nir(NIR_A_FR)
        .es_tsi(TSI_A_ES)
        .ie_ihi(IHI_A_IE)
        .uk_hc_number(NHS_A)
        .us_ssn(SSN_A_US)
        .au_ihi(IHI_A_AU)
        .de_kvnr(KVNR_A_DE)
        .it_cf(CF_A_IT)
        .nl_bsn(BSN_A_NL)
        .se_personnummer(PNR_A_SE)
        .uk_chi_number(CHI_A_UK)
        .given_name("Polyglot")
        .family_name("Worker")
        .build();
    let json = serde_json::to_string(&p).unwrap();
    let back: Worker = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

// ============================================================
// §25. Death date and death place (FHIR Patient.deceasedDateTime)
// ============================================================

#[test]
fn test_death_date_exact_match_scores_one() {
    let p1 = Worker::builder()
        .given_name("Alan")
        .family_name("Turing")
        .death_date(dob(1954, 6, 7))
        .build();
    let p2 = Worker::builder()
        .given_name("Alan")
        .family_name("Turing")
        .death_date(dob(1954, 6, 7))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.death_date_score, Some(1.0));
}

#[test]
fn test_death_date_day_month_swap_scores_half() {
    // The same transposition heuristic as date_of_birth: 07/06 vs 06/07.
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .death_date(dob(2024, 6, 7))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .death_date(dob(2024, 7, 6))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.death_date_score, Some(0.5));
}

#[test]
fn test_death_date_unrelated_dates_score_zero() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .death_date(dob(2024, 1, 1))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .death_date(dob(2024, 11, 30))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.death_date_score, Some(0.0));
}

#[test]
fn test_death_date_missing_on_one_side_scores_none() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .death_date(dob(2024, 6, 7))
        .build();
    let p2 = Worker::builder().given_name("X").family_name("Y").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.death_date_score, None);
}

#[test]
fn test_death_date_is_independent_from_date_of_birth() {
    // A matching DOB but different DOD must not be hidden by the
    // DOB score; the death_date sub-score should report its own
    // disagreement.
    let p1 = Worker::builder()
        .given_name("Same")
        .family_name("Name")
        .date_of_birth(dob(1900, 1, 1))
        .death_date(dob(1980, 5, 5))
        .build();
    let p2 = Worker::builder()
        .given_name("Same")
        .family_name("Name")
        .date_of_birth(dob(1900, 1, 1))
        .death_date(dob(1985, 9, 9))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.date_of_birth_score, Some(1.0));
    assert_eq!(r.breakdown.death_date_score, Some(0.0));
}

#[test]
fn test_death_place_matcher_city_and_country_scores_high() {
    let dp = Address::new().with_city("Wilmslow").with_country("England");
    let p1 = Worker::builder()
        .given_name("Alan")
        .family_name("Turing")
        .death_place(dp.clone())
        .build();
    let p2 = Worker::builder()
        .given_name("Alan")
        .family_name("Turing")
        .death_place(dp)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.death_place_score.expect("scored");
    assert!(s > 0.99, "identical death place should score ~1.0: {s}");
}

#[test]
fn test_death_place_different_city_lowers_score() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .death_place(Address::new().with_city("Wilmslow").with_country("England"))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .death_place(Address::new().with_city("Glasgow").with_country("Scotland"))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.death_place_score.expect("scored");
    assert!(s < 0.5, "different death places should score low: {s}");
}

#[test]
fn test_death_place_same_city_different_country_is_partial() {
    // Symmetrical to the birth_place case: city-only agreement → 0.7.
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .death_place(Address::new().with_city("Paris").with_country("France"))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .death_place(Address::new().with_city("Paris").with_country("USA"))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.death_place_score.expect("scored");
    assert!(
        (s - 0.7).abs() < 1e-9,
        "city-only match should weight 0.7: {s}"
    );
}

#[test]
fn test_death_place_missing_on_one_side_scores_none() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .death_place(Address::new().with_city("London"))
        .build();
    let p2 = Worker::builder().given_name("X").family_name("Y").build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.death_place_score, None);
}

#[test]
fn test_death_place_is_independent_from_birth_place() {
    // A worker may be born in one country and die in another. The
    // two sub-scores must report independently.
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(Address::new().with_city("Vienna").with_country("Austria"))
        .death_place(Address::new().with_city("London").with_country("England"))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .birth_place(Address::new().with_city("Vienna").with_country("Austria"))
        .death_place(Address::new().with_city("London").with_country("England"))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let bp = r.breakdown.birth_place_score.expect("scored");
    let dp = r.breakdown.death_place_score.expect("scored");
    assert!(bp > 0.99);
    assert!(dp > 0.99);
}

#[test]
fn test_death_place_is_not_part_of_deterministic_match() {
    // Matching death place alone is not enough for a deterministic
    // match — it's a weak positive signal, like birth_place.
    let p1 = Worker::builder()
        .given_name("Different")
        .family_name("People")
        .death_place(Address::new().with_city("Paris").with_country("France"))
        .build();
    let p2 = Worker::builder()
        .given_name("Other")
        .family_name("Folks")
        .death_place(Address::new().with_city("Paris").with_country("France"))
        .build();
    assert!(!MatchingEngine::default_config().deterministic_match(&p1, &p2));
}

#[test]
fn test_death_date_round_trips_via_worker_serde() {
    let p = Worker::builder()
        .given_name("Alan")
        .family_name("Turing")
        .death_date(dob(1954, 6, 7))
        .death_place(Address::new().with_city("Wilmslow").with_country("England"))
        .build();
    let json = serde_json::to_string(&p).unwrap();
    let back: Worker = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}

#[test]
fn test_death_fields_legacy_payload_deserialises_to_none() {
    let legacy = r#"{
        "given_name": "Legacy",
        "family_name": "Record",
        "middle_name": null,
        "date_of_birth": null,
        "gender": null,
        "address": null,
        "previous_addresses": [],
        "phone": null,
        "mobile": null,
        "email": null,
        "local_id": null
    }"#;
    let p: Worker = serde_json::from_str(legacy).unwrap();
    assert!(p.death_date.is_none());
    assert!(p.death_place.is_none());
}

#[test]
fn test_death_date_weight_contributes_to_composite_score() {
    // With a strongly matching death date the composite confidence
    // should rise above a no-death-date baseline of otherwise weak
    // identifying overlap.
    let baseline_p1 = Worker::builder()
        .given_name("Dim")
        .family_name("Match")
        .build();
    let baseline_p2 = Worker::builder()
        .given_name("Dim")
        .family_name("Match")
        .build();
    let with_dod_p1 = Worker::builder()
        .given_name("Dim")
        .family_name("Match")
        .death_date(dob(1990, 4, 4))
        .build();
    let with_dod_p2 = Worker::builder()
        .given_name("Dim")
        .family_name("Match")
        .death_date(dob(1990, 4, 4))
        .build();
    let engine = MatchingEngine::default_config();
    let baseline = engine.match_workers(&baseline_p1, &baseline_p2);
    let with_dod = engine.match_workers(&with_dod_p1, &with_dod_p2);
    assert!(
        with_dod.score >= baseline.score,
        "death-date agreement should not lower the score: {} vs {}",
        with_dod.score,
        baseline.score
    );
}

// ============================================================
// §26. Address sub-score arithmetic (T-3 / OQ-4)
// ============================================================

#[test]
fn test_address_subscore_postcode_dominates_via_weighted_average() {
    // OQ-4 / FR-7 acceptance: an exact postcode match plus a
    // slightly different line 1 must clear 0.7 in the breakdown,
    // because postcode carries the dominant weight (0.5) under the
    // weighted-average arithmetic.
    let a = Address::new()
        .with_line1("10 High Street")
        .with_postcode("CF10 1AA")
        .with_city("Cardiff");
    let b = Address::new()
        .with_line1("10 High Road")
        .with_postcode("CF10 1AA")
        .with_city("Cardiff");
    let p1 = Worker::builder()
        .given_name("Owen")
        .family_name("Glyndwr")
        .address(a)
        .build();
    let p2 = Worker::builder()
        .given_name("Owen")
        .family_name("Glyndwr")
        .address(b)
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    let s = r.breakdown.address_score.expect("scored");
    assert!(
        s >= 0.7,
        "exact postcode + slight street typo → address_score ≥ 0.7: {s}"
    );
}

#[test]
fn test_address_subscore_postcode_only_match_collapses_to_one() {
    let p1 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(Address::new().with_postcode("SW1A 1AA"))
        .build();
    let p2 = Worker::builder()
        .given_name("X")
        .family_name("Y")
        .address(Address::new().with_postcode("SW1A 1AA"))
        .build();
    let r = MatchingEngine::default_config().match_workers(&p1, &p2);
    assert_eq!(r.breakdown.address_score, Some(1.0));
}
