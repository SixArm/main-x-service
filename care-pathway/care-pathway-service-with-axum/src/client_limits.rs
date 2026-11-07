//! Request limits per client, and a lockout after failed authentication,
//! through `axa::ratelimit`. Both are off unless configured.
//!
//! | Variable | Meaning |
//! |---|---|
//! | `RATE_LIMIT_PER_MINUTE` | Requests one client may make per minute, before `429` |
//! | `LOCKOUT_AFTER_FAILURES` | `401`s one client may earn in fifteen minutes, before `429` |
//!
//! A client is the address axa resolved, so behind a proxy `TRUSTED_PROXIES`
//! must list it, or every client shares one limit. Each process counts for
//! itself. Health probes are never limited.

use std::num::NonZeroU32;

use axa::ratelimit::{Limiter, by_client, lock_out_failures};
use axum::{Router, middleware::from_fn_with_state};

/// The window `RATE_LIMIT_PER_MINUTE` counts in, in seconds.
const MINUTE: NonZeroU32 = NonZeroU32::new(60).unwrap();

/// The window `LOCKOUT_AFTER_FAILURES` counts `401`s in, in seconds.
const FIFTEEN_MINUTES: NonZeroU32 = NonZeroU32::new(900).unwrap();

/// `router` with the limits the environment asks for.
///
/// Layer it after the auth guard, so that it sits outside it and counts its
/// `401`s, and inside `axa::middleware::apply`, which resolves the client.
///
/// # Errors
///
/// When a variable is set to anything but a whole number above zero. A
/// limit set wrongly would leave a service open that its operator meant to
/// protect, so the service does not start.
pub fn from_env(router: Router) -> axa::Result<Router> {
    with_limits(router, &|name| std::env::var(name).ok())
}

/// [`from_env`], reading the variables through `lookup`.
///
/// # Errors
///
/// As [`from_env`].
pub fn with_limits(router: Router, lookup: &dyn Fn(&str) -> Option<String>) -> axa::Result<Router> {
    let mut router = router;
    if let Some(failures) = limit(lookup, "LOCKOUT_AFTER_FAILURES")? {
        router = router.layer(from_fn_with_state(
            Limiter::new(failures, FIFTEEN_MINUTES),
            lock_out_failures,
        ));
    }
    if let Some(requests) = limit(lookup, "RATE_LIMIT_PER_MINUTE")? {
        router = router.layer(from_fn_with_state(
            Limiter::new(requests, MINUTE),
            by_client,
        ));
    }
    Ok(router)
}

/// The limit `name` sets, or none when it is unset or blank.
fn limit(lookup: &dyn Fn(&str) -> Option<String>, name: &str) -> axa::Result<Option<NonZeroU32>> {
    let Some(value) = lookup(name) else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value.parse().map(Some).map_err(|_| {
        axa::Error::Message(format!(
            "{name} must be a whole number above zero, not {value:?}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axa::testing::TestServer;
    use axum::{http::StatusCode, routing::get};

    fn lookup(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_string())
        }
    }

    #[test]
    fn unset_or_blank_means_no_limit() {
        assert_eq!(limit(&lookup(&[]), "RATE_LIMIT_PER_MINUTE").unwrap(), None);
        assert_eq!(
            limit(
                &lookup(&[("RATE_LIMIT_PER_MINUTE", " ")]),
                "RATE_LIMIT_PER_MINUTE"
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn a_bad_value_stops_the_service() {
        for bad in ["0", "-1", "lots", "1.5"] {
            let pairs: &'static [(&'static str, &'static str)] =
                Box::leak(vec![("RATE_LIMIT_PER_MINUTE", bad)].into_boxed_slice());
            assert!(
                limit(&lookup(pairs), "RATE_LIMIT_PER_MINUTE").is_err(),
                "{bad}"
            );
        }
    }

    #[tokio::test]
    async fn a_client_past_its_limit_gets_a_429_problem() {
        let router = Router::new().route("/api/thing", get(|| async { "ok" }));
        let router = with_limits(router, &lookup(&[("RATE_LIMIT_PER_MINUTE", "2")])).unwrap();
        let server = TestServer::new(router);
        assert_eq!(server.get("/api/thing").await.status_code(), StatusCode::OK);
        assert_eq!(server.get("/api/thing").await.status_code(), StatusCode::OK);
        let refused = server.get("/api/thing").await;
        assert_eq!(refused.status_code(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(refused.header("content-type"), "application/problem+json");
        assert!(refused.maybe_header("retry-after").is_some());
    }

    #[tokio::test]
    async fn repeated_401s_lock_a_client_out() {
        let router = Router::new().route("/api/login", get(|| async { StatusCode::UNAUTHORIZED }));
        let router = with_limits(router, &lookup(&[("LOCKOUT_AFTER_FAILURES", "2")])).unwrap();
        let server = TestServer::new(router);
        assert_eq!(
            server.get("/api/login").await.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            server.get("/api/login").await.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            server.get("/api/login").await.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[tokio::test]
    async fn no_limits_leave_the_router_as_it_was() {
        let router = Router::new().route("/api/thing", get(|| async { "ok" }));
        let router = with_limits(router, &lookup(&[])).unwrap();
        let server = TestServer::new(router);
        for _ in 0..5 {
            assert_eq!(server.get("/api/thing").await.status_code(), StatusCode::OK);
        }
    }
}
