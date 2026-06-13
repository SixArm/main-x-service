## 2. Scope

### 2.1 In scope — entity level

This spec owns the **cross-subproject contract**:

- Composition: front-end → service REST API → embedded matcher.
- The DTO contract: the API request/response body **is**
  `case_matcher::Case`, stored verbatim as JSONB (§5). There is no
  separate service model and no adapter.
- The service ↔ front-end wire contract: raw loco JSON (no
  envelope), TypeScript type mirroring (§5.4).
- Shared invariants that more than one subproject must uphold (§5.5).
- Entity-wide goals: worldwide-governmental scale, multi-locale,
  auditability, privacy/compliance posture (§7, §12).

### 2.2 In scope — per subproject

**case-service-rust-crate** owns:

- Case CRUD with soft delete (`deleted_at`), `422` validation.
- `GET …/search?q=` (Postgres `ILIKE` title search),
  `POST …/match` (rank an explicit candidate set, no persistence),
  `POST …/check-duplicates` (match a query against stored cases),
  `POST …/merge` (+ `/merges/recent` history).
- Audit logging, in-memory event streaming, offline RS256 JWT
  verification (`whoami`, audit/merge `actor` stamping), OpenAPI 3 +
  Swagger UI.
- The `cases` table (`pid`, denormalised `title`, the full `Case`
  JSONB `data`, `active`, `deleted_at`) plus `audit_logs` and
  `merge_records`.
- loco.rs app structure, migrations, configuration.

**case-matcher-rust-crate** owns:

- Pure-library pairwise comparison: deterministic short-circuits
  (`Docket` / `ExternalCaseId` / `Uri` / `Uuid`, same-agency
  `case_number`, `same_as` URL overlap) + weighted probabilistic
  scoring (title, subjects, case number, case type, status, keywords)
  with per-component breakdown.
- The `Case` domain type and its supporting enums — the entity's
  canonical DTO.
- Normalisation (`fold`, case-number folding, set folding), Soundex
  bonus, config presets (`strict` / `default` / `lenient`).

**case-front-end-with-svelte** owns:

- Operator routes: list (`/`), create (`/new`), detail + delete +
  check-duplicates (`/[pid]`), edit (`/[pid]/edit`).
- Its own copy of API types, client, and form primitives (drift
  between front-ends is accepted — repo decision 2026-06-02).

### 2.3 Out of scope (today) — deferred

The entity ships CRUD + matching + audit + streaming + merge + JWT
verification + OpenAPI. Explicitly deferred, tracked in §13 / §15 and
the crate specs' §13:

- **Per-field privacy masking + GDPR data-subject export** — *raised
  in priority because case data is personal data* (§12); not yet
  built.
- Durable event bus (today the stream is an in-process ring buffer).
- Tantivy full-text / fuzzy search over the JSONB payload (today only
  `ILIKE` title search) and a front-end search box / audit views.
- Blanket `/api/*` JWT enforcement and JWKS-over-HTTP fetch (today the
  verifier is env-injected and only `whoami` is protected).
- Real-time duplicate detection on create (`409 Conflict`).
- Deeper validation (case-number / docket format, status-transition
  rules, terminology checks).
- gRPC.
