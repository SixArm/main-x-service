//! # Worker matcher demo
//!
//! Command-line demonstration of the [`worker_matcher`] crate. Runs a handful
//! of illustrative worker-record comparisons and prints the deterministic and
//! probabilistic results. See the library crate for the matching API.

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use worker_matcher::{Address, Gender, MatchConfig, MatchingEngine, Worker};

// A flat sequence of illustrative demo scenarios; length is inherent to a
// walkthrough binary, not a sign of tangled logic.
#[allow(clippy::too_many_lines)]
fn main() {
    println!("Worker matcher");
    println!("================\n");

    // Example 1: Perfect match
    println!("Example 1: Perfect Match");
    let worker1 = Worker::builder()
        .uk_nhs_number("1234567890")
        .given_name("Dafydd")
        .family_name("Jones")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1980, 5, 15).expect("valid date"))
        .gender(Gender::Male)
        .build();

    let worker2 = worker1.clone();

    let engine = MatchingEngine::default_config();
    let result = engine.match_workers(&worker1, &worker2);

    println!("Match Score: {:.2}", result.score);
    println!("Is Match: {}", result.is_match);

    // Example 2: Fuzzy name match
    println!("Example 2: Fuzzy Name Match (Stephen vs Steven)");
    let patient3 = Worker::builder()
        .given_name("Stephen")
        .family_name("Williams")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1975, 8, 22).expect("valid date"))
        .gender(Gender::Male)
        .build();

    let patient4 = Worker::builder()
        .given_name("Steven") // Different spelling
        .family_name("Williams")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1975, 8, 22).expect("valid date"))
        .gender(Gender::Male)
        .build();

    let result2 = engine.match_workers(&patient3, &patient4);
    println!("Match Score: {:.2}", result2.score);
    println!("Is Match: {}", result2.is_match);
    println!(
        "Given Name Score: {:.2}",
        result2.breakdown.given_name_score.unwrap()
    );
    println!(
        "Phonetic Match: {:?}\n",
        result2.breakdown.phonetic_name_score
    );

    // Example 3: name with diacritics
    println!("Example 3: Name with Diacritics");
    let patient5 = Worker::builder()
        .given_name("Siân") // With diacritic
        .family_name("Evans")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1990, 3, 10).expect("valid date"))
        .gender(Gender::Female)
        .build();

    let patient6 = Worker::builder()
        .given_name("Sian") // Without diacritic
        .family_name("Evans")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1990, 3, 10).expect("valid date"))
        .gender(Gender::Female)
        .build();

    let result3 = engine.match_workers(&patient5, &patient6);
    println!("Match Score: {:.2}", result3.score);
    println!("Is Match: {}", result3.is_match);
    println!();

    // Example 4: Address matching
    println!("Example 4: Address Matching");
    let mut address1 = Address::new();
    address1.line1 = Some("10 Downing Street".to_string());
    address1.city = Some("Riverside".to_string());
    address1.postcode = Some("CF10 1AA".to_string());

    let mut address2 = Address::new();
    address2.line1 = Some("10 Downing St".to_string());
    address2.city = Some("Riverside".to_string());
    address2.postcode = Some("CF10 1AA".to_string());

    let patient7 = Worker::builder()
        .given_name("Emma")
        .family_name("Davies")
        .address(address1)
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1985, 12, 1).expect("valid date"))
        .build();

    let patient8 = Worker::builder()
        .given_name("Emma")
        .family_name("Davies")
        .address(address2)
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1985, 12, 1).expect("valid date"))
        .build();

    let result4 = engine.match_workers(&patient7, &patient8);
    println!("Match Score: {:.2}", result4.score);
    println!(
        "Address Score: {:.2}",
        result4.breakdown.address_score.unwrap()
    );
    println!();

    // Example 5: No match
    println!("Example 5: Different Patients (No Match)");
    let patient9 = Worker::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1990, 1, 1).expect("valid date"))
        .gender(Gender::Female)
        .build();

    let patient10 = Worker::builder()
        .given_name("Zachary")
        .family_name("Zimmerman")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(2000, 12, 31).expect("valid date"))
        .gender(Gender::Male)
        .build();

    let result5 = engine.match_workers(&patient9, &patient10);
    println!("Match Score: {:.2}", result5.score);
    println!("Is Match: {}", result5.is_match);
    println!();

    // Example 6: Strict vs Lenient mode
    println!("Example 6: Strict vs Lenient Matching");
    let patient11 = Worker::builder()
        .given_name("Michael") // Formal name (not nickname)
        .family_name("Thomas")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1978, 6, 15).expect("valid date"))
        .build();

    let patient12 = Worker::builder()
        .given_name("Mike") // Nickname (not formal name)
        .family_name("Thomas")
        .date_of_birth(chrono::NaiveDate::from_ymd_opt(1978, 6, 15).expect("valid date"))
        .build();

    let strict_engine = MatchingEngine::new(MatchConfig::strict());
    let lenient_engine = MatchingEngine::new(MatchConfig::lenient());

    let strict_result = strict_engine.match_workers(&patient11, &patient12);
    let lenient_result = lenient_engine.match_workers(&patient11, &patient12);

    println!(
        "Strict Mode Score: {:.2} (Match: {})",
        strict_result.score, strict_result.is_match
    );
    println!(
        "Lenient Mode Score: {:.2} (Match: {})",
        lenient_result.score, lenient_result.is_match
    );
}
