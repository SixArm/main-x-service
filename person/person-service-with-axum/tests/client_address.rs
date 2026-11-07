//! The address on an audit row is the one `axa::client::ClientAddr` resolved,
//! on each path that writes one: a mutation, a read, a link, and an erasure.
//!
//! Its own test binary, because `PERSON_AUDIT_READS` is a process-wide
//! `OnceLock`. `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test client_address -- --ignored`.

mod common;

use axa::testing::request;
use axum::http::{HeaderName, HeaderValue};
use person_service::app::App;
use person_service::compliance::{disclosure, erasure};
use person_service::config::Config;
use person_service::db::models::audit_log;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

/// A person the duplicate check cannot match to one from an earlier run: a
/// matching row left by a failed run would answer `409` before any audit.
fn unique_person() -> Value {
    let bytes = *Uuid::new_v4().as_bytes();
    let letters = |range: std::ops::Range<usize>| -> String {
        bytes[range]
            .iter()
            .map(|b| char::from(b'a' + b % 26))
            .collect()
    };
    let (year, month, day) = (
        1920 + u32::from(bytes[0]) % 80,
        1 + u32::from(bytes[1]) % 12,
        1 + u32::from(bytes[2]) % 28,
    );
    json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": format!("Q{}", letters(0..10)),
            "given": [format!("Z{}", letters(10..16))]
        },
        "birth_date": format!("{year}-{month:02}-{day:02}"),
        "gender": "female"
    })
}

/// The addresses on the rows for `entity_id` with `action`.
async fn addresses(db: &DatabaseConnection, entity_id: Uuid, action: &str) -> Vec<Option<String>> {
    audit_log::Entity::find()
        .filter(audit_log::Column::EntityId.eq(entity_id))
        .filter(audit_log::Column::Action.eq(action))
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.ip_address)
        .collect()
}

/// Drive each audited path, sending `forwarded`, and check that every row
/// records `expected`. `TRUSTED_PROXIES` is read at boot, so it is set first.
async fn every_path_records(
    trusted: Option<&str>,
    forwarded: &'static str,
    expected: &'static str,
) {
    let config = Config::from_env().expect("test config");
    std::fs::create_dir_all(&config.search.index_path).unwrap();
    unsafe {
        std::env::set_var("PERSON_AUDIT_READS", "true");
        match trusted {
            Some(list) => std::env::set_var("TRUSTED_PROXIES", list),
            None => std::env::remove_var("TRUSTED_PROXIES"),
        }
    }
    request::<App, _, _>(|server, ctx| async move {
        let header = HeaderName::from_static("x-forwarded-for");
        let value = HeaderValue::from_static(forwarded);
        let expected = vec![Some(expected.to_string())];

        let created = server
            .post("/api/persons")
            .add_header(header.clone(), value.clone())
            .json(&unique_person())
            .await;
        created.assert_status(axum::http::StatusCode::CREATED);
        let body: Value = created.json();
        let id: Uuid = body["data"]["id"].as_str().unwrap().parse().unwrap();
        assert_eq!(addresses(&ctx.db, id, "CREATE").await, expected, "create");

        server
            .get(&format!("/api/persons/{id}"))
            .add_header(header.clone(), value.clone())
            .await
            .assert_status_ok();
        let reads = addresses(&ctx.db, id, disclosure::action::READ).await;
        assert_eq!(reads, expected, "read");

        let linked = server
            .post(&format!("/api/persons/{id}/links"))
            .add_header(header.clone(), value.clone())
            .json(&json!({
                "kind": "same_identity",
                "to_ref": format!("worker:{}", Uuid::new_v4())
            }))
            .await;
        linked.assert_status_success();
        let body: Value = linked.json();
        let link = body["data"]["id"].as_str().unwrap().parse().unwrap();
        assert_eq!(addresses(&ctx.db, link, "CREATE").await, expected, "link");

        server
            .post(&format!("/api/persons/{id}/erase"))
            .add_header(header, value)
            .await
            .assert_status_ok();
        let erased = addresses(&ctx.db, id, erasure::ACTION_ERASED).await;
        assert_eq!(erased, expected, "erasure");
    })
    .await;
    unsafe { std::env::remove_var("TRUSTED_PROXIES") };
}

/// A caller that is not a listed proxy cannot choose the address on the trail.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test client_address -- --ignored`"]
async fn a_forged_header_reaches_no_audit_row() {
    every_path_records(None, "6.6.6.6", "127.0.0.1").await;
}

/// Behind a listed proxy, the row names the entry that proxy appended.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test client_address -- --ignored`"]
async fn a_listed_proxy_is_believed_for_the_entry_it_wrote() {
    every_path_records(Some("127.0.0.0/8"), "6.6.6.6, 198.51.100.7", "198.51.100.7").await;
}
