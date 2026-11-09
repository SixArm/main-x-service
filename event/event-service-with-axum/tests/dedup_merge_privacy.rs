#![warn(clippy::pedantic)]

//! Duplicate detection, merge and privacy through the real router and a
//! real database (spec §13 T-5).
//!
//! Each test names its events with a fresh UUID, so a run finds its own
//! records among whatever earlier runs left in a shared database.
//!
//! The matcher scores a component that neither event has as `0`, so two
//! events identical in name, start, end, venue and organiser reach only
//! `0.75`, under the default `0.85` threshold (spec §13 T-16). The create
//! test therefore shares a ticket number, which short-circuits to `1.0`,
//! and the batch test asks for a `0.7` threshold.
//!
//! `#[ignore]`d — run with
//! `DATABASE_URL=… cargo test --test dedup_merge_privacy -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{TimeZone, Utc};
use event_service::db::{EventRepository, PgEventRepository, create_connection};
use event_service::models::{Event, Identifier, IdentifierType, Location, Party, PartyKind};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

async fn repository() -> PgEventRepository {
    let config = event_service::config::Config::from_env().expect("config");
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection");
    PgEventRepository::new(db)
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(b) => {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(b).unwrap())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// An event named for this run, at a fixed time, venue and organiser,
/// with one identifier from `system`.
fn event_with_from(name: &str, system: &str, identifier: &str) -> Event {
    let start = Utc.with_ymd_and_hms(2031, 3, 14, 9, 0, 0).unwrap();
    let mut event = Event::new(name, start);
    event.end_date = Some(Utc.with_ymd_and_hms(2031, 3, 14, 17, 0, 0).unwrap());
    event.location.push(Location::Text {
        value: "Royal Infirmary, Lecture Theatre 3".into(),
    });
    event.organizers.push(Party {
        kind: PartyKind::Organization,
        id: None,
        name: "Northern Cardiology Society".into(),
        email: None,
        url: None,
    });
    event.identifiers.push(Identifier {
        use_type: None,
        identifier_type: IdentifierType::ExternalRef,
        system: system.into(),
        value: identifier.into(),
        assigner: None,
    });
    event
}

/// [`event_with_from`] with one fixed identifier system.
fn event_with(name: &str, identifier: &str) -> Event {
    event_with_from(name, "https://example.org/events", identifier)
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn creating_a_near_identical_event_is_a_409_naming_the_match() {
    let app = common::create_test_router().await;
    let name = format!("T5 Realtime {}", Uuid::new_v4());
    let body = json!({
        "name": name,
        "start_date": "2031-03-14T09:00:00Z",
        "end_date": "2031-03-14T17:00:00Z",
        "location": [{ "kind": "text", "value": "Royal Infirmary, Lecture Theatre 3" }],
        "organizers": [{ "kind": "organization", "name": "Northern Cardiology Society" }],
        "identifiers": [{
            "identifier_type": "TicketNumber",
            "system": "https://tickets.example.org",
            "value": format!("TKT-{}", Uuid::new_v4().simple()),
        }],
    });

    let (status, first) = call(&app, "POST", "/api/events", Some(&body)).await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let (status, second) = call(&app, "POST", "/api/events", Some(&body)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{second}");
    assert_eq!(second["code"], "duplicate_detected", "{second}");
    assert!(
        second.to_string().contains(first["id"].as_str().unwrap()),
        "the 409 names the event it matched: {second}"
    );
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn the_batch_scan_finds_a_pair_and_merge_folds_one_into_the_other() {
    let app = common::create_test_router().await;
    let repo = repository().await;
    // Seeded through the repository, past the create path's 409, as a
    // legacy import would leave them.
    let name = format!("T5 Batch {}", Uuid::new_v4());
    // Two systems' copies of one event: each holds its own reference, so
    // the identifiers do not conflict.
    let a = repo
        .create(&event_with_from(
            &name,
            "https://a.example.org",
            "REF-A-0001",
        ))
        .await
        .unwrap();
    let b = repo
        .create(&event_with_from(
            &name,
            "https://b.example.org",
            "REF-B-0002",
        ))
        .await
        .unwrap();

    let (status, scan) = call(
        &app,
        "POST",
        "/api/events/deduplicate",
        Some(&json!({ "threshold": 0.7, "max_candidates": 1000, "auto_merge_threshold": 0.999 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{scan}");
    let pair = scan["review_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| {
            let ids = [i["event_id_a"].as_str(), i["event_id_b"].as_str()];
            ids.contains(&Some(&a.id.to_string())) && ids.contains(&Some(&b.id.to_string()))
        })
        .unwrap_or_else(|| panic!("the scan should pair the two seeded events: {scan}"));
    assert!(pair["match_score"].as_f64().unwrap() >= 0.7, "{pair}");

    let (status, merged) = call(
        &app,
        "POST",
        "/api/events/merge",
        Some(&json!({ "main_event_id": a.id, "duplicate_event_id": b.id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{merged}");

    // The duplicate is gone; the survivor holds both identifiers.
    let (status, _) = call(&app, "GET", &format!("/api/events/{}", b.id), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, survivor) = call(&app, "GET", &format!("/api/events/{}", a.id), None).await;
    assert_eq!(status, StatusCode::OK);
    let values: Vec<&str> = survivor["identifiers"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["value"].as_str())
        .collect();
    assert!(
        values.contains(&"REF-A-0001") && values.contains(&"REF-B-0002"),
        "{survivor}"
    );

    // A second merge of the same duplicate finds nothing to merge.
    let (status, again) = call(
        &app,
        "POST",
        "/api/events/merge",
        Some(&json!({ "main_event_id": a.id, "duplicate_event_id": b.id })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{again}");
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn the_masked_view_hides_what_the_export_returns_whole() {
    let app = common::create_test_router().await;
    let repo = repository().await;
    let mut event = event_with(&format!("T5 Privacy {}", Uuid::new_v4()), "TICKET-12345678");
    event.organizers.clear();
    event.organizers.push(Party {
        kind: PartyKind::Person,
        id: Some(Uuid::new_v4()),
        name: "Ada Organiser".into(),
        email: Some("ada@example.org".into()),
        url: None,
    });
    let stored = repo.create(&event).await.unwrap();

    let (status, masked) = call(
        &app,
        "GET",
        &format!("/api/events/{}/masked", stored.id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{masked}");
    let id_value = masked["identifiers"][0]["value"].as_str().unwrap();
    assert_ne!(id_value, "TICKET-12345678");
    assert!(
        id_value.ends_with("5678") && id_value.contains('*'),
        "{id_value}"
    );
    let organiser = &masked["organizers"][0];
    assert_ne!(organiser["email"], "ada@example.org", "{organiser}");
    assert!(
        organiser["id"].is_null(),
        "external party id withheld: {organiser}"
    );
    assert_eq!(organiser["name"], "Ada Organiser");

    let (status, export) = call(
        &app,
        "GET",
        &format!("/api/events/{}/export", stored.id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{export}");
    assert_eq!(export["identifiers"][0]["value"], "TICKET-12345678");
    assert_eq!(export["organizers"][0]["email"], "ada@example.org");
    assert_eq!(
        export["organizers"][0]["id"],
        event.organizers[0].id.unwrap().to_string()
    );
}
