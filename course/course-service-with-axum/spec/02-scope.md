## 2. Scope

### 2.1 In scope

- Course identity CRUD with soft delete and audit trail.
- CourseInstance sub-resource (multiple instances per course; each
  with schedule, mode, instructors, location).
- Multiple identifiers per course (LMS id, course code, platform
  slug, DOI, Wikidata, ISCED, ROR, URI, UUID, custom).
- Educational-credential references (degree / diploma / certificate
  / micro-credential / badge / license).
- Syllabus sections (hierarchical table of contents).
- Probabilistic + deterministic matching with configurable weights.
- Tantivy-backed full-text + fuzzy + phonetic search.
- Real-time + batch duplicate detection with review queue +
  auto-merge.
- Record merging with link tracking and transferred-data snapshots.
- Data validation + normalisation at the boundary.
- Per-field privacy masking, GDPR Article 15 export.
- REST API (loco.rs controllers on Axum) at `/api/courses` and
  `/api/courses/{id}/instances`, version-negotiated via the
  `Accepts-version` header (T-25;
  [`api-versioning.md`](../../../agents/share/api-versioning.md)).
- A **non-standard FHIR surface** at `/fhir/Basic` — see §2.1b.
- Row-level integrity digests + audit-log MAC verification
  (`GET /api/records/verify`, `GET /api/audit/verify`) — see §2.1c.
- PostgreSQL persistence via SeaORM, with migrations.
- Structured tracing (`tracing`) **plus real OpenTelemetry OTLP
  export** (`src/observability.rs`, T-26) — see §2.1d.

### 2.1a Authentication / authorisation (adopted, default-off)

- Offline **PASETO v4.public** bearer verification + **ABAC** blanket
  guard on `/api/*` and `/fhir/*`, per the family contracts
  ([`authentication-sessions.md`](../../../agents/share/authentication-sessions.md),
  [`jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md),
  [`authorization-attributes.md`](../../../agents/share/authorization-attributes.md)).
  Enforcement is **off by default** (`COURSE_REQUIRE_AUTH`); activation
  is an operations decision. See §7 for the config vars and T-15. The
  verifier and the ABAC policy are hot-reloadable at runtime (key
  rotation without a restart; AU-2).

### 2.1b FHIR — deliberately non-standard (adopted)

No FHIR R5 resource models an educational course
([`fhir.md`](../../../agents/share/fhir.md) §3), so this crate wraps a
course as a FHIR `Basic` resource (`code = course`) rather than leaving
FHIR unimplemented. See §9 and T-20; this reverses the MVP scope note
this section used to carry, which predates T-20 shipping.

### 2.1c Integrity / compliance (adopted, default-off)

- Per-record SHA-256 + SHA3-256 digests and a keyed HMAC-SHA256 MAC
  over each `Course` and `audit_log` row, verified on demand via
  `GET /api/records/verify` and `GET /api/audit/verify`
  (`src/compliance/`). Default off — no `COURSE_INTEGRITY_MAC_KEY[_FILE]`
  ⇒ no MAC is written and rows report `mac_absent` rather than as
  mismatches. See §12.

### 2.1d Observability — real OpenTelemetry OTLP export (adopted)

`src/observability.rs` (T-26, PRO-H12 slice 1 of 7, landed 2026-08-30)
is a real exporter — `tracing-opentelemetry` bridged over an OTLP/gRPC
`SdkTracerProvider`/`SdkMeterProvider`, export-on-by-default at
`OTLP_ENDPOINT` (default `http://localhost:4317`) with `service.name`
from `OTLP_SERVICE_NAME` (default `course-service`). `trace_mw` is
layered as the outermost middleware on both of this crate's
router-construction surfaces. This reverses the MVP scope note §2.2
used to carry, which predates T-26 shipping — same shape as §2.1b's
FHIR reversal note above. See T-26 and
[`observability.md`](../../../agents/share/observability.md).

### 2.2 Out of scope (MVP)

- **gRPC.** No gRPC surface at all — not even a stub. There is no
  `tonic`/`prost` dependency; `GRPC_PORT` is unused legacy
  configuration (§7).
- ML-based match scoring.
- LMS round-trip integration (LTI / xAPI / SCORM).
- Course-discovery ranking.
- Enrollment / grade storage.
- Bulk import / export (T-19, designed in §9.2, not yet built).

