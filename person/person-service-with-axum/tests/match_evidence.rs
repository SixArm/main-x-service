#![warn(clippy::pedantic)]

//! Match evidence through the service (`agents/share/match-evidence.md`
//! §10 step 4): the batch scan flags an automatic merge only for a
//! `proven` pair, stores the evidence with the review item and returns it,
//! and `check-duplicates` / the create-time `409` carry evidence.
//!
//! `#[ignore]`d — run with
//! `DATABASE_URL=… cargo test --test match_evidence -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::NaiveDate;
use person_service::db::AuditContext;
use person_service::matching::evidence::{EvidenceStrength, ValueFrequencySource, pair_evidence};
use person_service::models::{Gender, HumanName, Identifier, IdentifierType, Person};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

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

/// A person with a run-unique name, so no other fixture scores near it.
fn named(family: &str, given: &str) -> Person {
    let mut p = Person::new(
        HumanName {
            use_type: None,
            family: family.to_string(),
            given: vec![given.to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Female,
    );
    p.birth_date = NaiveDate::from_ymd_opt(1974, 3, 21);
    p
}

/// A unique family name for this run: letters only, so it normalises to
/// itself.
fn run_family() -> String {
    let letters: String = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .filter_map(|c| {
            "abcdefghijklmnop"
                .chars()
                .nth(c.to_digit(16).unwrap() as usize)
        })
        .collect();
    format!("Evid{letters}")
}

/// A valid NHS number (mod-11 check digit) not used before.
fn fresh_nhs() -> String {
    loop {
        let n = Uuid::new_v4().as_u128();
        let digits: Vec<u32> = (0..9).map(|i| ((n >> (i * 4)) % 10) as u32).collect();
        let sum: u32 = digits.iter().zip((2..=10).rev()).map(|(d, w)| d * w).sum();
        let check = 11 - (sum % 11);
        let check = if check == 11 { 0 } else { check };
        if check == 10 || digits[0] == 0 {
            continue;
        }
        return digits.iter().map(ToString::to_string).collect::<String>() + &check.to_string();
    }
}

/// A well-formed US SSN not used before.
fn fresh_ssn() -> String {
    let n = Uuid::new_v4().as_u128();
    let area = 100 + (n % 566);
    let group = 10 + ((n >> 16) % 90);
    let serial = 1000 + ((n >> 32) % 9000);
    format!("{area:03}-{group:02}-{serial:04}")
}

fn nhs(value: &str) -> Identifier {
    Identifier::new(
        IdentifierType::Other,
        "https://fhir.nhs.uk/Id/nhs-number".into(),
        value.into(),
    )
}

/// Run a scan and return the reported item for `(a, b)`.
async fn scan_item(app: &axum::Router, a: Uuid, b: Uuid, threshold: f64, auto: f64) -> Value {
    let body = json!({
        "threshold": threshold,
        "max_candidates": 1000,
        "auto_merge_threshold": auto,
    });
    let (status, scan) = call(app, "POST", "/api/persons/deduplicate", Some(&body)).await;
    assert_eq!(status, StatusCode::OK, "{scan}");
    let (ia, ib) = (a.to_string(), b.to_string());
    scan["review_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| {
            let ids = [i["person_id_a"].as_str(), i["person_id_b"].as_str()];
            ids.contains(&Some(ia.as_str())) && ids.contains(&Some(ib.as_str()))
        })
        .cloned()
        .unwrap_or_else(|| panic!("the pair should be reported: {scan}"))
}

/// Two records agreeing only on name, birth date and gender clear the
/// auto-merge score, and still stay `pending`: that is what twins look
/// like. The evidence is stored and comes back from the list endpoint.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_name_and_birth_date_pair_above_the_auto_merge_score_stays_pending() {
    let state = common::create_test_app_state().await;
    let app = common::create_test_router().await;
    let family = run_family();
    let ctx = AuditContext::default();
    let a = state
        .person_repository
        .create(&named(&family, "Margaret"), &ctx)
        .await
        .unwrap();
    let b = state
        .person_repository
        .create(&named(&family, "Margaret"), &ctx)
        .await
        .unwrap();

    let score = state.matcher.match_persons(&a, &b).unwrap().score;
    assert!(
        score >= 0.95,
        "the fixture must clear the auto-merge score: {score}"
    );
    // The scan counts frequencies over what it loads; the fixture is only
    // meaningful if that leaves the pair short of proof.
    let loaded = state.person_repository.list_active(1000, 0).await.unwrap();
    let expected = pair_evidence(&a, &b, &ValueFrequencySource::from_persons(&loaded));
    assert_eq!(
        expected.strength,
        EvidenceStrength::Consistent,
        "fixture: {expected:?}"
    );

    let item = scan_item(&app, a.id, b.id, 0.9, 0.95).await;
    assert_eq!(item["status"], "pending", "{item}");
    assert_eq!(item["evidence"]["strength"], "consistent", "{item}");

    let (status, list) = call(
        &app,
        "GET",
        "/api/persons/review-queue?status=pending&limit=500",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{list}");
    let stored = list["items"]
        .as_array()
        .or_else(|| list.as_array())
        .expect("a list of items")
        .iter()
        .find(|i| i["id"] == item["id"])
        .cloned()
        .unwrap_or_else(|| panic!("the stored item should be listed: {list}"));
    assert_eq!(stored["evidence"]["strength"], "consistent", "{stored}");
}

/// A pair sharing a unique identifier (the tax id, routed to the US SSN
/// slot) clears the score and is proven, so it is flagged `automerged`.
/// The registry refuses two persons one `(system, value)` identifier, so
/// the tax id is how two stored records can share one.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_pair_sharing_a_unique_identifier_is_automerged_and_proven() {
    let state = common::create_test_app_state().await;
    let app = common::create_test_router().await;
    let family = run_family();
    let ssn = fresh_ssn();
    let ctx = AuditContext::default();
    let mut a = named(&family, "Margaret");
    a.tax_id = Some(ssn.clone());
    let mut b = named(&family, "Margaret");
    b.tax_id = Some(ssn);
    let a = state.person_repository.create(&a, &ctx).await.unwrap();
    let b = state.person_repository.create(&b, &ctx).await.unwrap();

    let item = scan_item(&app, a.id, b.id, 0.9, 0.95).await;
    assert_eq!(item["status"], "automerged", "{item}");
    assert_eq!(item["evidence"]["strength"], "proven", "{item}");
}

/// Two different NHS numbers contradict: the pair stays `pending` even at
/// a score above the request's auto-merge threshold.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_pair_with_conflicting_identifiers_stays_pending_and_contradicted() {
    let state = common::create_test_app_state().await;
    let app = common::create_test_router().await;
    let family = run_family();
    let ctx = AuditContext::default();
    let mut a = named(&family, "Margaret");
    a.identifiers.push(nhs(&fresh_nhs()));
    let mut b = named(&family, "Margaret");
    b.identifiers.push(nhs(&fresh_nhs()));
    let a = state.person_repository.create(&a, &ctx).await.unwrap();
    let b = state.person_repository.create(&b, &ctx).await.unwrap();

    let score = state.matcher.match_persons(&a, &b).unwrap().score;
    let auto = score - 0.01;
    let item = scan_item(&app, a.id, b.id, auto, auto).await;
    assert_eq!(item["status"], "pending", "{item}");
    assert_eq!(item["evidence"]["strength"], "contradicted", "{item}");
    assert!(
        !item["evidence"]["contradictions"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{item}"
    );
}

/// `check-duplicates` and the create-time `409` carry evidence for every
/// candidate, with no field value in it.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn check_duplicates_and_the_409_carry_evidence() {
    let app = common::create_test_router().await;
    let family = run_family();
    let body = json!({
        "name": { "family": family, "given": ["Margaret"] },
        "gender": "female",
        "birth_date": "1974-03-21",
    });
    let (status, created) = call(&app, "POST", "/api/persons", Some(&body)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");

    let (status, check) = call(&app, "POST", "/api/persons/check-duplicates", Some(&body)).await;
    assert_eq!(status, StatusCode::OK, "{check}");
    let matches = check["potential_matches"].as_array().unwrap();
    assert!(!matches.is_empty(), "{check}");
    for m in matches {
        assert!(m["evidence"]["strength"].is_string(), "{m}");
        let evidence = m["evidence"].to_string();
        assert!(!evidence.contains(&family.to_lowercase()), "{evidence}");
        assert!(!evidence.contains("1974"), "{evidence}");
    }

    let (status, conflict) = call(&app, "POST", "/api/persons", Some(&body)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{conflict}");
    let matches = conflict["errors"]["potential_matches"].as_array().unwrap();
    assert!(
        matches
            .iter()
            .all(|m| m["evidence"]["strength"].is_string()),
        "{conflict}"
    );
}

/// The database scan behind the cached table reads, pages and applies
/// the population floor: a table only from `MIN_FREQUENCY_POPULATION`
/// records up, and then it knows a freshly stored family name.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn the_frequency_scan_counts_the_stored_population() {
    use person_matcher::evidence::{ValueFrequencies, frequency_key};
    use person_service::matching::evidence::{MIN_FREQUENCY_POPULATION, scan_frequencies};

    let state = common::create_test_app_state().await;
    let family = run_family();
    state
        .person_repository
        .create(&named(&family, "Margaret"), &AuditContext::default())
        .await
        .unwrap();

    let source = scan_frequencies(&state.db).await.unwrap();
    let key = frequency_key("family_name", &family).unwrap();
    match &source {
        ValueFrequencySource::None => assert_eq!(source.share("family_name", &key), None),
        ValueFrequencySource::Table { population, .. } => {
            assert!(*population >= MIN_FREQUENCY_POPULATION);
            let share = source.share("family_name", &key).unwrap();
            assert!(
                (share - 1.0 / f64::from(u32::try_from(*population).unwrap())).abs() < 1e-12,
                "{share}"
            );
        }
    }
}
