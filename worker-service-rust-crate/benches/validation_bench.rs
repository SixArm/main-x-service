//! Criterion benchmarks for the validation / normalization layer.
//!
//! Times worker validation across simple, fully-populated, and invalid
//! records, plus phone normalization (several input formats) and address
//! standardization (full and minimal). Run with `cargo bench`.

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use worker_service::models::*;
use worker_service::validation::{validate_worker, normalize_phone, standardize_address};

/// Builds a minimal [`Worker`] fixture; the more complex benches extend it.
fn create_test_worker(family: &str, given: &str, birth_date: Option<NaiveDate>) -> Worker {
    let now = Utc::now();
    Worker {
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
        worker_type: None,
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

/// Benchmarks validating a minimal valid worker (name + DOB).
fn bench_validate_simple_worker(c: &mut Criterion) {
    let worker = create_test_worker(
        "Smith",
        "John",
        NaiveDate::from_ymd_opt(1980, 1, 15),
    );

    c.bench_function("validate_simple_worker", |b| {
        b.iter(|| {
            validate_worker(black_box(&worker))
        })
    });
}

/// Benchmarks validating a fully-populated worker (telecom, address,
/// document, emergency contact, tax id) — the worst case for validation cost.
fn bench_validate_complex_worker(c: &mut Criterion) {
    let mut worker = create_test_worker(
        "Smith",
        "John",
        NaiveDate::from_ymd_opt(1980, 1, 15),
    );

    worker.tax_id = Some("123-45-6789".to_string());

    worker.telecom.push(ContactPoint {
        system: ContactPointSystem::Phone,
        value: "(555) 123-4567".to_string(),
        use_type: Some(ContactPointUse::Home),
    });
    worker.telecom.push(ContactPoint {
        system: ContactPointSystem::Email,
        value: "john.smith@example.com".to_string(),
        use_type: Some(ContactPointUse::Work),
    });

    worker.addresses.push(Address {
        use_type: Some(AddressUse::Home),
        line1: Some("123 Main Street".to_string()),
        line2: Some("Apt 4B".to_string()),
        city: Some("Springfield".to_string()),
        state: Some("IL".to_string()),
        postal_code: Some("62701".to_string()),
        country: Some("US".to_string()),
    });

    worker.documents.push(IdentityDocument {
        document_type: DocumentType::Passport,
        number: "X12345678".to_string(),
        issuing_country: Some("US".to_string()),
        issuing_authority: None,
        issue_date: NaiveDate::from_ymd_opt(2020, 1, 1),
        expiry_date: NaiveDate::from_ymd_opt(2030, 1, 1),
        verified: false,
    });

    worker.emergency_contacts.push(EmergencyContact {
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

    c.bench_function("validate_complex_worker", |b| {
        b.iter(|| {
            validate_worker(black_box(&worker))
        })
    });
}

/// Benchmarks validating a worker that triggers several errors (blank name,
/// bad email, empty emergency contact) — the error-collection path.
fn bench_validate_invalid_worker(c: &mut Criterion) {
    let mut worker = create_test_worker("", "", None);
    worker.telecom.push(ContactPoint {
        system: ContactPointSystem::Email,
        value: "not-an-email".to_string(),
        use_type: None,
    });
    worker.emergency_contacts.push(EmergencyContact {
        name: "".to_string(),
        relationship: "".to_string(),
        telecom: vec![],
        address: None,
        is_primary: false,
    });

    c.bench_function("validate_invalid_worker", |b| {
        b.iter(|| {
            validate_worker(black_box(&worker))
        })
    });
}

/// Benchmarks phone normalization across US-formatted, international, and
/// raw-digit inputs.
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

/// Benchmarks address standardization for a full address and a minimal one.
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
    bench_validate_simple_worker,
    bench_validate_complex_worker,
    bench_validate_invalid_worker,
    bench_normalize_phone,
    bench_standardize_address,
);
criterion_main!(benches);
