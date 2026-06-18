## 2. Scope

### 2.1 In scope — and who owns what

This spec owns the **integration contract**; each subproject owns its
internals.

**thing-service-with-loco** (system of record) owns:

- Thing identity CRUD with soft delete and full audit trail.
- schema.org/Thing canonical properties + `PropertyValue` identifiers.
- Tantivy full-text / fuzzy / boolean search.
- Real-time + batch duplicate detection, review queue, merge with
  link tracking and JSON snapshots.
- Per-field privacy masking, GDPR Article 15 export, consent records.
- Event publishing on every CRUD / merge / link.
- REST API (loco.rs / Axum) + OpenAPI / Swagger; gRPC stub.
- PostgreSQL persistence via SeaORM.

**thing-matcher-rust-crate** (canonical algorithm) owns:

- Pairwise `Thing` ↔ `Thing` comparison: deterministic match plus a
  renormalised weighted probabilistic score over ten schema.org
  components, with per-field `MatchBreakdown`.
- Normalisation rules (text, URL, phonetic).
- Config presets (`strict` / `default` / `lenient`) and tuning knobs.
- Pure-library guarantees: no IO, no `unsafe`, deterministic,
  `Send + Sync`.

**thing-front-end-with-svelte** (operator UI) owns:

- SvelteKit SPA routes for CRUD, search, match, merge, audit.
- Its own copy of API types / client / form primitives (drift between
  front-ends is accepted by project decision 2026-06-02).
- Vitest unit tests and Playwright e2e smoke tests.

**This entity spec** owns:

- The service ↔ matcher DTO contract (adapter mapping, §5.3).
- The front-end ↔ service REST contract summary (§9).
- Shared invariants (identifier semantics, soft-delete-only,
  audit-everything) and entity-wide goals (§1, §7, §15).

### 2.2 Out of scope (today)

- FHIR — Things are not a FHIR-resource concern (service spec §9).
- Production event-bus (Fluvio) publishers / consumers — in-memory
  only (service spec §13 T-1).
- ML / embedding-based match scoring (service spec §13 T-5).
- Authentication enforcement — planned via the central
  [authentication entity](../../authentication/) (§15).
- File / blob storage for images.
