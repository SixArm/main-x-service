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
- REST API (Axum) at `/api/courses` and `/api/courses/{id}/instances`.
- PostgreSQL persistence via SeaORM, with migrations.
- Observability (tracing + OpenTelemetry OTLP).

### 2.2 Out of scope (MVP)

- Authentication / authorisation middleware (planned — §15).
- gRPC API (stub only).
- FHIR resource mapping (no FHIR resource fits Course cleanly).
- ML-based match scoring.
- LMS round-trip integration (LTI / xAPI / SCORM).
- Course-discovery ranking.
- Enrollment / grade storage.

