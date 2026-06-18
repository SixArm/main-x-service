## 2. Scope

This entity-level spec owns the **cross-subproject contract**; each
subproject's spec owns its internals (see the authority banner in
[`index.md`](index.md)).

### 2.1 In scope — entity level

- The composition contract: front-end → service REST API (`/api/v1`)
  → embedded matcher library.
- The service ↔ matcher DTO contract
  (`adapter::to_matcher_event`, §5.3).
- Shared invariants that more than one subproject must agree on
  (time-window rules, identifier uniqueness, soft-delete-only).
- Entity-wide goals: population-scale registry, multi-locale,
  auditability, privacy compliance (§7, §12, §15).

### 2.2 In scope — per subproject

**event-service-with-loco** owns:

- Event identity CRUD with soft delete and full audit trail.
- The persisted domain model (schema.org/Event-aligned: time window,
  `Location` union, parties, offers, identifiers, hierarchy).
- Tantivy full-text + fuzzy search with date-range filter.
- Real-time + batch duplicate detection, review queue, merging.
- In-service matching *and* the bridge to the embedded matcher.
- Per-field privacy masking, GDPR Article 15 export, consent records.
- REST API (15 endpoints under `/api/v1`), OpenAPI/Swagger, FHIR
  stub, gRPC stub.
- PostgreSQL persistence via SeaORM; audit log; event streaming.

**event-matcher-rust-crate** owns:

- Pure, deterministic, IO-free pairwise comparison of two `Event`
  records (probabilistic score + per-field breakdown, deterministic
  bool).
- Text normalisation, string similarity, Gaussian temporal decay,
  Haversine geo scoring, Soundex phonetics.
- `EventIdScheme` ticketing-system identifiers and the
  deterministic-match rule.
- Its own SemVer public-API contract.

**event-front-end-with-svelte** owns:

- The operator UI: list/search, create with 409-duplicate surfacing,
  detail/edit/soft-delete, audit view, match check, merge.
- Its copies of API types, client, and form primitives (drift between
  front-ends is accepted by repo decision 2026-06-02).
- Front-end build, routes, design-system integration, e2e tests.

### 2.3 Out of scope (today)

- FHIR R5 surface — stubbed `501` in the service until the Event →
  Encounter / Appointment mapping is fixed (service spec OQ-1).
- Recurrence (RFC 5545 RRULE) — roadmap (§15).
- Authentication / authorisation — roadmap; the front-end ships no
  auth until the service does (§15, ET-5).
- Durable event bus — the service publishes index-level events
  in-memory only (§15).
- Consent-management, masked-view, and GDPR-export UI in the
  front-end (front-end spec §13 T-19 / T-20).
