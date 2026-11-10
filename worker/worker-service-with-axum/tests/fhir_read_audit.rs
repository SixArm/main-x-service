//! FHIR reads are audited as the REST reads are (spec §13 T-27, HIPAA
//! §164.312(b)): with `WORKER_AUDIT_READS` on, `GET /fhir/Practitioner/{id}`
//! writes the same `READ` row for the worker that `GET /api/workers/{id}`
//! writes, and a FHIR search writes one `SEARCH` row against the nil id,
//! as `GET /api/workers/search` does.
//!
//! Its own test binary, because `WORKER_AUDIT_READS` is a process-wide
//! `OnceLock`. `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test fhir_read_audit -- --ignored`.

mod common;

use axa_test::request;
use serde_json::{Value, json};
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;
use worker_service::app::App;
use worker_service::compliance::disclosure;
use worker_service::config::Config;
use worker_service::db::AuditLogRepository;

/// `(entity_type, action, user_id)` of every row for `entity_id` with
/// `action`, in chain order.
async fn rows(db: &PgPool, entity_id: Uuid, action: &str) -> Vec<(String, String, Option<String>)> {
    AuditLogRepository::new(db.clone())
        .rows_for(None, entity_id, Some(action))
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.entity_type, row.action, row.user_id))
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn fhir_reads_write_the_rows_rest_reads_write() {
    let config = Config::from_env().expect("test config");
    std::fs::create_dir_all(&config.search.index_path).unwrap();
    // SAFETY: set before the app boots and before any thread reads it.
    unsafe {
        std::env::set_var("WORKER_AUDIT_READS", "true");
    }
    request::<App, _, _>(|server, ctx| async move {
        let family = format!("Fhiraudit{}", Uuid::new_v4().simple());
        let created = server
            .post("/api/workers")
            .json(&json!({
                "name": { "family": family, "given": ["Rowena"] },
                "gender": "female",
                "birth_date": "1971-03-09",
            }))
            .await;
        created.assert_status_success();
        let body: Value = created.json();
        let id: Uuid = body["id"].as_str().unwrap().parse().unwrap();

        server
            .get(&format!("/api/workers/{id}"))
            .await
            .assert_status_ok();
        let after_rest = rows(&ctx.db, id, disclosure::action::READ).await;
        assert_eq!(after_rest.len(), 1, "REST read writes one READ row");

        server
            .get(&format!("/fhir/Practitioner/{id}"))
            .await
            .assert_status_ok();
        let after_fhir = rows(&ctx.db, id, disclosure::action::READ).await;
        assert_eq!(after_fhir.len(), 2, "the FHIR read writes a READ row too");
        assert_eq!(
            after_fhir[1], after_fhir[0],
            "the FHIR row is the REST row's twin"
        );

        let searches = rows(&ctx.db, Uuid::nil(), disclosure::action::SEARCH)
            .await
            .len();
        server
            .get(&format!("/fhir/Practitioner?name={family}"))
            .await
            .assert_status_ok();
        assert_eq!(
            rows(&ctx.db, Uuid::nil(), disclosure::action::SEARCH)
                .await
                .len(),
            searches + 1,
            "a FHIR search writes one SEARCH row against the nil id"
        );
    })
    .await;
}
