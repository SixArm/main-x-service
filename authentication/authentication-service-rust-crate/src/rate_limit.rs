//! In-memory, per-key sliding-window rate limiter for magic-link issuance.
//!
//! The unauthenticated magic-link endpoints (`POST /api/auth/signup` and
//! `POST /api/auth/magic-link`) can be abused to email-bomb a victim or to
//! probe for the existence of accounts. This module throttles issuance by a
//! **normalised email** key: at most [`MAX_REQUESTS`] requests per
//! [`WINDOW`].
//!
//! Design notes:
//!
//! - **Monotonic clock.** The window is measured with [`Instant`], not
//!   wall-clock time, so it is immune to clock skew and to the env-free
//!   constraint of the loco boot. Tests inject a synthetic "now" through
//!   [`check_at`] for determinism.
//! - **Process-wide store.** Loco has no per-request shared state for this,
//!   so the store is a `OnceLock`-initialised global, mirroring the family's
//!   in-memory event-stream pattern.
//! - **Anti-enumeration preserved.** The limiter keys on request *volume*,
//!   never on whether an account exists; the handlers keep their always-`200`
//!   success shape and only swap in `429` once the volume cap is exceeded.

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Maximum number of issuance requests allowed per key within [`WINDOW`].
/// The N+1th request inside the window is rejected.
pub const MAX_REQUESTS: usize = 5;

/// Length of the sliding window over which [`MAX_REQUESTS`] is counted.
/// Five minutes balances legitimate retries (a user who mistypes, or whose
/// first email is slow) against abuse.
pub const WINDOW: Duration = Duration::from_mins(5);

/// How long a request is rejected for: the time until the oldest in-window
/// request ages out. Returned to callers so they can surface `Retry-After`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryAfter(pub Duration);

/// Normalise an email for keying: trim surrounding whitespace and lowercase.
/// This collapses trivially-distinct spellings (`Alice@X.com `,
/// `alice@x.com`) onto a single bucket so they share one quota.
#[must_use]
pub fn normalize_key(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Per-key history of in-window request instants (oldest at the front).
type Store = Mutex<HashMap<String, VecDeque<Instant>>>;

fn store() -> &'static Store {
    static STORE: OnceLock<Store> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Record a request for `key` and decide whether it is allowed, using the
/// real monotonic clock. Returns `Ok(())` when the request is within the
/// quota, or `Err(RetryAfter)` when the cap is exceeded (in which case the
/// request is *not* counted, so a throttled caller cannot push the window
/// forward).
///
/// # Errors
///
/// Returns [`RetryAfter`] when `key` already has [`MAX_REQUESTS`] requests
/// inside the current [`WINDOW`].
pub fn check(key: &str) -> Result<(), RetryAfter> {
    check_at(key, Instant::now())
}

/// Clock-injectable core of [`check`]. `now` is the synthetic instant to
/// evaluate the window against; production callers pass `Instant::now()`.
/// Exposed for deterministic unit tests.
///
/// # Errors
///
/// Returns [`RetryAfter`] when `key` already has [`MAX_REQUESTS`] requests
/// inside the window ending at `now`.
pub fn check_at(key: &str, now: Instant) -> Result<(), RetryAfter> {
    let key = normalize_key(key);
    let mut map = store()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let history = map.entry(key).or_default();

    // Drop requests that have aged out of the window.
    while let Some(&front) = history.front() {
        if now.duration_since(front) >= WINDOW {
            history.pop_front();
        } else {
            break;
        }
    }

    // `history.front()` is `Some` whenever the bucket is full (a non-empty
    // `VecDeque`), so the cap check piggybacks on it without an `unwrap`.
    if let Some(&oldest) = history.front()
        && history.len() >= MAX_REQUESTS
    {
        // Rejected: do not record this request. Retry once the oldest
        // in-window request ages out.
        let retry = WINDOW.saturating_sub(now.duration_since(oldest));
        return Err(RetryAfter(retry));
    }

    history.push_back(now);
    Ok(())
}

/// Clear all rate-limit state. A test-support helper so that suites which
/// share the process-wide store (unit tests here and the DB-gated request
/// tests) can start from a known-empty slate. Not used on any production
/// code path.
pub fn reset() {
    store()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests share the process-wide store, so they must not run
    // concurrently with one another. A test-local mutex serialises them
    // (cargo runs tests in parallel by default); each guards `reset()` +
    // its assertions under the same lock.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    fn normalize_key_trims_and_lowercases() {
        assert_eq!(normalize_key("  Alice@Example.COM "), "alice@example.com");
    }

    #[test]
    fn allows_up_to_max_then_rejects_the_next() {
        let _g = guard();
        reset();
        let key = "allows-up-to-max@example.com";
        let now = Instant::now();
        for i in 0..MAX_REQUESTS {
            assert!(
                check_at(key, now).is_ok(),
                "request {i} (<= MAX_REQUESTS) should be allowed"
            );
        }
        // The MAX_REQUESTS+1th request inside the window is rejected.
        let err = check_at(key, now).expect_err("the N+1th request must be throttled");
        assert!(err.0 <= WINDOW);
    }

    #[test]
    fn window_resets_after_the_window_elapses() {
        let _g = guard();
        reset();
        let key = "window-resets@example.com";
        let start = Instant::now();
        for _ in 0..MAX_REQUESTS {
            check_at(key, start).expect("within quota");
        }
        check_at(key, start).expect_err("over quota at the start");

        // Once the whole window has elapsed, the bucket is empty again.
        let later = start + WINDOW + Duration::from_secs(1);
        assert!(
            check_at(key, later).is_ok(),
            "the window should reset after WINDOW elapses"
        );
    }

    #[test]
    fn sliding_window_frees_one_slot_at_a_time() {
        let _g = guard();
        reset();
        let key = "sliding@example.com";
        let start = Instant::now();
        // Spread MAX_REQUESTS requests one second apart.
        for i in 0..MAX_REQUESTS {
            check_at(key, start + Duration::from_secs(i as u64)).expect("within quota");
        }
        // Still full at the moment the last one landed.
        let full_at = start + Duration::from_secs((MAX_REQUESTS - 1) as u64);
        check_at(key, full_at).expect_err("bucket is full");

        // Just after the *first* request ages out, exactly one slot frees.
        let one_freed = start + WINDOW + Duration::from_millis(1);
        check_at(key, one_freed).expect("one slot should have freed");
        // ...but only one: the next is rejected again.
        check_at(key, one_freed).expect_err("only one slot frees per aged-out request");
    }

    #[test]
    fn distinct_keys_have_independent_quotas() {
        let _g = guard();
        reset();
        let now = Instant::now();
        for _ in 0..MAX_REQUESTS {
            check_at("a@example.com", now).expect("a within quota");
        }
        check_at("a@example.com", now).expect_err("a is over quota");
        // A different key is unaffected.
        check_at("b@example.com", now).expect("b has its own quota");
    }

    #[test]
    fn normalized_keys_share_a_bucket() {
        let _g = guard();
        reset();
        let now = Instant::now();
        for _ in 0..MAX_REQUESTS {
            check_at("  Mixed@Example.com ", now).expect("within quota");
        }
        // A differently-spelled but equal email shares the same bucket.
        check_at("mixed@example.com", now).expect_err("normalised spellings share a quota");
    }

    #[test]
    fn rejection_does_not_consume_further_quota() {
        let _g = guard();
        reset();
        let key = "no-consume@example.com";
        let now = Instant::now();
        for _ in 0..MAX_REQUESTS {
            check_at(key, now).expect("within quota");
        }
        // Many rejected attempts must not push the window forward; once the
        // window elapses from the *original* requests, the bucket frees.
        for _ in 0..10 {
            check_at(key, now).expect_err("over quota");
        }
        let later = now + WINDOW + Duration::from_secs(1);
        check_at(key, later).expect("window should free despite the rejected attempts");
    }
}
