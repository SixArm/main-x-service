//! Benchmarks for event search engine

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use chrono::{NaiveDate, Utc};
use tempfile::TempDir;
use uuid::Uuid;

use main_event_index::models::*;
use main_event_index::search::SearchEngine;

fn create_test_event(family: &str, given: &str, birth_date: Option<NaiveDate>) -> Event {
    let now = Utc::now();
    Event {
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

/// Family name pools for generating realistic test data
const FAMILY_NAMES: &[&str] = &[
    "Smith", "Johnson", "Williams", "Brown", "Jones",
    "Garcia", "Miller", "Davis", "Rodriguez", "Martinez",
    "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson",
    "Thomas", "Taylor", "Moore", "Jackson", "Martin",
    "Lee", "Perez", "Thompson", "White", "Harris",
    "Sanchez", "Clark", "Ramirez", "Lewis", "Robinson",
];

const GIVEN_NAMES: &[&str] = &[
    "James", "Robert", "John", "Michael", "David",
    "William", "Richard", "Joseph", "Thomas", "Charles",
    "Mary", "Patricia", "Jennifer", "Linda", "Barbara",
    "Elizabeth", "Susan", "Jessica", "Sarah", "Karen",
];

fn bench_index_single_event(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let engine = SearchEngine::new(temp_dir.path()).unwrap();
    let event = create_test_event("Smith", "John", NaiveDate::from_ymd_opt(1980, 1, 15));

    c.bench_function("index_single_event", |b| {
        b.iter(|| {
            engine.index_event(black_box(&event)).unwrap()
        })
    });
}

fn bench_index_bulk_events(c: &mut Criterion) {
    let events_50: Vec<Event> = (0..50)
        .map(|i| {
            let family = FAMILY_NAMES[i % FAMILY_NAMES.len()];
            let given = GIVEN_NAMES[i % GIVEN_NAMES.len()];
            create_test_event(family, given, None)
        })
        .collect();

    c.bench_function("bulk_index_50_events", |b| {
        b.iter_with_setup(
            || {
                let temp_dir = TempDir::new().unwrap();
                let engine = SearchEngine::new(temp_dir.path()).unwrap();
                (temp_dir, engine)
            },
            |(_temp_dir, engine)| {
                engine.index_events(black_box(&events_50)).unwrap()
            },
        )
    });
}

fn bench_search_queries(c: &mut Criterion) {
    // Set up index with 1000 events
    let temp_dir = TempDir::new().unwrap();
    let engine = SearchEngine::new(temp_dir.path()).unwrap();

    let events: Vec<Event> = (0..1000)
        .map(|i| {
            let family = FAMILY_NAMES[i % FAMILY_NAMES.len()];
            let given = GIVEN_NAMES[i % GIVEN_NAMES.len()];
            let dob = NaiveDate::from_ymd_opt(1950 + (i as i32 % 50), 1 + (i as u32 % 12), 1 + (i as u32 % 28));
            create_test_event(family, given, dob)
        })
        .collect();

    engine.index_events(&events).unwrap();
    engine.reload().unwrap();

    c.bench_function("search_1000_events_exact", |b| {
        b.iter(|| {
            engine.search(black_box("Smith"), 10).unwrap()
        })
    });

    c.bench_function("search_1000_events_limit_50", |b| {
        b.iter(|| {
            engine.search(black_box("Smith"), 50).unwrap()
        })
    });

    c.bench_function("fuzzy_search_1000_events", |b| {
        b.iter(|| {
            engine.fuzzy_search(black_box("Smyth"), 10).unwrap()
        })
    });

    c.bench_function("search_by_name_and_year_1000", |b| {
        b.iter(|| {
            engine.search_by_name_and_year(black_box("Smith"), black_box(Some(1980)), 10).unwrap()
        })
    });

    c.bench_function("search_no_results", |b| {
        b.iter(|| {
            engine.search(black_box("Zzzzxyzzy"), 10).unwrap()
        })
    });
}

fn bench_delete_event(c: &mut Criterion) {
    c.bench_function("delete_and_reindex_event", |b| {
        b.iter_with_setup(
            || {
                let temp_dir = TempDir::new().unwrap();
                let engine = SearchEngine::new(temp_dir.path()).unwrap();
                let event = create_test_event("Smith", "John", None);
                engine.index_event(&event).unwrap();
                engine.reload().unwrap();
                let id = event.id.to_string();
                (temp_dir, engine, id)
            },
            |(_temp_dir, engine, id)| {
                engine.delete_event(black_box(&id)).unwrap()
            },
        )
    });
}

criterion_group!(
    benches,
    bench_index_single_event,
    bench_index_bulk_events,
    bench_search_queries,
    bench_delete_event,
);
criterion_main!(benches);
