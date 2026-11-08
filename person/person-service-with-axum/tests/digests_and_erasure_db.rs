//! DB-gated tests for the stored digests of a soft-deleted or erased
//! record (PE-D1, PE-E1) and for the order of an erasure's audit
//! redaction against the chain lock (PE-E2).
//!
//! A test binary of its own because the record MAC is only written when
//! a key is configured, and the key is read once per process: setting it
//! here, before anything computes a MAC, leaves every other suite's
//! process without one, as it was.

use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use person_service::compliance::disclosure::AccessContext;
use person_service::compliance::{erasure, record_integrity};
use person_service::db::audit::CHAIN_LOCK_KEY;
use person_service::db::repositories::stored_record;
use person_service::db::{AuditContext, AuditLogRepository, PersonRepository, PgPersonRepository};
use person_service::models::{Gender, HumanName, Person};
use serde_json::json;
use sqlx::{Connection as _, PgPool};
use uuid::Uuid;

/// Configure a MAC key for this process, once, before any MAC is taken.
fn with_mac_key() {
    static KEY: Once = Once::new();
    KEY.call_once(|| {
        // Fixed rather than generated, so a rerun against the same database
        // verifies the MACs an earlier run wrote. A test key, never a
        // deployment's.
        let key: String = (0u8..32).map(|b| format!("{b:02x}")).collect();
        // SAFETY: `set_var` is `unsafe` in edition 2024 because another
        // thread may read the environment concurrently. This runs once,
        // before the first MAC is computed (the key set is read on first
        // use), and the DB suites run with one test thread.
        unsafe {
            std::env::set_var(person_service::compliance::mac::KEY_ENV, key);
        }
    });
    assert!(
        person_service::compliance::mac::is_enabled(),
        "the MAC key must load, or the MAC checks below check nothing"
    );
}

/// A pool on `DATABASE_URL`, migrated.
async fn connect() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
    let db = PgPool::connect(&url)
        .await
        .expect("connect to DATABASE_URL");
    person_service::db::MIGRATOR
        .run(&db)
        .await
        .expect("run migrations");
    db
}

fn a_person(family: &str) -> Person {
    Person::new(
        HumanName {
            use_type: None,
            family: format!("{family}{}", Uuid::new_v4().simple()),
            given: vec!["Test".to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Female,
    )
}

/// Hard-delete a test record, so its MAC (made with this process's key)
/// is not left for another suite to find.
async fn purge(db: &PgPool, id: Uuid) {
    sqlx::query("DELETE FROM persons WHERE id = $1")
        .bind(id)
        .execute(db)
        .await
        .expect("purge test record");
}

/// PE-D1: a soft delete restamps all three stored values over the row as
/// it now stands, `deleted_at` included. Read directly, because
/// `/api/records/verify` skips deleted rows.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_soft_delete_restamps_every_digest() {
    with_mac_key();
    let db = connect().await;
    let repo = PgPersonRepository::new(db.clone());
    let ctx = AuditContext::default();
    let created = repo.create(&a_person("SoftDeleted"), &ctx).await.unwrap();

    let (_, live) = stored_record(&db, created.id).await.unwrap().unwrap();
    repo.delete(&created.id, &ctx).await.unwrap();
    let (stored, after) = stored_record(&db, created.id).await.unwrap().unwrap();

    let deleted_at = after.deleted_at.expect("the row is soft-deleted");
    let expected =
        record_integrity::digests_with_deleted_at(&stored, Some(deleted_at.timestamp_micros()))
            .unwrap();
    assert_eq!(
        after.content_hash.as_deref(),
        Some(expected.sha256.as_str())
    );
    assert_eq!(
        after.content_hash_sha3.as_deref(),
        Some(expected.sha3.as_str()),
        "the SHA-3 digest must cover the soft-delete stamp"
    );
    assert!(expected.mac.is_some(), "a key is configured");
    assert_eq!(
        after.content_mac, expected.mac,
        "the MAC must cover the soft-delete stamp"
    );
    // And each of them changed, so the checks above are not comparing a
    // live record's digests with themselves.
    assert_ne!(after.content_hash_sha3, live.content_hash_sha3);
    assert_ne!(after.content_mac, live.content_mac);

    purge(&db, created.id).await;
}

/// PE-E1: erasure leaves no keyed digest of the erased content. It nulls
/// the MAC as it nulls the two hashes.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn erasure_nulls_the_mac_with_the_hashes() {
    with_mac_key();
    let db = connect().await;
    let repo = PgPersonRepository::new(db.clone());
    let created = repo
        .create(&a_person("ErasedMac"), &AuditContext::default())
        .await
        .unwrap();
    let (_, before) = stored_record(&db, created.id).await.unwrap().unwrap();
    assert!(before.content_mac.is_some(), "a fresh record carries a MAC");

    let audit = AuditLogRepository::new(db.clone());
    let access = AccessContext::from_parts(None, None, None);
    let mut txn = db.begin().await.unwrap();
    erasure::erase(&mut txn, created.id, Some("tester"), &access, &audit)
        .await
        .unwrap();
    txn.commit().await.unwrap();

    // Read the columns directly, so this check does not depend on the
    // erased record assembling (that is PE-E3's test, below).
    let (hash, sha3, mac): (Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT content_hash, content_hash_sha3, content_mac FROM persons WHERE id = $1",
    )
    .bind(created.id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(hash, None);
    assert_eq!(sha3, None);
    assert_eq!(
        mac, None,
        "a MAC over erased content must not survive the erasure"
    );

    purge(&db, created.id).await;
}

/// Whether backend `pid` is waiting for an advisory lock.
async fn waits_on_advisory_lock(db: &PgPool, pid: i32) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_locks
                        WHERE pid = $1 AND locktype = 'advisory' AND NOT granted)",
    )
    .bind(pid)
    .fetch_one(db)
    .await
    .expect("read pg_locks")
}

/// PE-E2: an audit row about the subject that is appended while the
/// erasure runs is redacted with the rest.
///
/// A second session holds the chain lock, so the erasure blocks on it.
/// Once `pg_locks` shows it waiting, that session appends a chained row
/// about the subject, commits it, and releases the lock. Taking the lock
/// before the redaction sweep means the sweep runs after that row
/// committed, and redacts it. Taking it after (in the `erased` append)
/// means the sweep has already run, and the row survives unredacted.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn erasure_redacts_an_audit_row_appended_while_it_waits() {
    with_mac_key();
    let db = connect().await;
    let audit = Arc::new(AuditLogRepository::new(db.clone()));
    let repo = PgPersonRepository::new(db.clone()).with_audit_log(Arc::clone(&audit));
    let ctx = AuditContext::default();
    let created = repo.create(&a_person("ErasedLate"), &ctx).await.unwrap();
    let id = created.id;

    // The holder takes the chain lock at session level.
    let mut holder = db.acquire().await.unwrap();
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(CHAIN_LOCK_KEY)
        .execute(&mut *holder)
        .await
        .unwrap();

    // The erasure, on a connection of its own, in its own transaction.
    let mut eraser = db.acquire().await.unwrap();
    let eraser_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *eraser)
        .await
        .unwrap();
    let erase_audit = Arc::clone(&audit);
    let erasing = tokio::spawn(async move {
        let access = AccessContext::from_parts(None, None, None);
        let mut txn = eraser.begin().await.unwrap();
        let outcome = erasure::erase(&mut txn, id, Some("tester"), &access, &erase_audit)
            .await
            .unwrap();
        txn.commit().await.unwrap();
        outcome
    });

    let deadline = Instant::now() + Duration::from_secs(20);
    while !waits_on_advisory_lock(&db, eraser_pid).await {
        assert!(
            Instant::now() < deadline,
            "the erasure never waited on the chain lock"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Append a chained row about the subject while the erasure waits.
    // The holder's session lock makes the append's own lock re-entrant.
    audit
        .log_update_on(
            &mut *holder,
            "Person",
            id,
            json!({ "family": "Late" }),
            json!({ "family": "Later" }),
            &ctx,
        )
        .await
        .unwrap();
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(CHAIN_LOCK_KEY)
        .execute(&mut *holder)
        .await
        .unwrap();
    drop(holder);

    let outcome = tokio::time::timeout(Duration::from_secs(20), erasing)
        .await
        .expect("the erasure finishes once the lock is free")
        .unwrap();
    assert!(outcome.audit_rows_redacted >= 2, "{outcome:?}");

    let unredacted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log
         WHERE entity_id = $1 AND action <> 'erased' AND redacted_at IS NULL",
    )
    .bind(id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(
        unredacted, 0,
        "an audit row about the subject survived the erasure unredacted"
    );

    let report = audit.verify_chain(1000).await.unwrap();
    assert!(report.verified, "the chain must still verify: {report:?}");

    purge(&db, id).await;
}

/// PE-E3: an erased record still assembles. Its tombstone name is the
/// primary name, so a read of the deleted row through the repository
/// returns the tombstone rather than failing with "Person has no primary
/// name". The erased content stays gone and the digests stay NULL (PE-E1).
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn an_erased_record_reassembles_as_its_tombstone() {
    with_mac_key();
    let db = connect().await;
    let repo = PgPersonRepository::new(db.clone());
    let mut person = a_person("ErasedReassembled");
    person.tax_id = Some("123-45-6789".to_string());
    person.additional_names.push(HumanName {
        use_type: None,
        family: "AliasToErase".to_string(),
        given: vec!["Alias".to_string()],
        prefix: vec![],
        suffix: vec![],
    });
    let created = repo
        .create(&person, &AuditContext::default())
        .await
        .unwrap();

    let audit = AuditLogRepository::new(db.clone());
    let access = AccessContext::from_parts(None, None, None);
    let mut txn = db.begin().await.unwrap();
    erasure::erase(&mut txn, created.id, Some("tester"), &access, &audit)
        .await
        .unwrap();
    txn.commit().await.unwrap();

    let (stored, digests) = stored_record(&db, created.id)
        .await
        .expect("an erased record must assemble through the repository")
        .expect("the erased row is retained");
    assert_eq!(stored.name.family, erasure::TOMBSTONE_NAME);
    assert!(stored.name.given.is_empty(), "no given name survives");
    assert!(
        stored.additional_names.is_empty(),
        "no alias survives erasure: {:?}",
        stored.additional_names
    );
    assert_eq!(stored.tax_id, None, "the tax id is scrubbed");
    assert_eq!(stored.birth_date, None, "the birth date is scrubbed");
    assert_eq!(stored.gender, Gender::Unknown);
    assert!(digests.deleted_at.is_some(), "the erased record is retired");
    assert_eq!(digests.content_hash, None);
    assert_eq!(digests.content_hash_sha3, None);
    assert_eq!(
        digests.content_mac, None,
        "PE-E1: no MAC over erased content"
    );

    purge(&db, created.id).await;
}

/// PE-E3: the backfill migration flags a tombstone an earlier erasure
/// wrote non-primary, so that record assembles too. The earlier erasure
/// is simulated by un-flagging a fresh tombstone; the migration's SQL is
/// then run again (it is idempotent and narrow).
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn the_backfill_flags_an_old_non_primary_tombstone() {
    with_mac_key();
    let db = connect().await;
    let repo = PgPersonRepository::new(db.clone());
    let created = repo
        .create(&a_person("ErasedBackfill"), &AuditContext::default())
        .await
        .unwrap();
    let audit = AuditLogRepository::new(db.clone());
    let access = AccessContext::from_parts(None, None, None);
    let mut txn = db.begin().await.unwrap();
    erasure::erase(&mut txn, created.id, Some("tester"), &access, &audit)
        .await
        .unwrap();
    txn.commit().await.unwrap();

    sqlx::query("UPDATE person_names SET is_primary = FALSE WHERE person_id = $1")
        .bind(created.id)
        .execute(&db)
        .await
        .unwrap();
    assert!(
        stored_record(&db, created.id).await.is_err(),
        "the simulated pre-fix tombstone must fail to assemble, or this tests nothing"
    );

    // One statement, so it runs as one prepared query.
    sqlx::query(include_str!(
        "../migrations/2026110800000001_erased_tombstone_name_primary.up.sql"
    ))
    .execute(&db)
    .await
    .expect("run the backfill");

    let (stored, digests) = stored_record(&db, created.id)
        .await
        .expect("the backfilled tombstone assembles")
        .expect("the erased row is retained");
    assert_eq!(stored.name.family, erasure::TOMBSTONE_NAME);
    assert_eq!(digests.content_hash, None, "the backfill rehashes nothing");
    assert_eq!(digests.content_mac, None);

    purge(&db, created.id).await;
}
