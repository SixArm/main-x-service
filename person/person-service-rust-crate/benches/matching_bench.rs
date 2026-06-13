#![warn(clippy::pedantic)]

//! Criterion benchmarks for the person matching engine.
//!
//! Covers the per-field algorithms (name, DOB, gender, address,
//! phonetic, tax-id, document), the full [`ProbabilisticMatcher`] and
//! [`DeterministicMatcher`] on a single pair, and `find_matches`
//! scaling across 10/100/1000 candidates. Run with `cargo bench`.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use jiff::{Timestamp, civil::Date};
use uuid::Uuid;

use person_service::config::MatchingConfig;
use person_service::matching::algorithms::{
    address_matching, dob_matching, document_matching, gender_matching, name_matching,
    tax_id_matching,
};
use person_service::matching::phonetic;
use person_service::matching::*;
use person_service::models::*;

/// Build a minimal [`Person`] with the given family/given names and DOB.
fn create_test_person(family: &str, given: &str, birth_date: Option<Date>) -> Person {
    let now = Timestamp::now();
    Person {
        id: Uuid::new_v4(),
        identifiers: vec![],
        active: true,
        name: HumanName {
            use_type: None,
            family: family.to_string(),
            given: vec![given.to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        additional_names: vec![],
        telecom: vec![],
        gender: Gender::Male,
        birth_date,
        tax_id: None,
        documents: vec![],
        emergency_contacts: vec![],
        deceased: false,
        deceased_datetime: None,
        addresses: vec![],
        marital_status: None,
        multiple_birth: None,
        photo: vec![],
        managing_organization: None,
        links: vec![],
        created_at: now,
        updated_at: now,
    }
}

/// Build a test [`Person`] that also carries one address.
fn create_test_person_with_address(
    family: &str,
    given: &str,
    birth_date: Option<Date>,
    city: &str,
    state: &str,
    postal_code: &str,
) -> Person {
    let mut person = create_test_person(family, given, birth_date);
    person.addresses.push(Address {
        use_type: None,
        line1: Some("123 Main Street".to_string()),
        line2: None,
        city: Some(city.to_string()),
        state: Some(state.to_string()),
        postal_code: Some(postal_code.to_string()),
        country: Some("US".to_string()),
    });
    person
}

/// A matching config with a 0.7 threshold for benchmarks.
fn create_matching_config() -> MatchingConfig {
    MatchingConfig {
        threshold_score: 0.7,
        exact_match_score: 1.0,
        fuzzy_match_score: 0.8,
    }
}

/// Benchmark name matching (fuzzy, exact, family-only, given variants).
fn bench_name_matching(c: &mut Criterion) {
    let name1 = HumanName {
        use_type: None,
        family: "Smith".to_string(),
        given: vec!["John".to_string()],
        prefix: vec![],
        suffix: vec![],
    };
    let name2 = HumanName {
        use_type: None,
        family: "Smyth".to_string(),
        given: vec!["Jon".to_string()],
        prefix: vec![],
        suffix: vec![],
    };

    c.bench_function("name_match_fuzzy", |b| {
        b.iter(|| name_matching::match_names(black_box(&name1), black_box(&name2)))
    });

    let name_exact = name1.clone();
    c.bench_function("name_match_exact", |b| {
        b.iter(|| name_matching::match_names(black_box(&name1), black_box(&name_exact)))
    });

    c.bench_function("family_name_match", |b| {
        b.iter(|| name_matching::match_family_names(black_box("Smith"), black_box("Smyth")))
    });

    c.bench_function("given_name_match_variants", |b| {
        let given1 = vec!["William".to_string()];
        let given2 = vec!["Bill".to_string()];
        b.iter(|| name_matching::match_given_names(black_box(&given1), black_box(&given2)))
    });
}

/// Benchmark birth-date matching (exact, off-by-one, missing).
fn bench_dob_matching(c: &mut Criterion) {
    let dob1 = Some(jiff::civil::date(1980, 1, 15));
    let dob2 = Some(jiff::civil::date(1980, 1, 16));

    c.bench_function("dob_match_exact", |b| {
        b.iter(|| dob_matching::match_birth_dates(black_box(dob1), black_box(dob1)))
    });

    c.bench_function("dob_match_typo", |b| {
        b.iter(|| dob_matching::match_birth_dates(black_box(dob1), black_box(dob2)))
    });

    c.bench_function("dob_match_missing", |b| {
        b.iter(|| dob_matching::match_birth_dates(black_box(dob1), black_box(None)))
    });
}

/// Benchmark gender matching (same vs different).
fn bench_gender_matching(c: &mut Criterion) {
    c.bench_function("gender_match_same", |b| {
        b.iter(|| gender_matching::match_gender(black_box(Gender::Male), black_box(Gender::Male)))
    });

    c.bench_function("gender_match_different", |b| {
        b.iter(|| gender_matching::match_gender(black_box(Gender::Male), black_box(Gender::Female)))
    });
}

/// Benchmark address matching on a near-identical address pair.
fn bench_address_matching(c: &mut Criterion) {
    let addr1 = Address {
        use_type: None,
        line1: Some("123 Main Street".to_string()),
        line2: None,
        city: Some("Springfield".to_string()),
        state: Some("IL".to_string()),
        postal_code: Some("62701".to_string()),
        country: Some("US".to_string()),
    };
    let addr2 = Address {
        use_type: None,
        line1: Some("123 Main St".to_string()),
        line2: None,
        city: Some("Springfield".to_string()),
        state: Some("IL".to_string()),
        postal_code: Some("62701".to_string()),
        country: Some("US".to_string()),
    };

    c.bench_function("address_match_similar", |b| {
        let addrs1 = vec![addr1.clone()];
        let addrs2 = vec![addr2.clone()];
        b.iter(|| address_matching::match_addresses(black_box(&addrs1), black_box(&addrs2)))
    });
}

/// Benchmark Soundex encoding and phonetic similarity.
fn bench_phonetic_matching(c: &mut Criterion) {
    c.bench_function("soundex_encode_short", |b| {
        b.iter(|| phonetic::soundex(black_box("Smith")))
    });

    c.bench_function("soundex_encode_long", |b| {
        b.iter(|| phonetic::soundex(black_box("Christopher")))
    });

    c.bench_function("soundex_match", |b| {
        b.iter(|| phonetic::soundex_match(black_box("Smith"), black_box("Smyth")))
    });

    c.bench_function("phonetic_similarity", |b| {
        b.iter(|| phonetic::phonetic_similarity(black_box("Robert"), black_box("Rupert")))
    });
}

/// Benchmark the full probabilistic matcher: a single pair and
/// `find_matches` against 10/100/1000 candidates.
fn bench_full_person_matcher(c: &mut Criterion) {
    let config = create_matching_config();
    let matcher = ProbabilisticMatcher::new(config);

    let dob = Some(jiff::civil::date(1980, 1, 15));
    let person =
        create_test_person_with_address("Smith", "John", dob, "Springfield", "IL", "62701");

    c.bench_function("match_persons_pair", |b| {
        let candidate = create_test_person("Smyth", "Jon", dob);
        b.iter(|| {
            matcher
                .match_persons(black_box(&person), black_box(&candidate))
                .unwrap()
        })
    });

    let candidates_10: Vec<Person> = (0..10)
        .map(|i| create_test_person(&format!("Person{}", i), &format!("Given{}", i), None))
        .collect();

    c.bench_function("find_matches_10_candidates", |b| {
        b.iter(|| {
            matcher
                .find_matches(black_box(&person), black_box(&candidates_10))
                .unwrap()
        })
    });

    let candidates_100: Vec<Person> = (0..100)
        .map(|i| create_test_person(&format!("Person{}", i), &format!("Given{}", i), None))
        .collect();

    c.bench_function("find_matches_100_candidates", |b| {
        b.iter(|| {
            matcher
                .find_matches(black_box(&person), black_box(&candidates_100))
                .unwrap()
        })
    });

    let candidates_1000: Vec<Person> = (0..1000)
        .map(|i| create_test_person(&format!("Person{}", i), &format!("Given{}", i), None))
        .collect();

    c.bench_function("find_matches_1000_candidates", |b| {
        b.iter(|| {
            matcher
                .find_matches(black_box(&person), black_box(&candidates_1000))
                .unwrap()
        })
    });
}

/// Benchmark the deterministic (rule-based) matcher on a pair.
fn bench_deterministic_matching(c: &mut Criterion) {
    let config = create_matching_config();
    let matcher = DeterministicMatcher::new(config);

    let dob = Some(jiff::civil::date(1980, 1, 15));
    let person = create_test_person("Smith", "John", dob);
    let candidate = create_test_person("Smith", "John", dob);

    c.bench_function("deterministic_match_pair", |b| {
        b.iter(|| {
            matcher
                .match_persons(black_box(&person), black_box(&candidate))
                .unwrap()
        })
    });
}

/// Benchmark tax-id matching (shared id vs missing).
fn bench_tax_id_matching(c: &mut Criterion) {
    let mut p1 = create_test_person("Smith", "John", None);
    p1.tax_id = Some("123-45-6789".to_string());

    let mut p2 = create_test_person("Smyth", "Jon", None);
    p2.tax_id = Some("123-45-6789".to_string());

    c.bench_function("tax_id_match_same", |b| {
        b.iter(|| tax_id_matching::match_tax_ids(black_box(&p1), black_box(&p2)))
    });

    let p3 = create_test_person("Jones", "Bob", None);
    c.bench_function("tax_id_match_missing", |b| {
        b.iter(|| tax_id_matching::match_tax_ids(black_box(&p1), black_box(&p3)))
    });
}

/// Benchmark identity-document matching on a matching passport pair.
fn bench_document_matching(c: &mut Criterion) {
    let doc1 = IdentityDocument {
        document_type: DocumentType::Passport,
        number: "X12345678".to_string(),
        issuing_country: Some("US".to_string()),
        issuing_authority: None,
        issue_date: None,
        expiry_date: None,
        verified: false,
    };
    let doc2 = IdentityDocument {
        document_type: DocumentType::Passport,
        number: "X12345678".to_string(),
        issuing_country: Some("US".to_string()),
        issuing_authority: None,
        issue_date: None,
        expiry_date: None,
        verified: false,
    };

    c.bench_function("document_match_same", |b| {
        let docs1 = vec![doc1.clone()];
        let docs2 = vec![doc2.clone()];
        b.iter(|| document_matching::match_documents(black_box(&docs1), black_box(&docs2)))
    });
}

criterion_group!(
    benches,
    bench_name_matching,
    bench_dob_matching,
    bench_gender_matching,
    bench_address_matching,
    bench_phonetic_matching,
    bench_full_person_matcher,
    bench_deterministic_matching,
    bench_tax_id_matching,
    bench_document_matching,
);
criterion_main!(benches);
