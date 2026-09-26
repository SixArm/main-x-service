//! Live worker-skill resolution over the worker service, by
//! `EntityRef` (T-28c). Lazy verify-on-read, TTL-cached in-process,
//! **never persisted** — people stay references (family doctrine); a
//! stored "has skill X" flag would drift the moment the worker
//! service's own record changed. Every failure mode (unconfigured,
//! non-worker reference, unreachable, non-2xx, malformed body)
//! resolves to an `Err` reason — the caller reports this as
//! [`crate::visibility::SkillStatus::Unknown`], never `Missing`,
//! because nothing was actually checked.
//!
//! ## The assumed contract, and why it is currently always unreachable
//!
//! This calls `GET {base}/api/workers/{id}/skills`, expecting a JSON
//! body `{"skills": ["tag", ...]}`. **The worker service carries no
//! such endpoint today** — confirmed by reading its `Worker` and
//! `Assessment` models directly rather than assumed: `Worker` has no
//! skills-tag field, and its `Assessment` machinery scores psychometric
//! scales (Watson-Glaser, SHL, …), not a short-tag vocabulary like
//! `allocations.skills_required`. So until (and unless) a future,
//! separately-scoped worker-service task adds this endpoint, every
//! real resolution in this deployment reports `unknown` — honestly,
//! not silently, and that is by design rather than a defect here. See
//! `spec/13-tasks.md` T-28c for the full "decided rather than guessed"
//! note; inventing that endpoint inside the worker crate as a side
//! effect of this task would be exactly the kind of unsupervised
//! cross-crate decision the family's discipline avoids.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use uuid::Uuid;

/// `PROJECT_PORTFOLIO_MANAGEMENT_WORKER_SERVICE_URL` — unset ⇒ no
/// resolver configured, every reference resolves `Unknown` without a
/// network call.
pub const BASE_URL_ENV: &str = "PROJECT_PORTFOLIO_MANAGEMENT_WORKER_SERVICE_URL";
/// `PROJECT_PORTFOLIO_MANAGEMENT_WORKER_SKILLS_CACHE_TTL_SECONDS` —
/// defaults to [`DEFAULT_CACHE_TTL_SECONDS`].
pub const CACHE_TTL_ENV: &str = "PROJECT_PORTFOLIO_MANAGEMENT_WORKER_SKILLS_CACHE_TTL_SECONDS";
/// Default cache TTL when [`CACHE_TTL_ENV`] is unset or unparseable.
pub const DEFAULT_CACHE_TTL_SECONDS: u64 = 300;

#[derive(Clone)]
enum CacheEntry {
    Resolved(Vec<String>),
    Unresolved(String),
}

fn cache() -> &'static Mutex<HashMap<Uuid, (CacheEntry, Instant)>> {
    static CACHE: OnceLock<Mutex<HashMap<Uuid, (CacheEntry, Instant)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_ttl() -> Duration {
    std::env::var(CACHE_TTL_ENV)
        .ok()
        .and_then(|v| v.parse().ok())
        .map_or(
            Duration::from_secs(DEFAULT_CACHE_TTL_SECONDS),
            Duration::from_secs,
        )
}

fn cached(pid: Uuid) -> Option<Result<Vec<String>, String>> {
    let guard = cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (entry, at) = guard.get(&pid)?;
    if at.elapsed() > cache_ttl() {
        return None;
    }
    Some(match entry {
        CacheEntry::Resolved(tags) => Ok(tags.clone()),
        CacheEntry::Unresolved(reason) => Err(reason.clone()),
    })
}

fn store(pid: Uuid, result: &Result<Vec<String>, String>) {
    let mut guard = cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let entry = match result {
        Ok(tags) => CacheEntry::Resolved(tags.clone()),
        Err(reason) => CacheEntry::Unresolved(reason.clone()),
    };
    guard.insert(pid, (entry, Instant::now()));
}

/// The non-redirecting HTTP client (SEC-B11 posture, mirroring
/// `webhooks::client`): the only host ever contacted is the
/// operator-configured worker-service base URL.
fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build the non-redirecting worker-skills client")
    })
}

#[derive(serde::Deserialize)]
struct SkillsResponse {
    #[serde(default)]
    skills: Vec<String>,
}

/// A `worker:<uuid>` reference's worker pid; `None` for any other
/// shape (including `person:`, which has no worker skill profile).
fn worker_pid_of(person_ref: &str) -> Option<Uuid> {
    Uuid::parse_str(person_ref.strip_prefix("worker:")?).ok()
}

async fn fetch(base_url: &str, worker_pid: Uuid) -> Result<Vec<String>, String> {
    if !crate::webhooks::url_is_permitted(base_url) {
        return Err("worker service URL is not https:// (or http:// on loopback)".to_string());
    }
    let url = format!(
        "{}/api/workers/{worker_pid}/skills",
        base_url.trim_end_matches('/')
    );
    let response = client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("worker service unreachable: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "worker service returned {}",
            response.status().as_u16()
        ));
    }
    response
        .json::<SkillsResponse>()
        .await
        .map(|body| body.skills)
        .map_err(|e| format!("worker service returned an unparseable body: {e}"))
}

/// Resolve one `worker:`/`person:` `EntityRef`'s held skill tags.
/// `base_url` is [`BASE_URL_ENV`]'s value, read by the caller so this
/// function stays testable without mutating process env. `Ok` and
/// `Err` are both TTL-cached, so an unreachable worker service is not
/// re-hit on every read within the TTL window.
///
/// # Errors
///
/// A human-readable reason whenever a skill set could not be
/// resolved: not a `worker:` reference, no `base_url` configured, the
/// worker service unreachable, a non-2xx response, or an unparseable
/// body. Never treated as "holds no skills" — the caller reports this
/// as [`crate::visibility::SkillStatus::Unknown`].
pub async fn resolve_skills(
    base_url: Option<&str>,
    person_ref: &str,
) -> Result<Vec<String>, String> {
    let Some(worker_pid) = worker_pid_of(person_ref) else {
        return Err(
            "not a worker: reference (a person: reference has no worker skill profile)".to_string(),
        );
    };
    if let Some(hit) = cached(worker_pid) {
        return hit;
    }
    let Some(base_url) = base_url else {
        return Err("worker service not configured".to_string());
    };
    let result = fetch(base_url, worker_pid).await;
    store(worker_pid, &result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_pid_of_only_matches_worker_refs() {
        let pid = Uuid::new_v4();
        assert_eq!(worker_pid_of(&format!("worker:{pid}")), Some(pid));
        assert_eq!(worker_pid_of(&format!("person:{pid}")), None);
        assert_eq!(worker_pid_of("worker:not-a-uuid"), None);
    }

    /// Serve `status`/`body` from a local ephemeral-port HTTP listener
    /// (mirrors `auth::tests::serve_keys`) and return the base URL.
    async fn serve(status: u16, body: serde_json::Value) -> String {
        let app = axum::Router::new().route(
            "/api/workers/{id}/skills",
            axum::routing::get(move || {
                let body = body.clone();
                async move {
                    (
                        axum::http::StatusCode::from_u16(status).expect("valid status"),
                        axum::Json(body),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve stub");
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn no_base_url_is_unknown_without_a_network_call() {
        let pid = Uuid::new_v4();
        let result = resolve_skills(None, &format!("worker:{pid}")).await;
        assert_eq!(result, Err("worker service not configured".to_string()));
    }

    #[tokio::test]
    async fn a_person_ref_has_no_worker_skill_profile() {
        let pid = Uuid::new_v4();
        let result = resolve_skills(Some("http://127.0.0.1:1"), &format!("person:{pid}")).await;
        assert!(result.unwrap_err().contains("not a worker"));
    }

    #[tokio::test]
    async fn a_stubbed_404_yields_an_error_naming_the_status() {
        let base = serve(404, serde_json::json!({})).await;
        let pid = Uuid::new_v4();
        let result = resolve_skills(Some(&base), &format!("worker:{pid}")).await;
        assert_eq!(result, Err("worker service returned 404".to_string()));
    }

    #[tokio::test]
    async fn a_successful_fetch_resolves_and_is_cached() {
        let base = serve(200, serde_json::json!({ "skills": ["rust", "postgres"] })).await;
        let pid = Uuid::new_v4();
        let first = resolve_skills(Some(&base), &format!("worker:{pid}"))
            .await
            .expect("resolved");
        assert_eq!(first, vec!["rust".to_string(), "postgres".to_string()]);

        // A second call with a base URL that would fail still succeeds:
        // the cached result serves it, no second network hit needed.
        let second = resolve_skills(Some("http://127.0.0.1:1"), &format!("worker:{pid}"))
            .await
            .expect("served from cache");
        assert_eq!(second, first);
    }

    #[tokio::test]
    async fn an_unreachable_service_is_also_cached() {
        let pid = Uuid::new_v4();
        let first = resolve_skills(Some("http://127.0.0.1:1"), &format!("worker:{pid}")).await;
        assert!(first.is_err());
        // A second call, even against a URL that would now succeed,
        // still returns the cached failure within the TTL.
        let base = serve(200, serde_json::json!({ "skills": ["rust"] })).await;
        let second = resolve_skills(Some(&base), &format!("worker:{pid}")).await;
        assert_eq!(second, first);
    }
}
