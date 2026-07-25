//! FHIR Bulk Data Access (Flat FHIR) — the `$export` job registry.
//!
//! ONC §170.315(g)(10) requires population-level access through the HL7
//! **Bulk Data Access** IG: an asynchronous kickoff returning `202` with a
//! `Content-Location`, a status endpoint that returns `202` while the
//! export runs and `200` with a **manifest** when it completes, NDJSON
//! output files, and a `DELETE` that cancels.
//!
//! ## What this is, precisely
//!
//! The **protocol** is implemented faithfully, so a Bulk Data client works
//! against this service unmodified. The **execution model** is not a
//! background job: the NDJSON is materialised during kickoff and held in
//! this process, bounded by [`MAX_RESOURCES`] and [`MAX_BYTES`]. That is
//! adequate for a registry of pathway templates (thousands of small
//! resources) and is stated rather than disguised:
//!
//! - Jobs do **not** survive a restart, and are not visible to another
//!   replica. A client polling through a load balancer may get a `404`.
//! - The registry holds at most [`MAX_JOBS`]; the oldest is evicted first.
//! - Jobs expire after [`JOB_TTL_SECS`].
//!
//! Moving to `bg_pg` (the family's Postgres-backed worker queue) and an
//! artifact store is the natural upgrade, and is the same path
//! [`agents/share/bulk-import-export.md`](../../../../agents/share/bulk-import-export.md)
//! already specifies for the native bulk API.
//!
//! ## Disclosure
//!
//! A bulk export is a **mass read**. The caller's access context is
//! recorded with the kickoff exactly as for a single read, so an export —
//! including one that crosses the declared residency region — appears in
//! the audit trail and in the §164.528 accounting. See
//! [`super::disclosure`].

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use uuid::Uuid;

/// Maximum resources materialised into one export.
pub const MAX_RESOURCES: usize = 10_000;

/// Maximum NDJSON payload held for one export (8 MiB). Reached first by a
/// pathway set with large `interventions` lists.
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

/// Maximum concurrent jobs retained; the oldest is evicted beyond this.
pub const MAX_JOBS: usize = 8;

/// How long a completed job stays retrievable.
pub const JOB_TTL_SECS: i64 = 900;

/// The NDJSON media type the Bulk Data IG mandates.
pub const NDJSON_CONTENT_TYPE: &str = "application/fhir+ndjson";

/// A bulk-export job's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExportStatus {
    /// Materialised and retrievable.
    Complete,
    /// Cancelled by the client via `DELETE`.
    Cancelled,
}

/// One export job.
#[derive(Debug, Clone)]
pub struct ExportJob {
    /// Job id, used in the status and file URLs.
    pub id: Uuid,
    /// Current state.
    pub status: ExportStatus,
    /// The instant the exported data was current as of (`transactionTime`).
    pub transaction_time: chrono::DateTime<chrono::Utc>,
    /// The original kickoff request URL, echoed in the manifest.
    pub request: String,
    /// Resources included.
    pub resource_count: usize,
    /// Whether the resource cap truncated the export — surfaced in the
    /// manifest's `error` array as an `OperationOutcome`, because a
    /// silently truncated population export is a data-integrity problem
    /// for whoever consumes it.
    pub truncated: bool,
    /// The NDJSON payload.
    pub ndjson: String,
}

impl ExportJob {
    /// Whether the job has outlived [`JOB_TTL_SECS`].
    #[must_use]
    pub fn is_expired(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        (now - self.transaction_time).num_seconds() > JOB_TTL_SECS
    }

    /// The Bulk Data completion **manifest**.
    ///
    /// `requiresAccessToken` reports whether this deployment actually
    /// enforces authentication, rather than asserting `true`
    /// unconditionally: with `CARE_PATHWAY_REQUIRE_AUTH` off the output
    /// files genuinely are reachable without a token, and a manifest that
    /// claimed otherwise would mislead the client about its obligations.
    #[must_use]
    pub fn manifest(&self, base: &str) -> serde_json::Value {
        let mut error = Vec::new();
        if self.truncated {
            error.push(serde_json::json!({
                "type": "OperationOutcome",
                "url": format!("{base}/$export-file/{}/error.ndjson", self.id),
            }));
        }
        serde_json::json!({
            "transactionTime": self.transaction_time.to_rfc3339(),
            "request": self.request,
            "requiresAccessToken": crate::auth::require_auth(),
            "output": [{
                "type": "PlanDefinition",
                "url": format!("{base}/$export-file/{}/PlanDefinition.ndjson", self.id),
                "count": self.resource_count,
            }],
            "error": error,
        })
    }

    /// The `error.ndjson` body: one `OperationOutcome` explaining the
    /// truncation. Empty when the export was complete.
    #[must_use]
    pub fn error_ndjson(&self) -> String {
        if !self.truncated {
            return String::new();
        }
        let outcome = serde_json::json!({
            "resourceType": "OperationOutcome",
            "issue": [{
                "severity": "warning",
                "code": "incomplete",
                "diagnostics": format!(
                    "export truncated at {MAX_RESOURCES} resources or {MAX_BYTES} bytes; \
                     the output is a partial population and must not be treated as complete",
                ),
            }],
        });
        format!("{outcome}\n")
    }
}

/// The process-wide job registry.
fn registry() -> &'static Mutex<VecDeque<ExportJob>> {
    static JOBS: OnceLock<Mutex<VecDeque<ExportJob>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(VecDeque::new()))
}

/// Serialise resources to NDJSON, stopping at [`MAX_RESOURCES`] or
/// [`MAX_BYTES`]. Returns the payload, the count written, and whether the
/// caps truncated it.
///
/// A resource that fails to serialize is skipped rather than aborting the
/// export — one malformed stored payload must not deny a population read —
/// but it still counts towards truncation, so the manifest stays honest.
#[must_use]
pub fn to_ndjson<T: Serialize>(resources: &[T]) -> (String, usize, bool) {
    let mut body = String::new();
    let mut count = 0;
    for resource in resources.iter().take(MAX_RESOURCES) {
        let Ok(line) = serde_json::to_string(resource) else {
            continue;
        };
        if body.len() + line.len() + 1 > MAX_BYTES {
            return (body, count, true);
        }
        body.push_str(&line);
        body.push('\n');
        count += 1;
    }
    (body, count, resources.len() > MAX_RESOURCES)
}

// ---------------------------------------------------------------------
// Registry operations.
//
// Each is split into a pure `*_in` core that takes the store explicitly
// and a thin wrapper that locks the process-wide registry. The split
// exists so the tests can exercise eviction, expiry, and cancellation
// against their **own** store: with a single global, a test that fills the
// registry to its cap would evict jobs another test had just registered,
// and the suite would fail intermittently depending on thread scheduling.
// ---------------------------------------------------------------------

/// Insert `job`, evicting expired entries and then the oldest, keeping the
/// store within [`MAX_JOBS`].
fn insert_in(
    jobs: &mut VecDeque<ExportJob>,
    job: ExportJob,
    now: chrono::DateTime<chrono::Utc>,
) -> Uuid {
    let id = job.id;
    jobs.retain(|j| !j.is_expired(now));
    while jobs.len() >= MAX_JOBS {
        jobs.pop_front();
    }
    jobs.push_back(job);
    id
}

/// Find a live job in `jobs`.
fn find_in(
    jobs: &VecDeque<ExportJob>,
    id: Uuid,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<ExportJob> {
    jobs.iter()
        .find(|j| j.id == id && !j.is_expired(now))
        .cloned()
}

/// Cancel a live job in `jobs`, releasing its payload.
fn cancel_in(jobs: &mut VecDeque<ExportJob>, id: Uuid, now: chrono::DateTime<chrono::Utc>) -> bool {
    let Some(job) = jobs.iter_mut().find(|j| j.id == id && !j.is_expired(now)) else {
        return false;
    };
    job.status = ExportStatus::Cancelled;
    // Release the payload immediately: a cancelled export must not keep
    // clinical data resident just because the TTL has not elapsed.
    job.ndjson = String::new();
    true
}

/// Build a completed export job.
fn completed_job(
    request: String,
    ndjson: String,
    resource_count: usize,
    truncated: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> ExportJob {
    ExportJob {
        id: Uuid::new_v4(),
        status: ExportStatus::Complete,
        transaction_time: now,
        request,
        resource_count,
        truncated,
        ndjson,
    }
}

/// Register a completed export and return its job id.
///
/// Evicts expired jobs, then the oldest, keeping the registry within
/// [`MAX_JOBS`]. A poisoned registry lock drops the job rather than
/// panicking — an export is a convenience, not a data path.
#[must_use]
pub fn register(request: String, ndjson: String, resource_count: usize, truncated: bool) -> Uuid {
    let now = chrono::Utc::now();
    let job = completed_job(request, ndjson, resource_count, truncated, now);
    let id = job.id;
    if let Ok(mut jobs) = registry().lock() {
        insert_in(&mut jobs, job, now);
    }
    id
}

/// Fetch a job by id, if it exists and has not expired.
#[must_use]
pub fn get(id: Uuid) -> Option<ExportJob> {
    let jobs = registry().lock().ok()?;
    find_in(&jobs, id, chrono::Utc::now())
}

/// Cancel a job. Returns `true` when a live job was cancelled, `false`
/// when the id is unknown or already expired.
#[must_use]
pub fn cancel(id: Uuid) -> bool {
    let Ok(mut jobs) = registry().lock() else {
        return false;
    };
    cancel_in(&mut jobs, id, chrono::Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NDJSON is one JSON document per line, newline-terminated.
    #[test]
    fn ndjson_is_one_document_per_line() {
        let resources = vec![
            serde_json::json!({ "resourceType": "PlanDefinition", "id": "a" }),
            serde_json::json!({ "resourceType": "PlanDefinition", "id": "b" }),
        ];
        let (body, count, truncated) = to_ndjson(&resources);
        assert_eq!(count, 2);
        assert!(!truncated);
        assert!(body.ends_with('\n'));
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in lines {
            let parsed: serde_json::Value = serde_json::from_str(line).expect("each line is JSON");
            assert_eq!(parsed["resourceType"], "PlanDefinition");
            assert!(!line.contains('\n'));
        }
    }

    /// An empty export is a valid, empty NDJSON body — not an error.
    #[test]
    fn empty_export_is_valid() {
        let (body, count, truncated) = to_ndjson::<serde_json::Value>(&[]);
        assert!(body.is_empty());
        assert_eq!(count, 0);
        assert!(!truncated);
    }

    /// Exceeding the byte cap truncates and reports it, rather than
    /// growing without bound.
    #[test]
    fn byte_cap_truncates_and_reports() {
        let big = serde_json::json!({ "filler": "x".repeat(200_000) });
        let resources = vec![big; 100];
        let (body, count, truncated) = to_ndjson(&resources);
        assert!(truncated, "the byte cap must report truncation");
        assert!(body.len() <= MAX_BYTES);
        assert!(count < resources.len());
    }

    /// A truncated export surfaces an `OperationOutcome` in the manifest's
    /// `error` array — a silently partial population export is worse than
    /// a loud one.
    #[test]
    fn truncated_export_declares_itself_in_the_manifest() {
        let job = ExportJob {
            id: Uuid::nil(),
            status: ExportStatus::Complete,
            transaction_time: chrono::Utc::now(),
            request: "/fhir/$export".to_string(),
            resource_count: MAX_RESOURCES,
            truncated: true,
            ndjson: String::new(),
        };
        let manifest = job.manifest("/fhir");
        assert_eq!(manifest["error"].as_array().map(Vec::len), Some(1));
        assert_eq!(manifest["error"][0]["type"], "OperationOutcome");
        let outcome: serde_json::Value =
            serde_json::from_str(job.error_ndjson().trim()).expect("valid OperationOutcome");
        assert_eq!(outcome["issue"][0]["code"], "incomplete");
    }

    /// A complete export declares no errors.
    #[test]
    fn complete_export_has_no_errors() {
        let job = ExportJob {
            id: Uuid::nil(),
            status: ExportStatus::Complete,
            transaction_time: chrono::Utc::now(),
            request: "/fhir/$export".to_string(),
            resource_count: 3,
            truncated: false,
            ndjson: String::new(),
        };
        let manifest = job.manifest("/fhir");
        assert_eq!(manifest["error"].as_array().map(Vec::len), Some(0));
        assert!(job.error_ndjson().is_empty());
        assert_eq!(manifest["output"][0]["count"], 3);
        assert_eq!(manifest["output"][0]["type"], "PlanDefinition");
    }

    /// The manifest carries the Bulk Data IG's required fields.
    #[test]
    fn manifest_has_the_required_fields() {
        let job = ExportJob {
            id: Uuid::nil(),
            status: ExportStatus::Complete,
            transaction_time: chrono::Utc::now(),
            request: "/fhir/$export?_type=PlanDefinition".to_string(),
            resource_count: 1,
            truncated: false,
            ndjson: String::new(),
        };
        let manifest = job.manifest("/fhir");
        for key in [
            "transactionTime",
            "request",
            "requiresAccessToken",
            "output",
            "error",
        ] {
            assert!(
                manifest.get(key).is_some(),
                "{key} missing from the manifest"
            );
        }
        assert_eq!(manifest["request"], "/fhir/$export?_type=PlanDefinition");
    }

    /// A private store, so registry tests cannot evict each other's jobs
    /// (see the note above `insert_in`).
    fn store() -> VecDeque<ExportJob> {
        VecDeque::new()
    }

    /// A job round-trips through the registry, and an unknown id is
    /// simply absent. Exercises the process-wide wrappers end to end.
    #[test]
    fn jobs_round_trip_through_the_registry() {
        let id = register("/fhir/$export".to_string(), "{}\n".to_string(), 1, false);
        let job = get(id).expect("registered job is retrievable");
        assert_eq!(job.resource_count, 1);
        assert_eq!(job.status, ExportStatus::Complete);
        assert!(get(Uuid::new_v4()).is_none(), "unknown id must be absent");
    }

    /// Cancelling marks the job and **releases its payload immediately** —
    /// a cancelled export must not keep clinical data resident.
    #[test]
    fn cancel_releases_the_payload() {
        let now = chrono::Utc::now();
        let mut jobs = store();
        let id = insert_in(
            &mut jobs,
            completed_job(
                "/fhir/$export".to_string(),
                "sensitive\n".to_string(),
                1,
                false,
                now,
            ),
            now,
        );
        assert!(cancel_in(&mut jobs, id, now));
        let job = find_in(&jobs, id, now).expect("cancelled job is still addressable");
        assert_eq!(job.status, ExportStatus::Cancelled);
        assert!(job.ndjson.is_empty(), "payload must be dropped on cancel");
        assert!(
            !cancel_in(&mut jobs, Uuid::new_v4(), now),
            "unknown id cannot be cancelled"
        );
    }

    /// The registry is bounded: registering beyond `MAX_JOBS` evicts the
    /// oldest rather than growing without limit.
    #[test]
    fn registry_is_bounded() {
        let now = chrono::Utc::now();
        let mut jobs = store();
        let mut ids = Vec::new();
        for _ in 0..(MAX_JOBS + 3) {
            ids.push(insert_in(
                &mut jobs,
                completed_job("/fhir/$export".to_string(), String::new(), 0, false, now),
                now,
            ));
        }
        assert!(jobs.len() <= MAX_JOBS, "registry grew past its cap");
        let live = ids
            .iter()
            .filter(|id| find_in(&jobs, **id, now).is_some())
            .count();
        assert!(live <= MAX_JOBS, "registry grew past its cap: {live}");
        assert!(
            find_in(&jobs, *ids.last().expect("at least one job"), now).is_some(),
            "the newest job must survive eviction"
        );
        assert!(
            find_in(&jobs, ids[0], now).is_none(),
            "the oldest job must be the one evicted"
        );
    }

    /// Expired jobs are swept on the next insert, so the registry does not
    /// hold clinical data past its TTL just because nobody polled.
    #[test]
    fn expired_jobs_are_swept_on_insert() {
        // One clock reading, and `stale` derived from it. Two separate
        // `Utc::now()` calls would make this flaky: `is_expired` compares
        // `num_seconds()`, which truncates, so a `JOB_TTL_SECS + 1` offset
        // taken microseconds *later* than `now` measures as exactly
        // `JOB_TTL_SECS` and the job reads as live. The wide margin below
        // keeps the boundary far from that truncation edge.
        let now = chrono::Utc::now();
        let stale = now - chrono::Duration::seconds(JOB_TTL_SECS + 60);
        let mut jobs = store();
        let old = insert_in(
            &mut jobs,
            completed_job(
                "/fhir/$export".to_string(),
                "old\n".to_string(),
                1,
                false,
                stale,
            ),
            stale,
        );
        assert_eq!(jobs.len(), 1);
        insert_in(
            &mut jobs,
            completed_job("/fhir/$export".to_string(), String::new(), 0, false, now),
            now,
        );
        assert_eq!(jobs.len(), 1, "the expired job must have been swept");
        assert!(find_in(&jobs, old, now).is_none());
    }

    /// Expiry is measured against the transaction time, and the boundary
    /// is *strictly* greater than the TTL. One clock reading throughout,
    /// so the assertions do not depend on how long the test takes (see
    /// the note in `expired_jobs_are_swept_on_insert`).
    #[test]
    fn jobs_expire_after_the_ttl() {
        let now = chrono::Utc::now();
        let at = |age_secs: i64| ExportJob {
            id: Uuid::nil(),
            status: ExportStatus::Complete,
            transaction_time: now - chrono::Duration::seconds(age_secs),
            request: String::new(),
            resource_count: 0,
            truncated: false,
            ndjson: String::new(),
        };
        assert!(!at(0).is_expired(now), "a fresh job is live");
        assert!(
            !at(JOB_TTL_SECS).is_expired(now),
            "a job exactly at the TTL is still live (the check is strictly greater)"
        );
        assert!(
            at(JOB_TTL_SECS + 1).is_expired(now),
            "one second past the TTL expires"
        );
    }
}
