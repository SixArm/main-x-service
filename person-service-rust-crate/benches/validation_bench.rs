//! Criterion benchmarks for validation and normalization.
//!
//! Covers [`validate_person`] on simple/complex/invalid records,
//! [`normalize_phone`] across input formats, and
//! [`standardize_address`] on full and minimal addresses. All functions
//! are pure, so these isolate CPU cost. Run with `cargo bench`.

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use jiff::{Timestamp, civil::Date};
use uuid::Uuid;

use person_service::models::*;
use person_service::validation::{validate_person, normalize_phone, standardize_address};

/// Build a minimal [`Person`] for validation benchmarks.
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

/// Benchmark validating a simple, valid person.
fn bench_validate_simple_person(c: &mut Criterion) {
    let person = create_test_person(
        "Smith",
        "John",
        Some(jiff::civil::date(1980, 1, 15)),
    );

    c.bench_function("validate_simple_person", |b| {
        b.iter(|| {
            validate_person(black_box(&person))
        })
    });
}

/// Benchmark validating a fully-populated person (telecom, address,
/// document, emergency contact).
fn bench_validate_complex_person(c: &mut Criterion) {
    let mut person = create_test_person(
        "Smith",
        "John",
        Some(jiff::civil::date(1980, 1, 15)),
    );

    person.tax_id = Some("123-45-6789".to_string());

    person.telecom.push(ContactPoint {
        system: ContactPointSystem::Phone,
        value: "(555) 123-4567".to_string(),
        use_type: Some(ContactPointUse::Home),
    });
    person.telecom.push(ContactPoint {
        system: ContactPointSystem::Email,
        value: "john.smith@example.com".to_string(),
        use_type: Some(ContactPointUse::Work),
    });

    person.addresses.push(Address {
        use_type: Some(AddressUse::Home),
        line1: Some("123 Main Street".to_string()),
        line2: Some("Apt 4B".to_string()),
        city: Some("Springfield".to_string()),
        state: Some("IL".to_string()),
        postal_code: Some("62701".to_string()),
        country: Some("US".to_string()),
    });

    person.documents.push(IdentityDocument {
        document_type: DocumentType::Passport,
        number: "X12345678".to_string(),
        issuing_country: Some("US".to_string()),
        issuing_authority: None,
        issue_date: Some(jiff::civil::date(2020, 1, 1)),
        expiry_date: Some(jiff::civil::date(2030, 1, 1)),
        verified: false,
    });

    person.emergency_contacts.push(EmergencyContact {
        name: "Jane Smith".to_string(),
        relationship: "spouse".to_string(),
        telecom: vec![ContactPoint {
            system: ContactPointSystem::Phone,
            value: "555-0199".to_string(),
            use_type: None,
        }],
        address: None,
        is_primary: true,
    });

    c.bench_function("validate_complex_person", |b| {
        b.iter(|| {
            validate_person(black_box(&person))
        })
    });
}

/// Benchmark validating an invalid person (the error-collecting path).
fn bench_validate_invalid_person(c: &mut Criterion) {
    let mut person = create_test_person("", "", None);
    person.telecom.push(ContactPoint {
        system: ContactPointSystem::Email,
        value: "not-an-email".to_string(),
        use_type: None,
    });
    person.emergency_contacts.push(EmergencyContact {
        name: "".to_string(),
        relationship: "".to_string(),
        telecom: vec![],
        address: None,
        is_primary: false,
    });

    c.bench_function("validate_invalid_person", |b| {
        b.iter(|| {
            validate_person(black_box(&person))
        })
    });
}

/// Benchmark phone normalization across input formats.
fn bench_normalize_phone(c: &mut Criterion) {
    c.bench_function("normalize_phone_us_format", |b| {
        b.iter(|| {
            normalize_phone(black_box("(555) 123-4567"), black_box("1"))
        })
    });

    c.bench_function("normalize_phone_international", |b| {
        b.iter(|| {
            normalize_phone(black_box("+1-555-123-4567"), black_box("1"))
        })
    });

    c.bench_function("normalize_phone_raw_digits", |b| {
        b.iter(|| {
            normalize_phone(black_box("5551234567"), black_box("1"))
        })
    });
}

/// Benchmark address standardization on full and minimal addresses.
fn bench_standardize_address(c: &mut Criterion) {
    let addr = Address {
        use_type: None,
        line1: Some("123 main st.".to_string()),
        line2: None,
        city: Some("new york".to_string()),
        state: Some("ny".to_string()),
        postal_code: Some("10001".to_string()),
        country: Some("us".to_string()),
    };

    c.bench_function("standardize_address_full", |b| {
        b.iter(|| {
            standardize_address(black_box(&addr))
        })
    });

    let addr_minimal = Address {
        use_type: None,
        line1: None,
        line2: None,
        city: Some("chicago".to_string()),
        state: None,
        postal_code: Some("60601".to_string()),
        country: None,
    };

    c.bench_function("standardize_address_minimal", |b| {
        b.iter(|| {
            standardize_address(black_box(&addr_minimal))
        })
    });
}

criterion_group!(
    benches,
    bench_validate_simple_person,
    bench_validate_complex_person,
    bench_validate_invalid_person,
    bench_normalize_phone,
    bench_standardize_address,
);
criterion_main!(benches);
