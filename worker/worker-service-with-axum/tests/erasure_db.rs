#![warn(clippy::pedantic)]

//! DB-gated tests for what a GDPR Art. 17 erasure leaves behind: no keyed
//! digest of the erased content (WK-E1), and no audit row about the
//! subject left unredacted by a writer racing the erasure (WK-E2).
//!
//! A test binary of its own because the record MAC is only written when a
//! key is configured, and the key is read once per process: setting it
//! here, before anything computes a MAC, leaves every other suite's
//! process without one, as it was.

use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use sqlx::{Connection as _, PgPool};
use uuid::Uuid;
use worker_service::compliance::disclosure::AccessContext;
use worker_service::compliance::erasure;
use worker_service::db::audit::{AuditActor, CHAIN_LOCK_KEY};
use worker_service::db::{AuditContext, AuditLogRepository, PgWorkerRepository, WorkerRepository};
use worker_service::models::{Gender, HumanName, Worker};

/// Configure a MAC key for this process, once, before any MAC is taken.
fn with_mac_key() {
    static KEY: Once = Once::new();
    KEY.call_once(|| {
        // Fixed rather than generated, so a rerun against the same database
        // verifies the MACs an earlier run wrote. A test key, never a
        // deployment's.
        let key = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        // SAFETY: `set_var` is `unsafe` in edition 2024 because another
        // thread may read the environment concurrently. This runs once,
        // before the first MAC is computed (the key set is read on first
        // use), and the DB suites run with one test thread.
        unsafe {
            std::env::set_var(worker_service::compliance::mac::KEY_ENV, key);
        }
    });
    assert!(
        worker_service::compliance::mac::is_enabled(),
        "the MAC key must load, or the MAC checks below check nothing"
    );
}

/// A pool on `DATABASE_URL`, migrated.
async fn connect() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
    let db = PgPool::connect(&url)
        .await
        .expect("connect to DATABASE_URL");
    worker_service::db::MIGRATOR
        .run(&db)
        .await
        .expect("run migrations");
    db
}

fn a_worker(family: &str) -> Worker {
    Worker::new(
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

/// The stored `(content_hash, content_hash_sha3, content_mac)` of `id`.
async fn digests(db: &PgPool, id: Uuid) -> (Option<String>, Option<String>, Option<String>) {
    sqlx::query_as("SELECT content_hash, content_hash_sha3, content_mac FROM workers WHERE id = $1")
        .bind(id)
        .fetch_one(db)
        .await
        .expect("read the digests")
}

/// Hard-delete a test record, so its MAC (made with this process's key)
/// is not left for another suite to find.
async fn purge(db: &PgPool, id: Uuid) {
    for sql in [
        "DELETE FROM worker_assessments WHERE worker_id = $1",
        "DELETE FROM workers WHERE id = $1",
    ] {
        sqlx::query(sql)
            .bind(id)
            .execute(db)
            .await
            .expect("purge test record");
    }
}

/// WK-E1: erasure leaves no keyed digest of the erased content. It nulls
/// the MAC as it nulls the two hashes.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn erasure_nulls_the_mac_with_the_hashes() {
    with_mac_key();
    let db = connect().await;
    let repo = PgWorkerRepository::new(db.clone());
    let created = repo
        .create(&a_worker("ErasedMac"), &AuditContext::default())
        .await
        .unwrap();
    let (_, _, mac) = digests(&db, created.id).await;
    assert!(mac.is_some(), "a fresh record carries a MAC");

    let audit = AuditLogRepository::new(db.clone());
    let access = AccessContext::from_parts(None, None, None);
    let mut txn = db.begin().await.unwrap();
    erasure::erase(&mut txn, created.id, Some("tester"), &access, &audit)
        .await
        .unwrap();
    txn.commit().await.unwrap();

    let (hash, sha3, mac) = digests(&db, created.id).await;
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

/// WK-E2: an audit row about the subject that is appended while the
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
    let repo = PgWorkerRepository::new(db.clone()).with_audit_log(Arc::clone(&audit));
    let created = repo
        .create(&a_worker("ErasedLate"), &AuditContext::default())
        .await
        .unwrap();
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
        .log_access_on(
            &mut *holder,
            "Worker",
            id,
            "READ",
            &AuditActor::default(),
            &AccessContext::from_parts(Some("care"), None, None),
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
         WHERE entity_id = $1 AND action <> $2 AND redacted_at IS NULL",
    )
    .bind(id)
    .bind(erasure::ACTION_ERASED)
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
