//! Benchmarks for patient validation and normalization

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use chrono::{NaiveDate, Utc};
use uuid::Uuid;

use main_patient_index::models::*;
use main_patient_index::validation::{validate_patient, normalize_phone, standardize_address};

fn create_test_patient(family: &str, given: &str, birth_date: Option<NaiveDate>) -> Patient {
    let now = Utc::now();
    Patient {
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

fn bench_validate_simple_patient(c: &mut Criterion) {
    let patient = create_test_patient(
        "Smith",
        "John",
        NaiveDate::from_ymd_opt(1980, 1, 15),
    );

    c.bench_function("validate_simple_patient", |b| {
        b.iter(|| {
            validate_patient(black_box(&patient))
        })
    });
}

fn bench_validate_complex_patient(c: &mut Criterion) {
    let mut patient = create_test_patient(
        "Smith",
        "John",
        NaiveDate::from_ymd_opt(1980, 1, 15),
    );

    patient.tax_id = Some("123-45-6789".to_string());

    patient.telecom.push(ContactPoint {
        system: ContactPointSystem::Phone,
        value: "(555) 123-4567".to_string(),
        use_type: Some(ContactPointUse::Home),
    });
    patient.telecom.push(ContactPoint {
        system: ContactPointSystem::Email,
        value: "john.smith@example.com".to_string(),
        use_type: Some(ContactPointUse::Work),
    });

    patient.addresses.push(Address {
        use_type: Some(AddressUse::Home),
        line1: Some("123 Main Street".to_string()),
        line2: Some("Apt 4B".to_string()),
        city: Some("Springfield".to_string()),
        state: Some("IL".to_string()),
        postal_code: Some("62701".to_string()),
        country: Some("US".to_string()),
    });

    patient.documents.push(IdentityDocument {
        document_type: DocumentType::Passport,
        number: "X12345678".to_string(),
        issuing_country: Some("US".to_string()),
        issuing_authority: None,
        issue_date: NaiveDate::from_ymd_opt(2020, 1, 1),
        expiry_date: NaiveDate::from_ymd_opt(2030, 1, 1),
        verified: false,
    });

    patient.emergency_contacts.push(EmergencyContact {
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

    c.bench_function("validate_complex_patient", |b| {
        b.iter(|| {
            validate_patient(black_box(&patient))
        })
    });
}

fn bench_validate_invalid_patient(c: &mut Criterion) {
    let mut patient = create_test_patient("", "", None);
    patient.telecom.push(ContactPoint {
        system: ContactPointSystem::Email,
        value: "not-an-email".to_string(),
        use_type: None,
    });
    patient.emergency_contacts.push(EmergencyContact {
        name: "".to_string(),
        relationship: "".to_string(),
        telecom: vec![],
        address: None,
        is_primary: false,
    });

    c.bench_function("validate_invalid_patient", |b| {
        b.iter(|| {
            validate_patient(black_box(&patient))
        })
    });
}

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
    bench_validate_simple_patient,
    bench_validate_complex_patient,
    bench_validate_invalid_patient,
    bench_normalize_phone,
    bench_standardize_address,
);
criterion_main!(benches);
