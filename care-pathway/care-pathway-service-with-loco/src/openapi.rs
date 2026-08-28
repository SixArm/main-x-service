//! Hand-written `OpenAPI` 3 description of the care-pathway REST API.
//!
//! The request/response `CarePathway` body is the
//! `care_pathway_matcher::CarePathway` shape. That crate is intentionally
//! dependency-light (no `utoipa`), so the schema is authored here by hand
//! rather than derived — which also keeps the doc accurate to the wire
//! format.

use serde_json::{Value, json};

/// The full `OpenAPI` document, served at `/api-docs/openapi.json`.
#[must_use]
pub fn spec() -> Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Care Pathway Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Registry of clinical care-pathway identities: CRUD + matching. The request/response body for a pathway is the care-pathway-matcher CarePathway shape. Validation failures (blank name, malformed condition_codes) return 422."
        },
        "paths": paths(),
        "components": components(),
    })
}

/// The `paths` object of the `OpenAPI` document, composed from the
/// CRUD/matching paths and the auxiliary (auth/audit/events/metrics)
/// paths.
fn paths() -> Value {
    let mut paths = crud_paths();
    merge_object(&mut paths, aux_paths());
    merge_object(&mut paths, compliance_paths());
    merge_object(&mut paths, tba_recording_paths());
    merge_object(&mut paths, tba_analysis_paths());
    merge_object(&mut paths, instance_paths());
    merge_object(&mut paths, insight_paths());
    paths
}

/// Shallow-merge the top-level keys of `src` into `dst`. Both are JSON
/// objects; this keeps the composed document byte-identical to the
/// single literal it was split from.
fn merge_object(dst: &mut Value, src: Value) {
    if let (Some(dst), Value::Object(src)) = (dst.as_object_mut(), src) {
        for (k, v) in src {
            dst.insert(k, v);
        }
    }
}

/// The CRUD + matching + merge paths.
fn crud_paths() -> Value {
    json!({
            "/api/care-pathways": {
                "get": {
                    "tags": ["care-pathways"],
                    "summary": "List active care pathways (capped at 100)",
                    "responses": { "200": { "description": "List of references",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/PathwayRef" } } } } } }
                },
                "post": {
                    "tags": ["care-pathways"],
                    "summary": "Create a care pathway",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CarePathway" } } } },
                    "responses": {
                        "200": { "description": "Created", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PathwayRef" } } } },
                        "422": { "description": "Validation failure: blank name or malformed condition_codes" }
                    }
                }
            },
            "/api/care-pathways/search": {
                "get": {
                    "tags": ["care-pathways"],
                    "summary": "Case-insensitive name search (Postgres ILIKE, cap 50)",
                    "parameters": [{ "name": "q", "in": "query", "required": true, "schema": { "type": "string" } }],
                    "responses": {
                        "200": { "description": "Matches", "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/PathwayRef" } } } } },
                        "400": { "description": "Missing or blank `q`" }
                    }
                }
            },
            "/api/care-pathways/match": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Rank a query against an explicit candidate list (no persistence)",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MatchRequest" } } } },
                    "responses": { "200": { "description": "Ranked results (index + MatchResult)" } }
                }
            },
            "/api/care-pathways/check-duplicates": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Match a query against stored pathways",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CarePathway" } } } },
                    "responses": { "200": { "description": "Scored matches",
                        "content": { "application/json": { "schema": { "type": "array", "items": { "$ref": "#/components/schemas/ScoredRef" } } } } } }
                }
            },
            "/api/care-pathways/merge": {
                "post": {
                    "tags": ["matching"],
                    "summary": "Merge a confirmed duplicate into a surviving pathway",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/MergeRequest" } } } },
                    "responses": {
                        "200": { "description": "The survivor's merged payload + the merged pids" },
                        "404": { "description": "main_pid or duplicate_pid not found" },
                        "422": { "description": "main_pid and duplicate_pid are equal" }
                    }
                }
            },
            "/api/care-pathways/merges/recent": {
                "get": { "tags": ["matching"], "summary": "Recent merge-history records", "responses": { "200": { "description": "Merge records" } } }
            }
    })
}

/// The auth / audit / events / single-record / metrics paths.
fn aux_paths() -> Value {
    json!({
            "/api/care-pathways/whoami": {
                "get": {
                    "tags": ["auth"],
                    "summary": "Echo the verified claims of the bearer token",
                    "security": [{ "bearer": [] }],
                    "responses": {
                        "200": { "description": "Verified token claims" },
                        "401": { "description": "Missing or invalid bearer token" }
                    }
                }
            },
            "/api/care-pathways/audit/recent": {
                "get": { "tags": ["audit"], "summary": "Recent audit-log entries across all pathways", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/api/care-pathways/events/recent": {
                "get": { "tags": ["audit"], "summary": "Recent events from the in-memory stream", "responses": { "200": { "description": "Events" } } }
            },
            "/api/care-pathways/{pid}": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["care-pathways"], "summary": "Fetch the stored care pathway",
                    "responses": { "200": { "description": "CarePathway", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CarePathway" } } } }, "404": { "description": "Not found" } } },
                "put": { "tags": ["care-pathways"], "summary": "Replace a care pathway's payload",
                    "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/CarePathway" } } } },
                    "responses": { "200": { "description": "Updated", "content": { "application/json": { "schema": { "$ref": "#/components/schemas/PathwayRef" } } } }, "404": { "description": "Not found" }, "422": { "description": "Validation failure: blank name or malformed condition_codes" } } },
                "delete": { "tags": ["care-pathways"], "summary": "Soft-delete a care pathway", "responses": { "200": { "description": "Deleted" } } }
            },
            "/api/care-pathways/{pid}/audit": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": { "tags": ["audit"], "summary": "Audit trail for one care pathway", "responses": { "200": { "description": "Audit entries" } } }
            },
            "/metrics.prom": {
                "get": {
                    "tags": ["observability"],
                    "summary": "Prometheus metrics (text-exposition format)",
                    "description": "Process-wide metric registry rendered as Prometheus text (Content-Type: text/plain; version=0.0.4). Mounted at the root (not under /api) and public under blanket JWT enforcement so a scraper needs no token. Configure your scraper with metrics_path: /metrics.prom.",
                    "responses": {
                        "200": {
                            "description": "Prometheus text exposition",
                            "content": { "text/plain": { "schema": { "type": "string" } } }
                        }
                    }
                }
            }
    })
}

/// The compliance-evidence paths (spec §12): posture, SBOM, audit-chain
/// verification, the HIPAA §164.528 accounting of disclosures, and the
/// GDPR Art. 17 erasure endpoint.
fn compliance_paths() -> Value {
    json!({
            "/api/compliance": {
                "get": {
                    "tags": ["compliance"],
                    "summary": "Compliance posture: software identification, controls, declarations",
                    "description": "Build provenance, the IEC 62304 safety classification and its rationale, which controls are actually live in this process, the declared data-protection posture (residency / lawful basis / Art. 9 condition / transfer safeguard, each 'undeclared' until configured), and per-framework lists of what is implemented and what is deliberately NOT claimed.",
                    "responses": { "200": { "description": "Posture report" } }
                }
            },
            "/api/compliance/sbom": {
                "get": {
                    "tags": ["compliance"],
                    "summary": "CycloneDX 1.5 software bill of materials + SOUP register",
                    "description": "Derived at compile time from the crate's own Cargo.lock, merged with the IEC 62304 §8.1.2 annotations in compliance/soup.tsv, so it cannot drift from the running binary. Deterministic: no timestamp, no serial number.",
                    "responses": { "200": { "description": "CycloneDX document" } }
                }
            },
            "/api/compliance/records/verify": {
                "get": {
                    "tags": ["compliance"],
                    "summary": "Verify row-level record integrity",
                    "description": "Recomputes each care-pathway row's content hash and names any row changed outside the service. Complements the audit-chain check: that one proves the trail was not rewritten, this proves the records were not. Soft-deleted and erased rows are included deliberately.",
                    "parameters": [{ "name": "limit", "in": "query", "required": false, "schema": { "type": "integer", "default": 1000, "maximum": 10000, "minimum": 1 } }],
                    "responses": { "200": { "description": "Record integrity report" } }
                }
            },
            "/api/compliance/audit/verify": {
                "get": {
                    "tags": ["compliance"],
                    "summary": "Verify the tamper-evident audit hash chain (HIPAA §164.312(c))",
                    "description": "Recomputes the trailing rows and reports every linkage or content break. Attests to the audit trail only, not to the care_pathways rows.",
                    "parameters": [{ "name": "limit", "in": "query", "required": false, "schema": { "type": "integer", "default": 1000, "maximum": 10000, "minimum": 1 } }],
                    "responses": { "200": { "description": "Chain verification report" } }
                }
            },
            "/api/care-pathways/{pid}/audit/disclosures": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "get": {
                    "tags": ["compliance"],
                    "summary": "Accounting of disclosures for one care pathway (HIPAA §164.528)",
                    "description": "Only audit rows classified as an outward disclosure, newest first. The response states whether the accounting is complete, or INCOMPLETE because CARE_PATHWAY_AUDIT_READS is off — an empty list must not be read as 'nothing was disclosed'.",
                    "responses": { "200": { "description": "Disclosure accounting" }, "400": { "description": "pid is not a UUID" } }
                }
            },
            "/api/care-pathways/{pid}/erase": {
                "parameters": [{ "name": "pid", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "post": {
                    "tags": ["compliance"],
                    "summary": "Erase a care pathway under GDPR Art. 17 (irreversible)",
                    "description": "Replaces the payload with a tombstone, retires the record, destroys the content of every audit row about it, and appends a chained 'erased' accountability row — the audit chain still verifies. This is NOT the reversible soft delete (DELETE /{pid}); it is a DESTRUCTIVE action under ABAC and requires access=admin under the default policy. Idempotent: re-erasing, or erasing an already-deleted pid, still sweeps any audit content held about it.",
                    "security": [{ "bearer": [] }],
                    "responses": {
                        "200": { "description": "Erasure outcome (pid, rows redacted, irreversible: true)" },
                        "400": { "description": "pid is not a UUID" },
                        "403": { "description": "Valid credential, but the policy denies a destructive action" }
                    }
                }
            }
    })
}

/// The `components` object of the `OpenAPI` document.
fn components() -> Value {
    json!({
            "securitySchemes": {
                // The credential is PASETO v4.public, not a JWT — the
                // RS256/JWKS model was decommissioned family-wide (see
                // agents/share/authentication-sessions.md). `bearerFormat`
                // is a free-text hint, so it names what is actually sent.
                "bearer": { "type": "http", "scheme": "bearer", "bearerFormat": "PASETO v4.public",
                    "description": "Short-lived PASETO v4.public (Ed25519) token minted by the authentication-service from a cookie session, verified offline against its published key set at /.well-known/paseto-keys. No shared secret and no introspection hop." }
            },
            "schemas": {
                "SegmentPayload": { "type": "object",
                    "required": ["label", "stage", "category", "started_at"],
                    "description": "A recorded interval of the patient journey (spec/time-based-analysis.md §5.1).",
                    "properties": {
                        "label": { "type": "string", "description": "Human name — \"MRI\", \"await triage outcome\"" },
                        "stage": { "type": "string", "enum": ["referral", "triage", "diagnostics", "treatment", "follow_up", "discharge", "other"] },
                        "category": { "type": "string", "enum": ["value_adding", "necessary_non_value_adding", "unnecessary_non_value_adding"],
                            "description": "The value-stream-mapping classification: VA / NNVA / UNVA." },
                        "waste": { "type": "string", "nullable": true,
                            "enum": ["waiting", "transportation", "motion", "over_processing", "defects", "inventory", "overproduction", "underutilised_people"],
                            "description": "Refused on a value_adding segment; required on an unnecessary_non_value_adding one." },
                        "started_at": { "type": "string", "format": "date-time" },
                        "ended_at": { "type": "string", "format": "date-time", "nullable": true,
                            "description": "Omit to open a running segment; only one may be open per instance." },
                        "actor_ref": { "type": "string", "nullable": true, "description": "worker: / organization: URN — who" },
                        "location_ref": { "type": "string", "nullable": true, "description": "place: / organization: URN — where" },
                        "note": { "type": "string", "nullable": true } } },
                "PathwayRef": { "type": "object", "required": ["pid", "name"], "properties": {
                    "pid": { "type": "string", "format": "uuid" }, "name": { "type": "string" } } },
                "ScoredRef": { "type": "object", "properties": {
                    "pid": { "type": "string" }, "name": { "type": "string" },
                    "score": { "type": "number", "format": "double" }, "confidence": { "type": "string" },
                    "is_match": { "type": "boolean" } } },
                "MatchRequest": { "type": "object", "required": ["query", "candidates"], "properties": {
                    "query": { "$ref": "#/components/schemas/CarePathway" },
                    "candidates": { "type": "array", "items": { "$ref": "#/components/schemas/CarePathway" } } } },
                "MergeRequest": { "type": "object", "required": ["main_pid", "duplicate_pid"], "properties": {
                    "main_pid": { "type": "string", "format": "uuid" },
                    "duplicate_pid": { "type": "string", "format": "uuid" },
                    "reason": { "type": "string", "nullable": true } } },
                "ConditionCode": { "type": "object", "required": ["system", "code"], "properties": {
                    "system": { "description": "Icd10 | Icd11 | Snomed | {Custom: string}" },
                    "code": { "type": "string", "description": "Format-validated by system: ICD-10 (e.g. I63.9), ICD-11 stem (e.g. 1A00), SNOMED CT SCTID (6-18 digits, Verhoeff check digit); Custom must be non-blank." } } },
                "PathwayIdentifier": { "type": "object", "required": ["scheme", "value"], "properties": {
                    "scheme": { "description": "Doi | Wikidata | GuidelineId | Uri | Uuid | {Custom: string}" },
                    "value": { "type": "string" } } },
                "CarePathway": { "type": "object", "required": ["name"], "properties": {
                    "name": { "type": "string" },
                    "alternate_names": { "type": "array", "items": { "type": "string" } },
                    "pathway_code": { "type": "string", "nullable": true, "description": "Provider-scoped local code" },
                    "provider_id": { "type": "string", "nullable": true },
                    "provider_name": { "type": "string", "nullable": true },
                    "care_setting": { "type": "string", "nullable": true, "description": "Inpatient | Outpatient | Emergency | Community | …" },
                    "condition_codes": { "type": "array", "items": { "$ref": "#/components/schemas/ConditionCode" } },
                    "interventions": { "type": "array", "items": { "type": "string" } },
                    "keywords": { "type": "array", "items": { "type": "string" } },
                    "identifiers": { "type": "array", "items": { "$ref": "#/components/schemas/PathwayIdentifier" } },
                    "same_as": { "type": "array", "items": { "type": "string" } },
                    "in_language": { "type": "array", "items": { "type": "string" }, "description": "BCP-47 language codes" } } }
            }
    })
}

/// The instance-pid path parameter, shared by every per-instance
/// time-based-analysis path.
fn instance_pid_param() -> Value {
    json!({
        "name": "pid", "in": "path", "required": true,
        "schema": { "type": "string", "format": "uuid" }
    })
}

/// The time-based-analysis **recording** paths
/// (`spec/time-based-analysis.md` §10.1).
fn tba_recording_paths() -> Value {
    let instance_pid = instance_pid_param();
    json!({
        "/api/instances/{pid}/segments": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "List a journey's recorded segments, in time order",
                "parameters": [instance_pid],
                "responses": { "200": { "description": "Segments" }, "404": { "description": "Unknown instance" } }
            },
            "post": {
                "tags": ["time-based-analysis"],
                "summary": "Record a segment of the patient journey",
                "description": "A bounded interval classified value_adding / necessary_non_value_adding / unnecessary_non_value_adding. Omitting ended_at opens a running segment; only one may be open at a time. A waste type is refused on a value-adding segment and required on an unnecessary one.",
                "parameters": [instance_pid],
                "requestBody": { "required": true, "content": { "application/json": { "schema": { "$ref": "#/components/schemas/SegmentPayload" } } } },
                "responses": {
                    "200": { "description": "Recorded" },
                    "404": { "description": "Unknown instance" },
                    "422": { "description": "Blank label, unknown stage/category/waste, reversed interval, or a second open segment" }
                }
            }
        },
        "/api/instances/{pid}/segments/{seg}/close": {
            "post": {
                "tags": ["time-based-analysis"],
                "summary": "Close a running segment",
                "parameters": [
                    instance_pid,
                    { "name": "seg", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }
                ],
                "responses": {
                    "200": { "description": "Closed" },
                    "404": { "description": "Unknown instance or segment" },
                    "422": { "description": "Already closed, or the end precedes the start" }
                }
            }
        },
        "/api/instances/{pid}/clock": {
            "post": {
                "tags": ["time-based-analysis"],
                "summary": "Set the pathway clock start or stop",
                "description": "Events are `start` and `stop`. There is deliberately no `pause`: patient-caused delay is recorded as an unnecessary_non_value_adding segment so it stays visible, rather than silently shrinking the denominator every ratio is measured against.",
                "parameters": [instance_pid],
                "requestBody": { "required": true, "content": { "application/json": { "schema": {
                    "type": "object", "required": ["event"],
                    "properties": {
                        "event": { "type": "string", "enum": ["start", "stop"] },
                        "at": { "type": "string", "format": "date-time", "description": "Defaults to now" }
                    }
                } } } },
                "responses": {
                    "200": { "description": "Clock set" },
                    "404": { "description": "Unknown instance" },
                    "422": { "description": "Unknown event, or a stop at or before the start" }
                }
            }
        },
    })
}

/// The time-based-analysis **read** paths
/// (`spec/time-based-analysis.md` §10.2). Every figure is derived on
/// read; nothing here is stored.
fn tba_analysis_paths() -> Value {
    let instance_pid = instance_pid_param();
    json!({
        "/api/instances/{pid}/time-analysis": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Per-instance time-based analysis",
                "description": "Lead time, value time, process time, touch time, the value-adding ratio, and coverage. The denominator is elapsed calendar time, never the sum of recorded activity, so unrecorded time counts as non-value-adding; coverage_ratio and confidence say how much of the journey was mapped at all.",
                "parameters": [instance_pid],
                "responses": { "200": { "description": "Analysis" }, "404": { "description": "Unknown instance" } }
            }
        },
        "/api/instances/{pid}/timeline": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "The mapped journey as an ordered wall of segments and gaps",
                "parameters": [instance_pid],
                "responses": { "200": { "description": "Timeline" }, "404": { "description": "Unknown instance" } }
            }
        },
        "/api/instances/flow": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Queueing-theory flow analysis (Little's Law)",
                "description": "Arrival rate, service rate, utilisation, work in progress, and the lead time Little's Law implies for a journey entering now, compared against the observed median. Used as a consistency check, not a forecast.",
                "parameters": [
                    { "name": "window_days", "in": "query", "schema": { "type": "integer", "default": 90, "minimum": 1, "maximum": 3650 } },
                    { "name": "pathway", "in": "query", "schema": { "type": "string", "format": "uuid" } }
                ],
                "responses": { "200": { "description": "Flow" }, "422": { "description": "Window out of range" } }
            }
        },
        "/api/instances/time-standards": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "The access-standard catalogue and segment vocabularies",
                "description": "NHS access standards with thresholds, operational targets, authority and citation date; plus the closed stage / category / waste vocabularies. Reference data, not an assertion that a pathway is subject to any of them.",
                "responses": { "200": { "description": "Standards" } }
            }
        },
        "/api/care-pathways/{pathway}/time-analysis": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Cohort time-based analysis for one pathway",
                "description": "Nearest-rank lead-time percentiles, aggregate and median value-adding ratio, and compliance against a named standard. A cohort smaller than five withholds percentile detail, which would otherwise identify an individual journey.",
                "parameters": [
                    { "name": "pathway", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } },
                    { "name": "standard", "in": "query", "schema": { "type": "string", "example": "rtt_18_weeks" } },
                    { "name": "target_days", "in": "query", "schema": { "type": "number" } },
                    { "name": "status", "in": "query", "schema": { "type": "string", "enum": ["open", "closed", "all"] } }
                ],
                "responses": {
                    "200": { "description": "Cohort analysis" },
                    "404": { "description": "Unknown pathway" },
                    "422": { "description": "Unknown standard, or a non-positive target_days" }
                }
            }
        },
        "/api/care-pathways/{pathway}/constraints": {
            "get": {
                "tags": ["time-based-analysis"],
                "summary": "Ranked constraints: where the cohort's time goes",
                "description": "Disclosed-rule findings ordered by recoverable time. Deliberately not a composite score, and deliberately never per-clinician.",
                "parameters": [
                    { "name": "pathway", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } },
                    { "name": "status", "in": "query", "schema": { "type": "string", "enum": ["open", "closed", "all"] } }
                ],
                "responses": { "200": { "description": "Findings" }, "404": { "description": "Unknown pathway" } }
            }
        }
    })
}

/// The **operational instance layer** — a patient enrolled on a
/// pathway template, its status/urgency lifecycle, care team, step
/// completion, outcomes, and the derived caseload/overdue/cohort views
/// (`src/controllers/instances.rs`). Was the pre-existing `OpenAPI` gap
/// (`spec/time-based-analysis.md` §17); closed by documenting these
/// alongside the TBA surface rather than by undocumenting that one.
/// Split across three functions purely to stay under the crate's
/// `too_many_lines` lint; together they are one logical path set.
fn instance_paths() -> Value {
    let mut paths = instance_pathway_paths();
    merge_object(&mut paths, instance_action_paths());
    merge_object(&mut paths, instance_record_and_view_paths());
    paths
}

/// The pathway-scoped instance paths: enrolment, the chronic cohort, and
/// outcome analytics.
fn instance_pathway_paths() -> Value {
    json!({
        "/api/care-pathways/{pathway}/instances": {
            "parameters": [{ "name": "pathway", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
            "post": {
                "tags": ["instances"],
                "summary": "Enrol a subject on a pathway template",
                "description": "Copies the template's declared steps into the new instance's checklist and starts its time-based-analysis clock. subject_ref must be a person:<uuid> URN.",
                "requestBody": { "required": true, "content": { "application/json": { "schema": {
                    "type": "object", "required": ["subject_ref"],
                    "properties": {
                        "subject_ref": { "type": "string", "description": "person:<uuid> URN" },
                        "urgency": { "type": "string", "enum": ["routine", "urgent", "emergency"], "default": "routine" },
                        "next_review_on": { "type": "string", "format": "date", "nullable": true },
                        "steps": { "type": "array", "items": { "type": "string" }, "description": "Optional initial step labels, in order" }
                    }
                } } } },
                "responses": {
                    "200": { "description": "The enrolled instance" },
                    "404": { "description": "Unknown pathway" },
                    "422": { "description": "subject_ref is not a person:<uuid> URN, or urgency is unknown" }
                }
            },
            "get": {
                "tags": ["instances"],
                "summary": "This template's live instances, newest first",
                "responses": { "200": { "description": "Instances" }, "404": { "description": "Unknown pathway" } }
            }
        },
        "/api/care-pathways/{pathway}/cohort": {
            "get": {
                "tags": ["instances"],
                "summary": "The chronic cohort on one pathway: counts by status/urgency + step completion",
                "parameters": [{ "name": "pathway", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "responses": { "200": { "description": "Cohort summary" }, "404": { "description": "Unknown pathway" } }
            }
        },
        "/api/care-pathways/{pathway}/outcomes": {
            "get": {
                "tags": ["instances"],
                "summary": "Outcome analytics for the pathway's closed instances",
                "description": "The recorded-outcome distribution (declared outcomes only; unrecorded counted separately) plus, per recorded measure name, the count and latest-value-per-instance average.",
                "parameters": [{ "name": "pathway", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }],
                "responses": { "200": { "description": "Outcome analytics" }, "404": { "description": "Unknown pathway" } }
            }
        }
    })
}

/// The `{pid}`-scoped instance-action paths: fetch, the lifecycle move,
/// review, and urgency.
fn instance_action_paths() -> Value {
    let instance_pid = instance_pid_param();
    json!({
        "/api/instances/{pid}": {
            "get": {
                "tags": ["instances"],
                "summary": "One instance with its steps, care team, events, and measures",
                "parameters": [instance_pid.clone()],
                "responses": { "200": { "description": "Instance detail" }, "404": { "description": "Unknown instance" } }
            }
        },
        "/api/instances/{pid}/status": {
            "post": {
                "tags": ["instances"],
                "summary": "The enrolment lifecycle move (active <-> on_hold -> a terminal status)",
                "description": "Closing (to completed or discontinued) stamps closed_on, stops the time-based-analysis clock, and accepts an optional declared outcome.",
                "parameters": [instance_pid.clone()],
                "requestBody": { "required": true, "content": { "application/json": { "schema": {
                    "type": "object", "required": ["to"],
                    "properties": {
                        "to": { "type": "string", "enum": ["active", "on_hold", "completed", "discontinued"] },
                        "reason": { "type": "string", "nullable": true },
                        "outcome": { "type": "string", "nullable": true,
                            "enum": ["improved", "stable", "deteriorated", "deceased", "transferred", "not_achieved", "other"],
                            "description": "Only accepted when `to` is a terminal status" }
                    }
                } } } },
                "responses": {
                    "200": { "description": "Updated instance" },
                    "404": { "description": "Unknown instance" },
                    "422": { "description": "Not a permitted transition, unknown outcome, or an outcome on a non-terminal move" }
                }
            }
        },
        "/api/instances/{pid}/review": {
            "post": {
                "tags": ["instances"],
                "summary": "Record a review and reschedule the next one (the chronic-management cadence)",
                "parameters": [instance_pid.clone()],
                "requestBody": { "required": true, "content": { "application/json": { "schema": {
                    "type": "object", "required": ["next_review_on"],
                    "properties": {
                        "next_review_on": { "type": "string", "format": "date" },
                        "note": { "type": "string", "nullable": true }
                    }
                } } } },
                "responses": {
                    "200": { "description": "Updated instance" },
                    "404": { "description": "Unknown instance" },
                    "422": { "description": "A closed instance is not reviewed" }
                }
            }
        },
        "/api/instances/{pid}/urgency": {
            "post": {
                "tags": ["instances"],
                "summary": "Change urgency; records an escalation/de_escalation event by direction",
                "parameters": [instance_pid.clone()],
                "requestBody": { "required": true, "content": { "application/json": { "schema": {
                    "type": "object", "required": ["to"],
                    "properties": {
                        "to": { "type": "string", "enum": ["routine", "urgent", "emergency"] },
                        "note": { "type": "string", "nullable": true }
                    }
                } } } },
                "responses": { "200": { "description": "Updated instance" }, "404": { "description": "Unknown instance" }, "422": { "description": "Unknown urgency level" } }
            }
        }
    })
}

/// The `{pid}`-scoped instance record paths (team, events, measures,
/// step completion) plus the derived, unparameterised views (caseload,
/// overdue reviews, care-team load).
fn instance_record_and_view_paths() -> Value {
    let instance_pid = instance_pid_param();
    json!({
        "/api/instances/{pid}/team": {
            "post": {
                "tags": ["instances"],
                "summary": "Add a care-team member",
                "parameters": [instance_pid.clone()],
                "requestBody": { "required": true, "content": { "application/json": { "schema": {
                    "type": "object", "required": ["member_ref", "role"],
                    "properties": {
                        "member_ref": { "type": "string", "description": "worker: / person: / organization: URN" },
                        "role": { "type": "string", "enum": ["lead_clinician", "gp", "specialist", "nurse", "mental_health", "coordinator", "other"] }
                    }
                } } } },
                "responses": {
                    "200": { "description": "The added team-member row" },
                    "404": { "description": "Unknown instance" },
                    "422": { "description": "Invalid member_ref/role, or that (member, role) pair already holds the team" }
                }
            }
        },
        "/api/instances/{pid}/events": {
            "post": {
                "tags": ["instances"],
                "summary": "Record a free event (note, referral, ...); reviews and escalations have their own endpoints",
                "parameters": [instance_pid.clone()],
                "requestBody": { "required": true, "content": { "application/json": { "schema": {
                    "type": "object", "required": ["kind"],
                    "properties": {
                        "kind": { "type": "string", "enum": ["note", "review", "escalation", "de_escalation", "referral"] },
                        "note": { "type": "string", "nullable": true }
                    }
                } } } },
                "responses": { "200": { "description": "The recorded event" }, "404": { "description": "Unknown instance" }, "422": { "description": "Unknown event kind" } }
            }
        },
        "/api/instances/{pid}/measures": {
            "post": {
                "tags": ["instances"],
                "summary": "Record a clinical / PROM measure",
                "parameters": [instance_pid.clone()],
                "requestBody": { "required": true, "content": { "application/json": { "schema": {
                    "type": "object", "required": ["name"],
                    "properties": {
                        "name": { "type": "string" },
                        "value_numeric": { "type": "number", "nullable": true },
                        "value_text": { "type": "string", "nullable": true },
                        "unit": { "type": "string", "nullable": true },
                        "recorded_on": { "type": "string", "format": "date", "nullable": true, "description": "Defaults to today" }
                    }
                } } } },
                "responses": {
                    "200": { "description": "The recorded measure" },
                    "404": { "description": "Unknown instance" },
                    "422": { "description": "Blank name, or neither value_numeric nor value_text given" }
                }
            }
        },
        "/api/instances/{pid}/steps/{step}/complete": {
            "post": {
                "tags": ["instances"],
                "summary": "Mark a checklist step done (stamps done_on; idempotent)",
                "parameters": [
                    instance_pid.clone(),
                    { "name": "step", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }
                ],
                "responses": { "200": { "description": "The step" }, "404": { "description": "Unknown instance or step" } }
            }
        },
        "/api/instances/caseload": {
            "get": {
                "tags": ["instances"],
                "summary": "Open instances (active + on_hold) by care setting and urgency",
                "responses": { "200": { "description": "Caseload summary" } }
            }
        },
        "/api/instances/overdue-reviews": {
            "get": {
                "tags": ["instances"],
                "summary": "Open instances whose next_review_on has passed (or is unset) — the chronic review register",
                "responses": { "200": { "description": "Overdue reviews, most overdue first" } }
            }
        },
        "/api/instances/care-team-load": {
            "get": {
                "tags": ["instances"],
                "summary": "Open-instance count per care-team member, with their roles",
                "responses": { "200": { "description": "Per-member load" } }
            }
        },
    })
}

/// The registry-insight lenses (`src/controllers/insights.rs`): derived,
/// read-only facets over the stored `CarePathway` templates. No
/// migration, no matcher change — facets come from existing DTO fields
/// plus disclosed keyword conventions (`specialty:<x>`,
/// `jurisdiction:<x>`).
fn insight_paths() -> Value {
    json!({
        "/api/care-pathways/insights/directory": {
            "get": {
                "tags": ["insights"],
                "summary": "Pathways faceted by care setting, each with specialty:<x> facet counts",
                "responses": { "200": { "description": "Directory" } }
            }
        },
        "/api/care-pathways/insights/coverage": {
            "get": {
                "tags": ["insights"],
                "summary": "Per condition code, which settings have a pathway; disclosed gap findings",
                "description": "Names conditions with no primary-care pathway and no emergency pathway.",
                "responses": { "200": { "description": "Coverage" } }
            }
        },
        "/api/care-pathways/insights/variants": {
            "get": {
                "tags": ["insights"],
                "summary": "Conditions offered by 2+ providers, with the jurisdiction:<x> facet",
                "description": "A comparison directory. Never a matching signal.",
                "responses": { "200": { "description": "Variants" } }
            }
        },
        "/api/care-pathways/insights/providers": {
            "get": {
                "tags": ["insights"],
                "summary": "Pathways per issuing provider, counted by setting",
                "responses": { "200": { "description": "Providers" } }
            }
        },
        "/api/care-pathways/insights/languages": {
            "get": {
                "tags": ["insights"],
                "summary": "Pathways per BCP-47 language, plus the single-language-condition equity lens",
                "responses": { "200": { "description": "Languages" } }
            }
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_is_wellformed() {
        let s = spec();
        assert_eq!(s["openapi"], "3.0.3");
        assert!(s["paths"]["/api/care-pathways"]["post"].is_object());
        assert!(s["paths"]["/api/care-pathways/check-duplicates"]["post"].is_object());
        assert!(s["components"]["schemas"]["CarePathway"]["properties"]["name"].is_object());
        assert!(s["components"]["schemas"]["ConditionCode"]["properties"]["code"].is_object());
    }

    #[test]
    fn spec_documents_core_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        // The seven core CRUD + matching operations.
        assert!(paths["/api/care-pathways"]["get"].is_object());
        assert!(paths["/api/care-pathways"]["post"].is_object());
        assert!(paths["/api/care-pathways/match"]["post"].is_object());
        assert!(paths["/api/care-pathways/check-duplicates"]["post"].is_object());
        assert!(paths["/api/care-pathways/{pid}"]["get"].is_object());
        assert!(paths["/api/care-pathways/{pid}"]["put"].is_object());
        assert!(paths["/api/care-pathways/{pid}"]["delete"].is_object());
    }

    #[test]
    fn spec_documents_audit_and_event_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/care-pathways/audit/recent"]["get"].is_object());
        assert!(paths["/api/care-pathways/events/recent"]["get"].is_object());
        assert!(paths["/api/care-pathways/{pid}/audit"]["get"].is_object());
    }

    /// The compliance-evidence endpoints (spec §12) are documented, so
    /// an auditor can find them from the API doc rather than the source.
    #[test]
    fn spec_documents_compliance_endpoints() {
        let s = spec();
        let paths = &s["paths"];
        assert!(paths["/api/compliance"]["get"].is_object());
        assert!(paths["/api/compliance/sbom"]["get"].is_object());
        assert!(paths["/api/compliance/audit/verify"]["get"].is_object());
        assert!(paths["/api/care-pathways/{pid}/audit/disclosures"]["get"].is_object());
        assert!(paths["/api/care-pathways/{pid}/erase"]["post"].is_object());
    }

    /// The erasure endpoint's documentation must say it is irreversible
    /// and distinct from the soft delete — the single most consequential
    /// thing a caller could misread about this API.
    #[test]
    fn spec_documents_the_time_based_analysis_surface() {
        let s = spec();
        let paths = &s["paths"];
        for path in [
            "/api/instances/{pid}/segments",
            "/api/instances/{pid}/segments/{seg}/close",
            "/api/instances/{pid}/clock",
            "/api/instances/{pid}/time-analysis",
            "/api/instances/{pid}/timeline",
            "/api/instances/flow",
            "/api/instances/time-standards",
            "/api/care-pathways/{pathway}/time-analysis",
            "/api/care-pathways/{pathway}/constraints",
        ] {
            assert!(paths[path].is_object(), "{path} is undocumented");
        }
        assert!(paths["/api/instances/{pid}/segments"]["post"].is_object());
        assert!(paths["/api/instances/{pid}/segments"]["get"].is_object());
        // The segment payload schema is where the closed vocabularies
        // are actually published to a client.
        let segment = &s["components"]["schemas"]["SegmentPayload"];
        assert!(segment["properties"]["category"]["enum"].is_array());
        assert!(segment["properties"]["waste"]["enum"].is_array());
        assert!(segment["properties"]["stage"]["enum"].is_array());
    }

    /// The instance layer and the registry-insight lenses were a
    /// pre-existing, spec-acknowledged `OpenAPI` gap
    /// (`spec/time-based-analysis.md` §17) — the TBA surface was
    /// documented and these were not. This pins that they now are.
    #[test]
    fn spec_documents_the_instance_and_insight_surface() {
        let s = spec();
        let paths = &s["paths"];
        for path in [
            "/api/care-pathways/{pathway}/instances",
            "/api/care-pathways/{pathway}/cohort",
            "/api/care-pathways/{pathway}/outcomes",
            "/api/instances/{pid}",
            "/api/instances/{pid}/status",
            "/api/instances/{pid}/review",
            "/api/instances/{pid}/urgency",
            "/api/instances/{pid}/team",
            "/api/instances/{pid}/events",
            "/api/instances/{pid}/measures",
            "/api/instances/{pid}/steps/{step}/complete",
            "/api/instances/caseload",
            "/api/instances/overdue-reviews",
            "/api/instances/care-team-load",
            "/api/care-pathways/insights/directory",
            "/api/care-pathways/insights/coverage",
            "/api/care-pathways/insights/variants",
            "/api/care-pathways/insights/providers",
            "/api/care-pathways/insights/languages",
        ] {
            assert!(paths[path].is_object(), "{path} is undocumented");
        }
        assert!(paths["/api/care-pathways/{pathway}/instances"]["post"].is_object());
        assert!(paths["/api/care-pathways/{pathway}/instances"]["get"].is_object());
        // The enrolment payload's URN convention and closed vocabularies
        // are where a client actually learns the wire contract.
        let enroll = &paths["/api/care-pathways/{pathway}/instances"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"];
        assert_eq!(enroll["properties"]["subject_ref"]["type"], "string");
        assert!(enroll["properties"]["urgency"]["enum"].is_array());
    }

    #[test]
    fn the_clock_endpoint_documents_that_there_is_no_pause() {
        // The absence of a pause is the anti-gaming property
        // (spec/time-based-analysis.md §12.3), so it is documented
        // rather than merely omitted — a client that looked for one and
        // found nothing would assume an oversight.
        let s = spec();
        let op = &s["paths"]["/api/instances/{pid}/clock"]["post"];
        let description = op["description"].as_str().unwrap_or_default();
        assert!(
            description.contains("no `pause`"),
            "the clock endpoint must say why there is no pause: {description}"
        );
        let events = &op["requestBody"]["content"]["application/json"]["schema"]["properties"]["event"]
            ["enum"];
        assert_eq!(events, &json!(["start", "stop"]));
    }

    #[test]
    fn erase_endpoint_is_documented_as_irreversible() {
        let s = spec();
        let op = &s["paths"]["/api/care-pathways/{pid}/erase"]["post"];
        let summary = op["summary"].as_str().unwrap_or_default();
        let description = op["description"].as_str().unwrap_or_default();
        assert!(summary.to_lowercase().contains("irreversible"), "{summary}");
        assert!(description.contains("NOT the reversible soft delete"));
        assert!(description.contains("DESTRUCTIVE"));
    }

    /// The security scheme names the credential this family actually
    /// uses. The RS256/JWKS model was decommissioned; a doc that still
    /// advertised it would send an integrator down the wrong path.
    #[test]
    fn security_scheme_describes_paseto_not_jwt() {
        let scheme = &spec()["components"]["securitySchemes"]["bearer"];
        assert_eq!(scheme["bearerFormat"], "PASETO v4.public");
        let description = scheme["description"].as_str().unwrap_or_default();
        assert!(description.contains("PASETO"));
        assert!(!description.contains("RS256"));
        assert!(!description.contains("JWKS"));
    }

    #[test]
    fn spec_documents_search_endpoint() {
        let s = spec();
        let op = &s["paths"]["/api/care-pathways/search"]["get"];
        assert!(op.is_object());
        assert_eq!(op["parameters"][0]["name"], "q");
    }

    #[test]
    fn spec_documents_merge_endpoints() {
        let s = spec();
        assert!(s["paths"]["/api/care-pathways/merge"]["post"].is_object());
        assert!(s["paths"]["/api/care-pathways/merges/recent"]["get"].is_object());
        assert!(s["components"]["schemas"]["MergeRequest"]["properties"]["main_pid"].is_object());
    }

    #[test]
    fn spec_documents_metrics_endpoint() {
        let s = spec();
        let op = &s["paths"]["/metrics.prom"]["get"];
        assert!(op.is_object());
        assert!(op["responses"]["200"]["content"]["text/plain"].is_object());
    }

    #[test]
    fn spec_documents_whoami_with_bearer_security() {
        let s = spec();
        assert!(s["paths"]["/api/care-pathways/whoami"]["get"]["security"][0]["bearer"].is_array());
        assert_eq!(
            s["components"]["securitySchemes"]["bearer"]["scheme"],
            "bearer"
        );
    }
}
