//! DB-gated tests for the Postgres-backed magic-link rate limiter
//! (`src/rate_limit.rs`).
//!
//! These exercise the sliding-window behaviour directly against the
//! `auth_rate_limits` table, injecting a synthetic `now` through
//! [`rate_limit::check_at`] for determinism. They boot the loco app and so
//! need the PostgreSQL instance from `config/test.yaml`; they are
//! `#[ignore]`d (run with `cargo test -- --ignored`) and `#[serial]` because
//! they share the one table. The pure key-normalisation case is unit-tested
//! DB-free in the module itself.

use authentication_service::{app::App, rate_limit};
use chrono::{Duration, Utc};
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// The window as a `chrono::Duration`, for building injected `now` offsets.
fn window() -> Duration {
    Duration::from_std(rate_limit::WINDOW).expect("WINDOW fits in chrono::Duration")
}

/// The first `MAX_REQUESTS` requests in a window are allowed; the next is
/// throttled with a `RetryAfter` no longer than the window.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn allows_up_to_max_then_rejects_the_next() {
    request::<App, _, _>(|_request, ctx| async move {
        rate_limit::reset(&ctx.db).await.unwrap();
        let key = "rl-allows@example.com";
        let now = Utc::now();
        for i in 0..rate_limit::MAX_REQUESTS {
            assert!(
                rate_limit::check_at(&ctx.db, key, now).await.is_ok(),
                "request {i} (<= MAX_REQUESTS) should be allowed"
            );
        }
        let err = rate_limit::check_at(&ctx.db, key, now)
            .await
            .expect_err("the N+1th request must be throttled");
        assert!(err.0 <= rate_limit::WINDOW);
    })
    .await;
}

/// Once the whole window has elapsed, the bucket is empty again.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn window_resets_after_the_window_elapses() {
    request::<App, _, _>(|_request, ctx| async move {
        rate_limit::reset(&ctx.db).await.unwrap();
        let key = "rl-resets@example.com";
        let start = Utc::now();
        for _ in 0..rate_limit::MAX_REQUESTS {
            rate_limit::check_at(&ctx.db, key, start)
                .await
                .expect("within quota");
        }
        rate_limit::check_at(&ctx.db, key, start)
            .await
            .expect_err("over quota at the start");

        let later = start + window() + Duration::seconds(1);
        assert!(
            rate_limit::check_at(&ctx.db, key, later).await.is_ok(),
            "the window should reset after WINDOW elapses"
        );
    })
    .await;
}

/// As each request ages out, exactly one slot frees — not the whole window.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn sliding_window_frees_one_slot_at_a_time() {
    request::<App, _, _>(|_request, ctx| async move {
        rate_limit::reset(&ctx.db).await.unwrap();
        let key = "rl-sliding@example.com";
        let start = Utc::now();
        // Spread MAX_REQUESTS requests one second apart.
        for i in 0..rate_limit::MAX_REQUESTS {
            rate_limit::check_at(&ctx.db, key, start + Duration::seconds(i))
                .await
                .expect("within quota");
        }
        // Still full at the moment the last one landed.
        let full_at = start + Duration::seconds(rate_limit::MAX_REQUESTS - 1);
        rate_limit::check_at(&ctx.db, key, full_at)
            .await
            .expect_err("bucket is full");

        // Just after the *first* request ages out, exactly one slot frees.
        let one_freed = start + window() + Duration::milliseconds(1);
        rate_limit::check_at(&ctx.db, key, one_freed)
            .await
            .expect("one slot should have freed");
        // ...but only one: the next is rejected again.
        rate_limit::check_at(&ctx.db, key, one_freed)
            .await
            .expect_err("only one slot frees per aged-out request");
    })
    .await;
}

/// Distinct keys have independent quotas.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn distinct_keys_have_independent_quotas() {
    request::<App, _, _>(|_request, ctx| async move {
        rate_limit::reset(&ctx.db).await.unwrap();
        let now = Utc::now();
        for _ in 0..rate_limit::MAX_REQUESTS {
            rate_limit::check_at(&ctx.db, "rl-a@example.com", now)
                .await
                .expect("a within quota");
        }
        rate_limit::check_at(&ctx.db, "rl-a@example.com", now)
            .await
            .expect_err("a is over quota");
        // A different key is unaffected.
        rate_limit::check_at(&ctx.db, "rl-b@example.com", now)
            .await
            .expect("b has its own quota");
    })
    .await;
}

/// Differently-spelled but equal emails share one bucket (normalisation).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn normalized_keys_share_a_bucket() {
    request::<App, _, _>(|_request, ctx| async move {
        rate_limit::reset(&ctx.db).await.unwrap();
        let now = Utc::now();
        for _ in 0..rate_limit::MAX_REQUESTS {
            rate_limit::check_at(&ctx.db, "  Mixed@Example.com ", now)
                .await
                .expect("within quota");
        }
        rate_limit::check_at(&ctx.db, "mixed@example.com", now)
            .await
            .expect_err("normalised spellings share a quota");
    })
    .await;
}

/// Rejected attempts are not recorded, so they cannot push the window
/// forward; the bucket frees on schedule from the *original* requests.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn rejection_does_not_consume_further_quota() {
    request::<App, _, _>(|_request, ctx| async move {
        rate_limit::reset(&ctx.db).await.unwrap();
        let key = "rl-no-consume@example.com";
        let now = Utc::now();
        for _ in 0..rate_limit::MAX_REQUESTS {
            rate_limit::check_at(&ctx.db, key, now)
                .await
                .expect("within quota");
        }
        for _ in 0..10 {
            rate_limit::check_at(&ctx.db, key, now)
                .await
                .expect_err("over quota");
        }
        let later = now + window() + Duration::seconds(1);
        rate_limit::check_at(&ctx.db, key, later)
            .await
            .expect("window should free despite the rejected attempts");
    })
    .await;
}
