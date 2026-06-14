//! Prometheus metrics for the authentication service.
//!
//! This module owns a process-wide [`Registry`](prometheus::Registry)
//! populated with a fixed set of auth-specific counters. Application code
//! increments the global [`Metrics`] handle returned by
//! [`Metrics::global`] (e.g. `Metrics::global().signup_total.inc()`). The
//! HTTP surface exposes the registry at `GET /metrics.prom` in Prometheus
//! text-exposition format (see [`crate::controllers::metrics`]). Configure
//! your scraper with `metrics_path: /metrics.prom`.
//!
//! Unlike the entity-CRUD services in this family, the authentication
//! service has no entity to count creates/updates/deletes on. Its metric
//! set therefore tracks the **passwordless magic-link lifecycle**:
//! signups, magic links issued, magic links redeemed, signouts, and the
//! rate-limited rejections that protect those flows.
//!
//! # Privacy
//!
//! These are **counters with no per-subject labels**. Email addresses,
//! magic-link tokens, JWTs, and user pids are never used as label values
//! — a metric label is high-cardinality and is exported in the clear, so
//! putting a subject identifier there would leak personal data into the
//! monitoring system. The only labelled metric is
//! [`Metrics::http_requests_total`], whose labels are the HTTP method,
//! the route *template* (never a concrete token or id), and the status
//! code. The DB-free test [`tests::render_has_no_secret_labels`] pins this
//! contract.
//!
//! # Metric inventory
//!
//! | Name | Type | Labels |
//! |---|---|---|
//! | `auth_signup_total` | counter | — |
//! | `auth_magic_link_issued_total` | counter | — |
//! | `auth_magic_link_redeemed_total` | counter | — |
//! | `auth_signout_total` | counter | — |
//! | `auth_rate_limited_total` | counter | — |
//! | `http_requests_total` | counter vec | `method`, `path`, `status` |

use prometheus::{Counter, Encoder, IntCounterVec, Opts, Registry, TextEncoder};
use std::sync::OnceLock;

/// Process-wide Prometheus registry plus the auth-specific metric handles.
/// Cloning a counter handle is cheap (`Arc` under the hood); always go
/// through [`Metrics::global`] rather than re-creating them, so every
/// increment lands in the one registry that `/metrics.prom` renders.
pub struct Metrics {
    /// The underlying registry. Exposed for callers that want to register
    /// service-specific metrics beyond this default set.
    pub registry: Registry,

    /// Count of successful account signups (a passwordless account was
    /// created or an existing one re-requested a link via signup).
    pub signup_total: Counter,
    /// Count of magic links issued (delivered to the tracing log in dev,
    /// emailed in prod) across both the signup and sign-in flows.
    pub magic_link_issued_total: Counter,
    /// Count of magic links successfully redeemed for an access token.
    pub magic_link_redeemed_total: Counter,
    /// Count of session signouts (a session was revoked).
    pub signout_total: Counter,
    /// Count of requests rejected by the per-email rate limiter (the
    /// `429` path on both signup and magic-link request).
    pub rate_limited_total: Counter,

    /// HTTP requests, labeled by method, route template, and status code.
    /// Labels never contain a concrete token, email, or pid.
    pub http_requests_total: IntCounterVec,
}

impl Metrics {
    /// Construct the full metric set and register every collector into a
    /// fresh [`Registry`]. Called once by [`Metrics::global`]; the
    /// `.expect`s are infallible because all opts/collectors are
    /// statically valid (constant names, no duplicate registration).
    fn new() -> Self {
        let registry = Registry::new();

        let signup_total = Counter::with_opts(Opts::new(
            "auth_signup_total",
            "Total successful passwordless account signups.",
        ))
        .expect("static counter opts are always valid");
        let magic_link_issued_total = Counter::with_opts(Opts::new(
            "auth_magic_link_issued_total",
            "Total magic links issued (signup + sign-in flows).",
        ))
        .expect("static counter opts are always valid");
        let magic_link_redeemed_total = Counter::with_opts(Opts::new(
            "auth_magic_link_redeemed_total",
            "Total magic links successfully redeemed for an access token.",
        ))
        .expect("static counter opts are always valid");
        let signout_total = Counter::with_opts(Opts::new(
            "auth_signout_total",
            "Total session signouts (sessions revoked).",
        ))
        .expect("static counter opts are always valid");
        let rate_limited_total = Counter::with_opts(Opts::new(
            "auth_rate_limited_total",
            "Total requests rejected by the per-email magic-link rate limiter (429).",
        ))
        .expect("static counter opts are always valid");

        let http_requests_total = IntCounterVec::new(
            Opts::new(
                "http_requests_total",
                "Total HTTP requests handled, labeled by method, route template, and status.",
            ),
            &["method", "path", "status"],
        )
        .expect("static counter-vec opts are always valid");

        for c in [
            Box::new(signup_total.clone()) as Box<dyn prometheus::core::Collector>,
            Box::new(magic_link_issued_total.clone()),
            Box::new(magic_link_redeemed_total.clone()),
            Box::new(signout_total.clone()),
            Box::new(rate_limited_total.clone()),
        ] {
            registry
                .register(c)
                .expect("registering a freshly-constructed collector cannot fail");
        }
        registry
            .register(Box::new(http_requests_total.clone()))
            .expect("registering a freshly-constructed collector cannot fail");

        Self {
            registry,
            signup_total,
            magic_link_issued_total,
            magic_link_redeemed_total,
            signout_total,
            rate_limited_total,
            http_requests_total,
        }
    }

    /// The process-wide auth-service metrics, initialised on first access.
    pub fn global() -> &'static Self {
        static METRICS: OnceLock<Metrics> = OnceLock::new();
        METRICS.get_or_init(Metrics::new)
    }

    /// Render the registry to Prometheus text exposition format
    /// (`text/plain; version=0.0.4`). Each metric family appears with its
    /// `# HELP` and `# TYPE` header lines followed by the sample lines.
    ///
    /// # Panics
    ///
    /// Never in practice. The two `.expect`s are unreachable: the
    /// `TextEncoder` writes into an in-memory `Vec` (which cannot fail on
    /// I/O), and that encoder only ever emits valid UTF-8.
    #[must_use]
    pub fn render(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buf = Vec::new();
        encoder
            .encode(&metric_families, &mut buf)
            .expect("text encoder writes to a Vec which never fails");
        String::from_utf8(buf).expect("Prometheus text encoder produces UTF-8")
    }
}

/// Content type for the Prometheus text-exposition format. Use this on
/// HTTP responses serving [`Metrics::render`].
pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Tests for the Prometheus registry and text rendering. DB-free: they
/// touch only the process-wide registry, never the database.
#[cfg(test)]
mod tests {
    use super::*;

    /// Incrementing each counter is reflected in the rendered exposition,
    /// and every metric family carries the `# HELP` / `# TYPE` header
    /// lines that make the output valid Prometheus text.
    #[test]
    fn render_is_valid_prometheus_text() {
        let m = Metrics::global();
        m.signup_total.inc();
        m.magic_link_issued_total.inc();
        m.magic_link_redeemed_total.inc();
        m.signout_total.inc();
        m.rate_limited_total.inc();
        m.http_requests_total
            .with_label_values(&["POST", "/api/auth/signup", "200"])
            .inc();

        let body = m.render();

        for name in [
            "auth_signup_total",
            "auth_magic_link_issued_total",
            "auth_magic_link_redeemed_total",
            "auth_signout_total",
            "auth_rate_limited_total",
            "http_requests_total",
        ] {
            assert!(body.contains(name), "missing metric {name}; got: {body}");
            assert!(
                body.contains(&format!("# HELP {name} ")),
                "missing HELP for {name}; got: {body}"
            );
            assert!(
                body.contains(&format!("# TYPE {name} ")),
                "missing TYPE for {name}; got: {body}"
            );
        }
    }

    /// The exposition must never leak a subject identifier. Only the
    /// `http_requests_total` label set (`method` / `path` / `status`) is
    /// allowed; no `email`, token, pid, or magic-link material may appear
    /// as a label key or value.
    #[test]
    fn render_has_no_secret_labels() {
        let m = Metrics::global();
        m.http_requests_total
            .with_label_values(&["GET", "/api/auth/me", "200"])
            .inc();
        let body = m.render();
        for forbidden in ["email", "token", "magic_link", "pid", "jti", "user_pid"] {
            assert!(
                !body.contains(&format!("{forbidden}=")),
                "metric label `{forbidden}=` would leak a subject identifier; got: {body}"
            );
        }
    }
}
