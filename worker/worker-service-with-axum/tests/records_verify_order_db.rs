#![warn(clippy::pedantic)]

//! DB-gated test that the workers half of `/api/records/verify` reads its
//! rows in a total order (WK-V1): `updated_at DESC, id DESC`, so a capped
//! run samples the same rows every time even when rows share an
//! `updated_at`.

use sqlx::PgPool;
use uuid::Uuid;
use worker_service::db::repositories::rows_for_verification;

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

/// Rows that share one `updated_at` come back in `id DESC` order, and a
/// capped read takes the largest ids among them.
///
/// The rows are inserted directly with an `updated_at` far in the future,
/// so they are the newest rows in the table whatever else it holds (the
/// update trigger only fires on `UPDATE`). They are inserted in ascending
/// id order, the opposite of the order asked for, so a sort that leaves
/// ties in an unspecified order does not pass by accident.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn tied_updated_at_rows_are_read_in_id_order() {
    let db = connect().await;
    let mut ids: Vec<Uuid> = (0..12).map(|_| Uuid::new_v4()).collect();
    ids.sort();
    for id in &ids {
        sqlx::query(
            "INSERT INTO workers (id, gender, updated_at)
             VALUES ($1, 'unknown', TIMESTAMPTZ '2999-01-01 00:00:00+00')",
        )
        .bind(id)
        .execute(&db)
        .await
        .expect("insert a tied row");
    }

    let cap = 5;
    let read = rows_for_verification(&db, i64::try_from(cap).unwrap()).await;

    // Clean up before asserting, so a failure leaves no far-future rows
    // at the top of every later verify run.
    sqlx::query("DELETE FROM workers WHERE id = ANY($1)")
        .bind(&ids)
        .execute(&db)
        .await
        .expect("purge the tied rows");

    let got: Vec<Uuid> = read
        .expect("read the verification rows")
        .into_iter()
        .map(|r| r.id)
        .collect();
    let want: Vec<Uuid> = ids.iter().rev().take(cap).copied().collect();
    assert_eq!(got, want, "tied rows must be read largest id first");
}
