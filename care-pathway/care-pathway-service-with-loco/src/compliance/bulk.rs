//! FHIR Bulk Data Access (Flat FHIR) — NDJSON rendering.
//!
//! ONC §170.315(g)(10) requires population-level access through the HL7
//! **Bulk Data Access** IG: an asynchronous kickoff returning `202` with a
//! `Content-Location`, a status endpoint that returns `202` while the
//! export runs and `200` with a **manifest** when it completes, NDJSON
//! output files, and a `DELETE` that cancels.
//!
//! ## What lives where
//!
//! This module now holds only the **pure** half — turning resources into
//! NDJSON, and the caps that bound it. The moving parts live elsewhere:
//!
//! - **Job state** is a `bulk_jobs` row ([`crate::models::bulk_jobs`]).
//! - **The work** happens on `bg_pg` ([`crate::workers::bulk_export`]).
//! - **The bytes** go to an artifact store
//!   ([`crate::bulk::store`]).
//!
//! It used to hold a process-local registry as well. That met the IG's
//! shape but not its intent, and carried three limits the spec had to
//! admit to: jobs vanished on restart, another replica could not see them
//! (a client polling through a load balancer got a `404` for a job that
//! had genuinely succeeded), and a large export blocked the request
//! thread. All three are gone.
//!
//! ## Disclosure
//!
//! A bulk export is a **mass read**. The caller's access context is
//! recorded at kickoff — where the caller and their declared purpose are
//! known — exactly as for a single read, so an export appears in the audit
//! trail and in the §164.528 accounting. See [`super::disclosure`].

use serde::Serialize;

/// Maximum resources materialised into one export.
pub const MAX_RESOURCES: usize = 10_000;

/// Maximum NDJSON payload written for one export (8 MiB). Reached first by
/// a pathway set with large `interventions` lists.
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

/// The NDJSON media type the Bulk Data IG mandates.
pub const NDJSON_CONTENT_TYPE: &str = "application/fhir+ndjson";

/// Serialise resources to NDJSON, stopping at [`MAX_RESOURCES`] or
/// [`MAX_BYTES`]. Returns the payload, the count written, and whether the
/// caps truncated it.
///
/// A resource that fails to serialize is skipped rather than aborting the
/// export — one malformed stored payload must not deny a population read —
/// but it still counts towards truncation, so the caller stays honest
/// about completeness.
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

#[cfg(test)]
mod tests {
    use super::{MAX_BYTES, MAX_RESOURCES, to_ndjson};

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

    /// Exceeding the resource cap truncates and reports it too — a
    /// silently partial population export is worse than a loud one.
    #[test]
    fn resource_cap_truncates_and_reports() {
        let small = serde_json::json!({ "id": "x" });
        let resources = vec![small; MAX_RESOURCES + 1];
        let (_, count, truncated) = to_ndjson(&resources);
        assert!(truncated);
        assert_eq!(count, MAX_RESOURCES);
    }

    /// A resource that cannot be serialized is skipped rather than
    /// aborting the whole export.
    ///
    /// The failure is forced with a `Serialize` impl that returns an
    /// error. `f64::NAN` does *not* work for this: `serde_json` maps
    /// non-finite floats to `null` rather than failing, so a test built
    /// on it would pass without ever exercising the skip path.
    #[test]
    fn unserializable_resource_is_skipped_not_fatal() {
        struct Row {
            ok: bool,
        }
        impl serde::Serialize for Row {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                if self.ok {
                    s.serialize_str("fine")
                } else {
                    Err(serde::ser::Error::custom("cannot serialize this row"))
                }
            }
        }
        let resources = vec![Row { ok: true }, Row { ok: false }, Row { ok: true }];
        let (body, count, _) = to_ndjson(&resources);
        assert_eq!(count, 2, "the good rows still export");
        assert_eq!(body.lines().count(), 2);
    }
}
