#![warn(clippy::pedantic)]

//! DB-gated proof that `GET /api/audit/verify` examines the **newest**
//! audit rows (repo `tasks.md` LG-A1), not an arbitrary sample.
//!
//! `audit_log.id` is a random UUID, so ordering by it picked whichever
//! rows happened to sort last. This test makes the newest row sort last
//! by id — any UUID is a possible random draw — tampers with it in SQL,
//! and checks that a small `limit` still reports it.
//!
//! `#[ignore]`d so the default `cargo test` stays green on a
//! database-less machine:
//!
//! ```sh
//! DATABASE_URL=postgres://loco:loco@localhost:5432/thing_service_test \
//!   cargo test --test audit_verify -- --ignored
//! ```

use axa::testing::TestApp;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

/// More rows than the `limit` the test asks for, so a wrong order can
/// leave the newest row out.
const ROWS: u128 = 12;
const LIMIT: usize = 3;

/// `GET /api/audit/verify?limit=LIMIT`, decoded.
async fn verify(app: &axum::Router) -> Value {
    let req = Request::builder()
        .uri(format!("/api/audit/verify?limit={LIMIT}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).expect("a JSON report")
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test audit_verify -- --ignored`"]
async fn verify_examines_the_newest_rows_and_catches_a_tampered_newest_row() {
    let config = thing_service::config::Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    let app = thing_service::app::App::boot_test()
        .await
        .expect("boot the app: set DATABASE_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router");
    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL"))
        .await
        .expect("connect");
    let audit_log = thing_service::db::audit::AuditLogRepository::new(db.clone());
    for i in 0..ROWS {
        audit_log
            .log_create(
                "thing",
                Uuid::from_u128(1000 + i),
                json!({ "n": i.to_string() }),
                &thing_service::db::audit::AuditContext::default(),
            )
            .await
            .expect("record audit row");
    }

    // The newest row gets the smallest possible id, so `ORDER BY id DESC`
    // sorts it last. `id` is outside the digest pre-image, so this alone
    // leaves the row intact.
    let newest = Uuid::nil();
    let moved = sqlx::query(
        "UPDATE audit_log SET id = $1
         WHERE id = (SELECT id FROM audit_log
                     ORDER BY created_at DESC, id DESC LIMIT 1)",
    )
    .bind(newest)
    .execute(&db)
    .await
    .expect("move newest id");
    assert_eq!(moved.rows_affected(), 1);

    let before = verify(&app).await;
    assert_eq!(before["verified"], true, "untampered trail: {before}");
    assert_eq!(before["rows"], LIMIT);

    // Tamper with the newest row's content.
    sqlx::query("UPDATE audit_log SET action = 'TAMPERED' WHERE id = $1")
        .bind(newest)
        .execute(&db)
        .await
        .expect("tamper");
    let after = verify(&app).await;

    // Put it back before asserting, so no later suite inherits a
    // tampered trail.
    sqlx::query("UPDATE audit_log SET action = 'CREATE' WHERE id = $1")
        .bind(newest)
        .execute(&db)
        .await
        .expect("restore");

    assert_eq!(after["rows"], LIMIT);
    assert_eq!(
        after["verified"], false,
        "a tampered newest row must be examined and caught: {after}"
    );
    assert_eq!(after["mismatched"], json!([newest.to_string()]));
}
