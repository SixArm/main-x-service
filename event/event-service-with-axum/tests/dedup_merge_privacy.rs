#![warn(clippy::pedantic)]

//! Duplicate detection, merge and privacy through the real router and a
//! real database (spec §13 T-5).
//!
//! Each test names its events with a fresh UUID, so a run finds its own
//! records among whatever earlier runs left in a shared database.
//!
//! Scores come from the `event-matcher` crate, which renormalises over
//! the components both events hold (spec §13 T-16). Until T-16 the
//! service's own scorer counted a component neither event had as `0`, so
//! two events identical in name, start, end, venue and organiser reached
//! only `0.75`, under the default `0.85` threshold: the create test had to
//! share a ticket number to be caught, and the batch test had to ask for a
//! `0.7` threshold. Now the identical events alone are a `409`, and the
//! batch scan finds them at its default threshold.
//!
//! What a high score no longer buys is an automatic merge. Every match
//! carries the matcher's evidence (`agents/share/match-evidence.md`), and a
//! batch item is `automerged` only when its evidence is `proven`: a pair
//! agreeing on name and start alone is `consistent` and stays `pending`,
//! and two ticket numbers from one system are `contradicted`.
//!
//! The batch scan's events are seeded through the repository, past the
//! create path's `409`, as a legacy import would leave them.
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

/// The JSON body of an event with no identifier, named for this run.
fn realtime_body(name: &str) -> Value {
    json!({
        "name": name,
        "start_date": "2031-03-14T09:00:00Z",
        "end_date": "2031-03-14T17:00:00Z",
        "location": [{ "kind": "text", "value": "Royal Infirmary, Lecture Theatre 3" }],
        "organizers": [{ "kind": "organization", "name": "Northern Cardiology Society" }],
    })
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn creating_a_near_identical_event_is_a_409_naming_the_match() {
    let app = common::create_test_router().await;
    // No identifier: before T-16 this pair scored 0.75 and was let through.
    let body = realtime_body(&format!("T5 Realtime {}", Uuid::new_v4()));

    let (status, first) = call(&app, "POST", "/api/events", Some(&body)).await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let (status, second) = call(&app, "POST", "/api/events", Some(&body)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{second}");
    assert_eq!(second["code"], "duplicate_detected", "{second}");
    assert!(
        second.to_string().contains(first["id"].as_str().unwrap()),
        "the 409 names the event it matched: {second}"
    );
    let candidate = &second["errors"]["potential_matches"][0];
    assert!(candidate["score"].as_f64().unwrap() > 0.99, "{candidate}");
    assert_eq!(candidate["is_match"], true, "{candidate}");
    let evidence = &candidate["evidence"];
    assert!(
        evidence["weight_bits"].as_f64().unwrap() > 0.0,
        "{evidence}"
    );
    assert!(
        evidence["fields_compared"].as_u64().unwrap() >= 5,
        "{evidence}"
    );
    assert!(
        evidence["components"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["component"] == "name"),
        "{evidence}"
    );
    // A component neither event holds is not compared.
    assert!(
        candidate["score_breakdown"]["performers_score"].is_null(),
        "{candidate}"
    );

    // `check-duplicates` answers the same, without creating.
    let (status, check) = call(&app, "POST", "/api/events/check-duplicates", Some(&body)).await;
    assert_eq!(status, StatusCode::OK, "{check}");
    assert_eq!(check["has_duplicates"], true, "{check}");
    assert_eq!(
        check["potential_matches"][0]["evidence"], *evidence,
        "{check}"
    );
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_shared_ticket_number_is_a_409_with_proven_evidence() {
    let app = common::create_test_router().await;
    let mut body = realtime_body(&format!("T5 Ticket {}", Uuid::new_v4()));
    body["identifiers"] = json!([{
        "identifier_type": "TicketNumber",
        "system": "https://tickets.example.org",
        "value": format!("TKT-{}", Uuid::new_v4().simple()),
    }]);

    let (status, first) = call(&app, "POST", "/api/events", Some(&body)).await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let (status, second) = call(&app, "POST", "/api/events", Some(&body)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{second}");
    let evidence = &second["errors"]["potential_matches"][0]["evidence"];
    assert_eq!(evidence["strength"], "proven", "{evidence}");
    // Evidence names components, never the ticket number itself.
    let ticket = body["identifiers"][0]["value"].as_str().unwrap();
    assert!(!evidence.to_string().contains(ticket), "{evidence}");
}

/// A second record with `event`'s content under a fresh id.
fn copy_of(event: &Event) -> Event {
    let mut copy = event.clone();
    copy.id = Uuid::new_v4();
    copy
}

/// Run a batch scan and return the item pairing `a` and `b`.
async fn scan_pair(app: &axum::Router, request: &Value, a: Uuid, b: Uuid) -> Value {
    let (status, scan) = call(app, "POST", "/api/events/deduplicate", Some(request)).await;
    assert_eq!(status, StatusCode::OK, "{scan}");
    scan["review_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| {
            let ids = [i["event_id_a"].as_str(), i["event_id_b"].as_str()];
            ids.contains(&Some(&a.to_string())) && ids.contains(&Some(&b.to_string()))
        })
        .cloned()
        .unwrap_or_else(|| panic!("the scan should pair the two seeded events: {scan}"))
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn the_batch_scan_finds_a_pair_and_merge_folds_one_into_the_other() {
    let app = common::create_test_router().await;
    let repo = repository().await;
    let name = format!("T5 Batch {}", Uuid::new_v4());
    // Two systems' copies of one event: each holds its own reference, so
    // the identifiers are two namespaces and do not conflict.
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

    // The scan's own default threshold (0.7) finds them now (T-16). The
    // score is not 1.0: both hold identifiers and share none, which the
    // matcher's `event_ids_score` counts as 0 whatever their schemes.
    let pair = scan_pair(&app, &json!({ "max_candidates": 1000 }), a.id, b.id).await;
    assert!(pair["match_score"].as_f64().unwrap() >= 0.7, "{pair}");
    assert!(
        pair["evidence"]["contradictions"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{pair}"
    );

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
async fn agreeing_on_name_and_start_alone_stays_pending_as_consistent() {
    let app = common::create_test_router().await;
    let repo = repository().await;
    let start = Utc.with_ymd_and_hms(2031, 3, 14, 9, 0, 0).unwrap();
    let name = format!("T5 Sparse {}", Uuid::new_v4());
    let a = repo.create(&Event::new(name.clone(), start)).await.unwrap();
    let b = repo.create(&Event::new(name, start)).await.unwrap();

    let request = json!({ "max_candidates": 1000, "auto_merge_threshold": 0.9 });
    let pair = scan_pair(&app, &request, a.id, b.id).await;
    // Above the auto-merge threshold on score, short of proof on evidence.
    assert!(pair["match_score"].as_f64().unwrap() > 0.9, "{pair}");
    assert_eq!(pair["evidence"]["strength"], "consistent", "{pair}");
    assert_eq!(pair["status"], "pending", "{pair}");
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_shared_ticket_is_automerged_as_proven() {
    let app = common::create_test_router().await;
    let repo = repository().await;
    let name = format!("T5 Proven {}", Uuid::new_v4());
    let ticket = format!("TKT-{}", Uuid::new_v4().simple());
    let mut first = event_with_from(&name, "https://tickets.example.org", &ticket);
    first.identifiers[0].identifier_type = IdentifierType::TicketNumber;
    let a = repo.create(&first).await.unwrap();
    let b = repo.create(&copy_of(&first)).await.unwrap();

    let request = json!({ "max_candidates": 1000, "auto_merge_threshold": 0.9 });
    let pair = scan_pair(&app, &request, a.id, b.id).await;
    assert_eq!(pair["evidence"]["strength"], "proven", "{pair}");
    assert_eq!(pair["status"], "automerged", "{pair}");
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn two_tickets_from_one_system_stay_pending_as_contradicted() {
    let app = common::create_test_router().await;
    let repo = repository().await;
    let name = format!("T5 Contradicted {}", Uuid::new_v4());
    let mut first = event_with_from(&name, "https://tickets.example.org", "TKT-ONE");
    first.identifiers[0].identifier_type = IdentifierType::TicketNumber;
    let mut second = copy_of(&first);
    second.identifiers[0].value = "TKT-TWO".into();
    let a = repo.create(&first).await.unwrap();
    let b = repo.create(&second).await.unwrap();

    let request = json!({ "threshold": 0.5, "max_candidates": 1000, "auto_merge_threshold": 0.5 });
    let pair = scan_pair(&app, &request, a.id, b.id).await;
    assert_eq!(pair["evidence"]["strength"], "contradicted", "{pair}");
    assert_eq!(
        pair["evidence"]["contradictions"][0]["field"],
        "identifier:other:ticketnumber@https://tickets.example.org",
        "{pair}"
    );
    assert_eq!(pair["status"], "pending", "{pair}");
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
