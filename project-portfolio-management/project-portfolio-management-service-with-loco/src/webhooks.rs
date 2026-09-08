//! Outbound webhook delivery — the family's `WebhookSink` contract
//! (`agents/share/event-bus.md` §12, repo `tasks.md` EV-3, this crate's
//! own `spec/13-tasks.md` T-28m). Portfolio is the first adopter; other
//! registries copy this shape when a consumer asks for it.
//!
//! ## What this is, and is not
//!
//! This is a **best-effort notification** fan-out, not a second durable
//! bus. The relay's primary sink ([`crate::relay::LoggingSink`] /
//! [`crate::relay::FluvioSink`]) is the at-least-once path the family
//! already depends on: a send failure there leaves the outbox row
//! unpublished so [`crate::relay::drain_once`] retries it. Webhook
//! delivery must never gate that — an operator's slow or unreachable
//! receiver would otherwise stall the entire outbox forever. So
//! [`WebhookSink::send`] never returns an error: each matching target
//! gets its own bounded retry-with-backoff on a spawned task, and a
//! target still failing once that is exhausted is recorded in the
//! [`crate::models::webhook_deliveries`] delivery log and moved past,
//! never re-blocking the row it fanned out from. Compose it with the
//! primary sink via [`crate::relay::CompositeSink`].
//!
//! ## Configuration
//!
//! `PROJECT_PORTFOLIO_MANAGEMENT_WEBHOOKS` (inline JSON) or
//! `PROJECT_PORTFOLIO_MANAGEMENT_WEBHOOKS_FILE` (a path to a JSON file;
//! takes precedence when both are set — same precedence rule as the MAC
//! key sourcing, `compliance/mac.rs`) — a JSON array of targets:
//!
//! ```json
//! [
//!   { "url": "https://ops.example.com/hook" },
//!   { "url": "https://audit.example.com/hook", "kinds": ["created", "merged"] }
//! ]
//! ```
//!
//! `kinds` omitted or `null` means every kind. This mirrors the ABAC
//! policy loader's env/file convention
//! (`agents/share/authorization-attributes.md` §5): unset ⇒ no targets
//! (the feature is off); malformed JSON ⇒ logged and treated as no
//! targets, never a boot failure — an optional feature's config typo
//! must not take the service down.
//!
//! Every target is additionally required to be `https://`, or `http://`
//! to a **loopback** host (`127.0.0.1` / `::1` / `localhost`) for local
//! dev/test — the same SEC-V1/SEC-B11 posture the PASETO key fetch and
//! the link-graph presence probe already use. A target that fails this
//! check is dropped at load time with a warning, not silently sent to
//! in plaintext.
//!
//! ## Signing
//!
//! Every delivery carries two headers:
//!
//! - `X-Mxi-Event-Id: <uuid>` — the envelope's dedup key, so a receiver
//!   can deduplicate without parsing the body (`agents/share/event-bus.md`
//!   §6: consumers must be idempotent, keyed on `event_id`).
//! - `X-Mxi-Signature: <tag>` — [`crate::compliance::mac::tag`] under
//!   [`crate::compliance::mac::Domain::Webhook`], i.e. the shared
//!   `integrity-mac` crate's `"<scheme>.<key id>:<hex>"` format,
//!   HMAC-SHA256 over the **exact request body bytes** (below), keyed by
//!   this service's `webhook`-domain subkey.
//!
//! **The pre-image is the exact bytes of the HTTP request body** — the
//! compact `serde_json::to_vec` serialization of the outbox envelope,
//! signed *before* being sent and never re-serialized afterwards, so
//! signer and sender always agree on what was signed.
//!
//! A receiver never holds this service's **root** MAC key — only the
//! `webhook`-domain subkey, which an operator derives offline (HKDF-SHA256
//! over the root key, `info` string `mxi/<service>/webhook/d1` — the
//! shared `integrity_mac` crate's scheme tag `d1`, see
//! `integrity_mac::KeyConfig::info` and the shared crate's module docs,
//! "Domain separation") and hands to the receiver as *their* configured
//! verification secret. The receiver computes HMAC-SHA256(subkey, raw
//! body bytes) and compares the hex after `d1.<key id>:`; a mismatch or
//! an unrecognised key id is a forged or corrupted delivery. Publishing
//! this pre-image format here is what makes the signature checkable at
//! all outside this codebase (repo `tasks.md` EV-3's own condition).
//!
//! No MAC key configured (`compliance::mac::is_enabled()` false) ⇒
//! [`crate::relay::spawn`] refuses to start webhook delivery at all
//! (logged `error`) when targets are configured, rather than silently
//! sending unsigned deliveries — an unsigned webhook is not a smaller
//! version of this feature, it is a different, unauthenticated one. This
//! is the "refuses to start" half of this crate's own T-28m acceptance
//! criterion.
//!
//! ## Retry policy
//!
//! A `5xx` response, or a transport-level failure (no response at all —
//! connection refused, timeout, TLS failure), is **retried**: the
//! receiver or its infrastructure is presumed to be having a transient
//! problem. A `4xx` response is **not retried**: it is the receiver
//! rejecting this exact request, and resending it unchanged would just
//! repeat the rejection. [`MAX_ATTEMPTS`] attempts, exponential backoff
//! starting at [`INITIAL_BACKOFF`] and doubling.

use std::time::Duration;

use sea_orm::DatabaseConnection;
use serde::Deserialize;

use crate::compliance::mac::{self, Domain};
use crate::models::webhook_deliveries::{DeliveryOutcome, Model as DeliveryLog};
use crate::relay::{EventSink, SinkError};

/// The env var holding the inline JSON target list.
pub const WEBHOOKS_ENV: &str = "PROJECT_PORTFOLIO_MANAGEMENT_WEBHOOKS";
/// The env var naming a file holding the JSON target list. Takes
/// precedence over [`WEBHOOKS_ENV`] when both are set.
pub const WEBHOOKS_FILE_ENV: &str = "PROJECT_PORTFOLIO_MANAGEMENT_WEBHOOKS_FILE";

/// The header carrying the envelope dedup id.
pub const EVENT_ID_HEADER: &str = "X-Mxi-Event-Id";
/// The header carrying the HMAC signature (`"<scheme>.<key id>:<hex>"`).
pub const SIGNATURE_HEADER: &str = "X-Mxi-Signature";

/// Maximum HTTP attempts per target per event (the initial attempt plus
/// retries).
pub const MAX_ATTEMPTS: u32 = 4;
/// The first retry's backoff; each subsequent retry doubles it.
pub const INITIAL_BACKOFF: Duration = Duration::from_millis(500);

/// One configured webhook delivery target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WebhookTarget {
    /// The URL delivered to.
    pub url: String,
    /// Event kinds this target receives (`created` / `updated` /
    /// `deleted` / `merged`). `None` (the field omitted, or explicit
    /// `null`) means every kind.
    #[serde(default)]
    pub kinds: Option<Vec<String>>,
}

impl WebhookTarget {
    /// Whether this target should receive an event of `kind`.
    #[must_use]
    pub fn matches_kind(&self, kind: &str) -> bool {
        self.kinds
            .as_ref()
            .is_none_or(|kinds| kinds.iter().any(|k| k == kind))
    }
}

/// Whether a URL may be delivered to (SEC-V1/SEC-B11 posture): `https`
/// to any host, or `http` only to a **loopback** host — the same rule
/// `authentication-verifier`'s key-set fetch and the link-graph presence
/// probe apply. Pure, so it is unit-tested without network access.
#[must_use]
pub fn url_is_permitted(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url.trim()) else {
        return false;
    };
    match parsed.scheme() {
        "https" => true,
        "http" => matches!(
            parsed.host_str(),
            Some("127.0.0.1" | "::1" | "[::1]" | "localhost")
        ),
        _ => false,
    }
}

/// Parse a raw `PROJECT_PORTFOLIO_MANAGEMENT_WEBHOOKS[_FILE]` JSON value
/// into usable targets: malformed JSON becomes an empty list (logged,
/// never a panic or a propagated error — an optional feature's config
/// typo must not block boot, same posture as the ABAC policy loader),
/// and a target failing [`url_is_permitted`] is dropped (logged) rather
/// than sent to in plaintext.
///
/// Pure and env-free, so it is fully unit-testable — [`targets`] is the
/// thin, untested-by-hand edge that supplies `raw` from the process
/// environment.
#[must_use]
pub fn parse_targets(raw: &str) -> Vec<WebhookTarget> {
    if raw.trim().is_empty() {
        return Vec::new();
    }
    let parsed: Vec<WebhookTarget> = match serde_json::from_str(raw) {
        Ok(targets) => targets,
        Err(err) => {
            tracing::warn!(
                error = %err,
                "failed to parse webhook targets; no webhook targets configured"
            );
            return Vec::new();
        }
    };
    parsed
        .into_iter()
        .filter(|t| {
            let ok = url_is_permitted(&t.url);
            if !ok {
                tracing::warn!(
                    url = %t.url,
                    "webhook target is not https:// (or http:// on loopback); dropped"
                );
            }
            ok
        })
        .collect()
}

/// Load the configured webhook targets from the environment:
/// [`WEBHOOKS_FILE_ENV`] (a path) takes precedence over [`WEBHOOKS_ENV`]
/// (inline JSON) when both are set; a file that fails to read is logged
/// and treated as "no targets configured", same as malformed JSON (see
/// [`parse_targets`]).
#[must_use]
pub fn targets() -> Vec<WebhookTarget> {
    let raw = std::env::var(WEBHOOKS_FILE_ENV).ok().map_or_else(
        || std::env::var(WEBHOOKS_ENV).ok(),
        |path| match std::fs::read_to_string(&path) {
            Ok(contents) => Some(contents),
            Err(err) => {
                tracing::warn!(
                    path,
                    error = %err,
                    "failed to read {WEBHOOKS_FILE_ENV}; no webhook targets configured"
                );
                None
            }
        },
    );
    raw.map(|raw| parse_targets(&raw)).unwrap_or_default()
}

/// Whether a failed attempt should be retried: a `5xx` is presumed
/// transient (the receiver or its infrastructure), a `4xx` is the
/// receiver's rejection of this exact request and would just repeat.
#[must_use]
pub fn should_retry_status(status: u16) -> bool {
    (500..600).contains(&status)
}

/// One delivery attempt's result, for the retry loop to act on.
enum AttemptResult {
    Delivered,
    /// Retry-worthy failure: `status` is `None` on a pure transport
    /// error (no response received at all).
    Retryable {
        status: Option<u16>,
        error: String,
    },
    /// A `4xx` — stop retrying immediately.
    Rejected {
        status: u16,
        error: String,
    },
}

/// The shared, **non-redirecting** HTTP client for webhook delivery
/// (SEC-B11 posture): disabling redirects means the only host ever
/// contacted is the operator-configured target — a receiver returning a
/// `3xx` cannot bounce the request to a different, attacker-chosen host.
/// Built once and reused.
fn client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build the non-redirecting webhook-delivery client")
    })
}

/// One HTTP attempt: send `body` (already the signed bytes) to `url`
/// with the given headers.
async fn attempt_once(url: &str, event_id: &str, signature: &str, body: &[u8]) -> AttemptResult {
    let result = client()
        .post(url)
        .header("content-type", "application/json")
        .header(EVENT_ID_HEADER, event_id)
        .header(SIGNATURE_HEADER, signature)
        .body(body.to_vec())
        .send()
        .await;
    match result {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                AttemptResult::Delivered
            } else if should_retry_status(status.as_u16()) {
                AttemptResult::Retryable {
                    status: Some(status.as_u16()),
                    error: format!("HTTP {status}"),
                }
            } else {
                AttemptResult::Rejected {
                    status: status.as_u16(),
                    error: format!("HTTP {status}"),
                }
            }
        }
        Err(err) => AttemptResult::Retryable {
            status: None,
            error: err.to_string(),
        },
    }
}

/// Deliver one event to one target, retrying `5xx`/transport failures
/// with exponential backoff up to [`MAX_ATTEMPTS`], and record the final
/// outcome in the delivery log. Never panics and never propagates an
/// error — this is the fire-and-forget body a spawned task runs.
async fn deliver_with_retry(
    db: &DatabaseConnection,
    target: &WebhookTarget,
    entity: &str,
    kind: &str,
    event_id: uuid::Uuid,
    signature: &str,
    body: &[u8],
) {
    let mut backoff = INITIAL_BACKOFF;
    let mut attempts = 0u32;
    let (delivered, status_code, error) = loop {
        attempts += 1;
        match attempt_once(&target.url, &event_id.to_string(), signature, body).await {
            AttemptResult::Delivered => break (true, None, None),
            AttemptResult::Rejected { status, error } => break (false, Some(status), Some(error)),
            AttemptResult::Retryable { status, error } => {
                if attempts >= MAX_ATTEMPTS {
                    break (false, status, Some(error));
                }
                tracing::warn!(
                    url = %target.url,
                    attempt = attempts,
                    error = %error,
                    "webhook delivery attempt failed; retrying"
                );
                tokio::time::sleep(backoff).await;
                backoff *= 2;
            }
        }
    };
    if !delivered {
        tracing::warn!(
            url = %target.url,
            attempts,
            error = ?error,
            "webhook delivery exhausted retries; recorded as failed"
        );
    }
    let outcome = DeliveryOutcome {
        event_id,
        entity: entity.to_string(),
        kind: kind.to_string(),
        url: target.url.clone(),
        attempts,
        delivered,
        status_code,
        error,
    };
    if let Err(err) = DeliveryLog::record(db, &outcome).await {
        tracing::warn!(error = %err, "failed to write webhook delivery-log row");
    }
}

/// The outbound webhook [`EventSink`]. Holds the configured targets and
/// a database handle for the delivery log.
///
/// Constructing this with targets configured but no MAC key enabled is
/// a caller error — [`crate::relay::spawn`] checks
/// [`mac::is_enabled`] first and refuses to build this sink at all in
/// that case (see the module docs' "Signing" section).
pub struct WebhookSink {
    targets: Vec<WebhookTarget>,
    db: DatabaseConnection,
}

impl WebhookSink {
    /// Build a sink over the given targets.
    #[must_use]
    pub fn new(targets: Vec<WebhookTarget>, db: DatabaseConnection) -> Self {
        Self { targets, db }
    }
}

#[async_trait::async_trait]
impl EventSink for WebhookSink {
    /// Fan out `payload` to every matching target on its own spawned
    /// task and return immediately — see the module docs for why this
    /// never blocks or fails the caller. Always returns `Ok`.
    async fn send(
        &self,
        entity: &str,
        _key: &str,
        payload: &serde_json::Value,
    ) -> Result<(), SinkError> {
        let kind = payload.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        let event_id = payload
            .get("event_id")
            .and_then(|v| v.as_str())
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .unwrap_or_else(uuid::Uuid::nil);
        let body = match serde_json::to_vec(payload) {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::warn!(error = %err, "failed to serialize webhook payload; skipping");
                return Ok(());
            }
        };
        let Some(signature) = mac::tag(Domain::Webhook, &body) else {
            // Constructed without a key configured — refuse to send
            // unsigned deliveries rather than silently downgrade.
            tracing::error!(
                "webhook targets are configured but no integrity-MAC key is active; \
                 skipping delivery rather than sending an unsigned request"
            );
            return Ok(());
        };
        for target in &self.targets {
            if !target.matches_kind(kind) {
                continue;
            }
            let target = target.clone();
            let entity = entity.to_string();
            let kind = kind.to_string();
            let signature = signature.clone();
            let body = body.clone();
            let db = self.db.clone();
            tokio::spawn(async move {
                deliver_with_retry(&db, &target, &entity, &kind, event_id, &signature, &body).await;
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{WebhookTarget, parse_targets, should_retry_status, url_is_permitted};

    #[test]
    fn https_to_any_host_is_permitted() {
        assert!(url_is_permitted("https://ops.example.com/hook"));
    }

    #[test]
    fn http_to_a_non_loopback_host_is_refused() {
        assert!(!url_is_permitted("http://ops.example.com/hook"));
    }

    #[test]
    fn http_to_loopback_is_permitted_for_dev() {
        assert!(url_is_permitted("http://127.0.0.1:8080/hook"));
        assert!(url_is_permitted("http://localhost:8080/hook"));
    }

    #[test]
    fn an_unparseable_url_is_refused() {
        assert!(!url_is_permitted("not a url"));
    }

    #[test]
    fn a_non_http_scheme_is_refused() {
        assert!(!url_is_permitted("file:///etc/passwd"));
    }

    #[test]
    fn a_5xx_is_retried_and_a_4xx_is_not() {
        // Pins this crate's own T-28m acceptance criterion directly.
        assert!(should_retry_status(500));
        assert!(should_retry_status(503));
        assert!(should_retry_status(599));
        assert!(!should_retry_status(400));
        assert!(!should_retry_status(404));
        assert!(!should_retry_status(429));
        assert!(!should_retry_status(200));
    }

    #[test]
    fn a_target_with_no_kinds_matches_every_kind() {
        let target = WebhookTarget {
            url: "https://example.com/hook".to_string(),
            kinds: None,
        };
        assert!(target.matches_kind("created"));
        assert!(target.matches_kind("merged"));
    }

    #[test]
    fn a_target_with_kinds_matches_only_those() {
        let target = WebhookTarget {
            url: "https://example.com/hook".to_string(),
            kinds: Some(vec!["created".to_string(), "merged".to_string()]),
        };
        assert!(target.matches_kind("created"));
        assert!(target.matches_kind("merged"));
        assert!(!target.matches_kind("updated"));
        assert!(!target.matches_kind("deleted"));
    }

    #[test]
    fn parsing_a_target_list_reads_the_kinds_filter() {
        let json = r#"[
            {"url": "https://ops.example.com/hook"},
            {"url": "https://audit.example.com/hook", "kinds": ["created", "merged"]}
        ]"#;
        let targets: Vec<WebhookTarget> = serde_json::from_str(json).unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].kinds, None);
        assert_eq!(
            targets[1].kinds,
            Some(vec!["created".to_string(), "merged".to_string()])
        );
    }

    #[test]
    fn parse_targets_drops_a_non_https_entry_and_keeps_the_rest() {
        let targets = parse_targets(
            r#"[{"url": "http://not-loopback.example.com/hook"}, {"url": "https://ok.example.com/hook"}]"#,
        );
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].url, "https://ok.example.com/hook");
    }

    /// Malformed JSON must not block boot — the config loader falls
    /// back to "no targets" rather than propagating a parse error.
    #[test]
    fn parse_targets_on_malformed_json_returns_empty_rather_than_panicking() {
        assert!(parse_targets("not json").is_empty());
    }

    /// An empty/unset value is "no targets configured", not a parse
    /// error — the common "feature not in use" case must be silent.
    #[test]
    fn parse_targets_on_blank_input_returns_empty() {
        assert!(parse_targets("").is_empty());
        assert!(parse_targets("   ").is_empty());
    }
}
