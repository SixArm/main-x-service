//! Person matcher demo binary.
//!
//! Smoke-tests the `person_matcher` library by running a handful of
//! illustrative matches against the public API. This is not part of the
//! library's `SemVer` surface; see `lib.rs` for the supported types.

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
// The demo `main` is one long sequence of illustrative examples by design.
#![allow(clippy::too_many_lines)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use person_matcher::{Address, Gender, MatchConfig, MatchingEngine, Person};

fn main() {
    println!("Person matcher");
    println!("================\n");

    // Example 1: Perfect match
    println!("Example 1: Perfect Match");
    let person1 = Person::builder()
        .united_kingdom_national_health_service_number("1234567890")
        .given_name("Dafydd")
        .family_name("Jones")
        .date_of_birth(jiff::civil::date(1980, 5, 15))
        .gender(Gender::Male)
        .build();

    let person2 = person1.clone();

    let engine = MatchingEngine::default_config();
    let result = engine.match_persons(&person1, &person2);

    println!("Match Score: {:.2}", result.score);
    println!("Is Match: {}", result.is_match);

    // Example 2: Fuzzy name match
    println!("Example 2: Fuzzy Name Match (Stephen vs Steven)");
    let patient3 = Person::builder()
        .given_name("Stephen")
        .family_name("Williams")
        .date_of_birth(jiff::civil::date(1975, 8, 22))
        .gender(Gender::Male)
        .build();

    let patient4 = Person::builder()
        .given_name("Steven") // Different spelling
        .family_name("Williams")
        .date_of_birth(jiff::civil::date(1975, 8, 22))
        .gender(Gender::Male)
        .build();

    let result2 = engine.match_persons(&patient3, &patient4);
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
    let patient5 = Person::builder()
        .given_name("Siân") // With diacritic
        .family_name("Evans")
        .date_of_birth(jiff::civil::date(1990, 3, 10))
        .gender(Gender::Female)
        .build();

    let patient6 = Person::builder()
        .given_name("Sian") // Without diacritic
        .family_name("Evans")
        .date_of_birth(jiff::civil::date(1990, 3, 10))
        .gender(Gender::Female)
        .build();

    let result3 = engine.match_persons(&patient5, &patient6);
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

    let patient7 = Person::builder()
        .given_name("Emma")
        .family_name("Davies")
        .address(address1)
        .date_of_birth(jiff::civil::date(1985, 12, 1))
        .build();

    let patient8 = Person::builder()
        .given_name("Emma")
        .family_name("Davies")
        .address(address2)
        .date_of_birth(jiff::civil::date(1985, 12, 1))
        .build();

    let result4 = engine.match_persons(&patient7, &patient8);
    println!("Match Score: {:.2}", result4.score);
    println!(
        "Address Score: {:.2}",
        result4.breakdown.address_score.unwrap()
    );
    println!();

    // Example 5: No match
    println!("Example 5: Different Patients (No Match)");
    let patient9 = Person::builder()
        .given_name("Alice")
        .family_name("Anderson")
        .date_of_birth(jiff::civil::date(1990, 1, 1))
        .gender(Gender::Female)
        .build();

    let patient10 = Person::builder()
        .given_name("Zachary")
        .family_name("Zimmerman")
        .date_of_birth(jiff::civil::date(2000, 12, 31))
        .gender(Gender::Male)
        .build();

    let result5 = engine.match_persons(&patient9, &patient10);
    println!("Match Score: {:.2}", result5.score);
    println!("Is Match: {}", result5.is_match);
    println!();

    // Example 6: Strict vs Lenient mode
    println!("Example 6: Strict vs Lenient Matching");
    let patient11 = Person::builder()
        .given_name("Michael") // Formal name (not nickname)
        .family_name("Thomas")
        .date_of_birth(jiff::civil::date(1978, 6, 15))
        .build();

    let patient12 = Person::builder()
        .given_name("Mike") // Nickname (not formal name)
        .family_name("Thomas")
        .date_of_birth(jiff::civil::date(1978, 6, 15))
        .build();

    let strict_engine = MatchingEngine::new(MatchConfig::strict());
    let lenient_engine = MatchingEngine::new(MatchConfig::lenient());

    let strict_result = strict_engine.match_persons(&patient11, &patient12);
    let lenient_result = lenient_engine.match_persons(&patient11, &patient12);

    println!(
        "Strict Mode Score: {:.2} (Match: {})",
        strict_result.score, strict_result.is_match
    );
    println!(
        "Lenient Mode Score: {:.2} (Match: {})",
        lenient_result.score, lenient_result.is_match
    );
}
