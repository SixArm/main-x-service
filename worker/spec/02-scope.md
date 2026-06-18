## 2. Scope

### 2.1 In scope — split across the trio

Each subproject owns a slice; this spec owns the seams.

**worker-service-with-loco** owns:

- Worker CRUD with soft delete and full audit trail.
- Persistence (PostgreSQL via SeaORM, 12+ tables, migrations).
- Full-text / fuzzy / phonetic search (Tantivy, 11 indexed fields).
- Duplicate detection: real-time on create (`409`), explicit
  check-duplicates, batch deduplicate with review queue.
- Merge (transfer + alias + `Replaces` link + soft delete + snapshot
  + `Merged` event).
- Validation / normalisation at the boundary (`422` on failure).
- Privacy: masking, GDPR export, consent model.
- Event streaming and audit logging for every mutation.
- The in-service matcher (probabilistic + deterministic) **and** the
  adapter that bridges to the canonical matcher crate.
- REST + FHIR R5 + gRPC (stub) API surfaces, OpenAPI / Swagger.

**worker-matcher-rust-crate** owns:

- The canonical pairwise matching algorithm: deterministic
  short-circuits plus weighted probabilistic scoring with per-field
  breakdown.
- 42 national personal-identifier scheme parsers (scheme-local —
  never cross-matched) and 9 passport-format validators feeding
  `PassportBook`.
- Normalisation: diacritic-correct names, postcodes, E.164 phone
  (39 jurisdictions), nickname tables.
- Configuration presets (`strict` / `default` / `lenient`) and the
  `MatchConfig` weight surface.
- Pure-library guarantees: no IO, no `unsafe`, deterministic.

**worker-front-end-with-svelte** owns:

- Operator UI routes: dashboard, list/search, create with 409
  surfacing, detail, edit, per-worker audit, match check, merge.
- Its own copy of API types, client, and form primitives (drift
  between front-ends is accepted by repo decision 2026-06-02).
- Unit (Vitest) and e2e smoke (Playwright) tests.

**This entity spec** owns:

- The composition contract: front-end → service REST API → embedded
  matcher (§5, §8, §9).
- The service↔matcher DTO contract and its pinning tests (§5).
- Shared invariants (identifier scheme-locality, soft-delete-only,
  audit-everything) and entity-wide goals (§6, §7).
- Entity-wide compliance posture (§12) and the roadmap to worldwide
  governmental scale (§15).

### 2.2 Out of scope

- Crate internals already specified by a subproject spec — link down,
  do not duplicate.
- Person, place, organization, and other entities (sibling
  directories at the repo root).
- Authentication / SSO implementation (authentication entity).
- Credential issuance and adjudication (issuing authorities).
