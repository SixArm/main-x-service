## 2. Scope

### 2.1 In scope — entity level

This spec owns the **cross-subproject contract**:

- Composition: front-end → service REST API → embedded matcher.
- The service ↔ matcher DTO contract (the adapter projection, §5.3).
- The service ↔ front-end wire contract (response envelope, `/api`
  base path, `{ items, total }` search shape, TypeScript type
  mirroring, §5.4).
- Shared invariants that more than one subproject must uphold (§5.5).
- Entity-wide goals: national-catalogue scale, multi-locale,
  auditability, privacy compliance (§7, §12).

### 2.2 In scope — per subproject

**course-service-with-loco** owns:

- Course CRUD with soft delete and full audit trail; the
  `CourseInstance` sub-resource (`/api/courses/{id}/instances/*`).
- Multiple identifiers per course (LMS id, course code, platform
  slug, DOI, Wikidata, ISCED, ROR, URI, UUID, custom);
  educational-credential references; syllabus sections.
- Probabilistic + deterministic matching via the embedded canonical
  matcher (adapter in `src/matching/adapter.rs`).
- Tantivy full-text / fuzzy search; blocking queries for matching.
- Real-time + explicit + batch duplicate detection, review queue,
  auto-merge, merge with transferred-data snapshots.
- Validation (FR-21..FR-28 in its spec), privacy masking, GDPR
  Article 15 export.
- REST API (loco.rs / Axum) + OpenAPI / Swagger; PostgreSQL via
  SeaORM; loco boot lifecycle, config, migrations, background queue.

**course-matcher-rust-crate** owns:

- Pure-library pairwise comparison: deterministic short-circuits
  (DOI / Wikidata / LOM / OER / URI / UUID, same-provider course
  code, `same_as` URL) + weighted probabilistic scoring with
  per-field breakdown.
- Component algorithms: name (Jaro-Winkler + Levenshtein + Soundex
  bonus), provider-scoped course code, provider, educational level,
  keywords / teaches Jaccard.
- Normalisation (case-fold, course-code shape, keyword tokenisation),
  config presets (`strict` / `default` / `lenient`).

**course-front-end-with-svelte** owns:

- Operator routes: dashboard, list/search with SVAR DataGrid, create
  with 409-duplicate surfacing, detail, edit, audit view, match
  check, merge.
- Its own copy of API types, client, and form primitives (drift
  between front-ends is accepted — repo decision 2026-06-02).

### 2.3 Out of scope (today)

- Authentication middleware in the service and sign-in in the
  front-end (roadmap §15; the SSO provider exists in the
  [authentication entity](../../authentication/)).
- Durable event bus — today the service publishes in-memory only
  (Fluvio adapter under feature flag pending).
- Syllabus-section read/write API (column ships JSONB; service
  roadmap v0.4) and instance / syllabus edit UI in the front-end.
- gRPC API (stub only), FHIR mapping (no FHIR resource fits Course),
  LMS round-trip integration (LTI / xAPI / SCORM), ML-based scoring.
- Multi-region deployment, bulk catalogue import, externalized
  search (§15).
