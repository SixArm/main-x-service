## 13. Tasks

Live entity-level work queue. Tasks that belong to one subproject's
internals should migrate into that crate's spec §13; they are listed
here while the crate specs are thin. Each task has an acceptance
criterion; tick the box when an automated test or clearly described
manual check confirms it. Split tasks too big for one PR
(`T-2a`, `T-2b`).

- [x] **T-1 — Stand up the trio (CRUD + matching).**
  - [x] matcher: `Case` type + enums; deterministic + probabilistic
    matching with per-component breakdown; presets.
  - [x] service: loco.rs chassis, `cases` table, CRUD with soft delete,
    `/match`, `/check-duplicates`.
  - [x] front-end: `/`, `/new`, `/[pid]`, `/[pid]/edit` over the REST
    API.
  - **Acceptance:** create → read → match → check-duplicates round-trip
    works end to end.
- [x] **T-2 — Validation (`422`).**
  - [x] Blank `title` → `422` on create and update (family convention).
  - [x] `opened_date` ISO-format check; blank identifier `value`; blank
    `subjects` / `keywords` entries. All problems reported in one
    `422` (`src/validation.rs`).
  - **Acceptance:** unit tests + request tests post each bad shape and
    get `422`; `400` stays for malformed bodies.
- [x] **T-3 — Audit log + event streaming.**
  - [x] `audit_logs` table + best-effort row per create/update/delete/
    merge (action + JSON snapshot + `actor`); read at
    `/audit/recent`, `/{pid}/audit`.
  - [x] In-memory `CaseEvent` ring buffer (cap 1 000); `created`/
    `updated`/`deleted`/`merged` published; read at `/events/recent`.
  - **Acceptance:** integration test creates + updates + deletes a case
    and reads back the audit rows and events; streaming pinned un-gated
    by `streaming::publish_and_read_back`.
- [x] **T-4 — Request-level integration tests (PostgreSQL).**
  - [x] loco testing harness over CRUD, `/search`, `/match`,
    `/check-duplicates`, `/merge`, audit/events, `whoami`, OpenAPI.
  - **Acceptance:** `cargo test -- --ignored` with a Postgres URL
    covers every endpoint, including a stored near-duplicate round-trip.
    (`#[ignore]`-gated so default `cargo test` stays DB-free.)
- [x] **T-5 — Front-end tests.**
  - [x] vitest units for `ApiClient` + `CaseRepository` (incl. a
    `check-duplicates` path regression).
  - [x] Playwright smoke over the four routes (API stubbed, runs on
    `vite preview`).
  - **Acceptance:** both suites run and fail on a broken endpoint
    contract.
- [x] **T-7 — Token verification (partial).**
  - [x] Verify tokens offline against the auth-service's published key via
    the embedded `authentication-verifier` (`src/auth.rs`), built from
    `CASE_PASETO_KEYS` / `CASE_TOKEN_ISSUER` / `CASE_TOKEN_AUDIENCE`.
    `AuthUser` (required) and `MaybeAuthUser` (optional) extractors;
    `/whoami` protected; audit / merge `actor` stamped from the token.
  - **Acceptance:** no token → `401`; valid signed token → `2xx`
    (un-gated crypto unit tests mint a token + matching key in-process).
  - [ ] Switch the credential from RS256 JWT to PASETO v4 public
    (Ed25519) per
    [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
    (source of truth; supersedes the RS256-JWT + JWKS model): verifier
    consumes the auth-service's published Ed25519 key(s)
    (`Verifier::from_paseto_keys_value` / `from_paseto_keys_url`); same
    `Claims` shape (`kid`/`iss`/`aud`/`exp`; footer carries `kid`);
    un-gated unit tests mint a real PASETO v4 public token + matching
    Ed25519 key in-process.
  - [ ] *Follow-up:* blanket enforcement on every `/api/*` route (awaits
    the coordinated family SSO rollout; the front-end BFF attaches the
    bearer token server-side) and paseto-keys-over-HTTP fetch from the
    auth service at boot (currently injected via env).
- [x] **T-8 — Record merge.**
  - [x] `POST /merge` folds a duplicate into a survivor (union list
    fields, former-title alias, soft-delete the duplicate,
    `merge_records` history + snapshot, `Merged` + `Deleted` events);
    equal pids → `422`, unknown → `404`; `/merges/recent` history.
    Pure `src/merge.rs`.
  - **Acceptance:** integration test merges two stored cases and
    verifies survivor contents + soft-deleted duplicate; merge logic
    pinned un-gated.
  - [ ] *Follow-up:* a front-end merge action from the duplicates list
    (T-5 / T-11 territory).
- [x] **T-9 — OpenAPI / Swagger.**
  - [x] Hand-written `src/openapi.rs` (the matcher's `Case` shape is the
    API DTO and is dependency-light, so the schema is authored by hand,
    not utoipa-derived) served at `/api-docs/openapi.json` + `/swagger-ui`.
  - **Acceptance:** Swagger UI serves every documented endpoint;
    `openapi::spec` unit tests assert well-formedness + endpoint
    coverage.
  - [ ] *Follow-up:* deeper validation — docket / case-number format
    checks, status-transition rules, terminology checks.

### Open / deferred

- [ ] **T-10 — Privacy: per-field masking + GDPR data-subject export.**
  *(highest-priority gap — case data is personal data, §12)*
  - [ ] Masked-view endpoint (`GET /api/cases/{pid}/masked`) applying
    per-field masking rules.
  - [ ] GDPR data-subject export (`GET /api/cases/{pid}/export`) and a
    subject-scoped export across cases sharing a `subjects` id.
  - [ ] A GDPR-erasure path layered on soft delete (retention policy).
  - **Acceptance:** masked view hides the configured fields; export
    returns a complete, machine-readable record set for a subject;
    erasure is auditable.
- [ ] **T-6 — Search + candidate blocking.** (partly done)
  - [x] Name/title search endpoint. **Done:** `GET /api/cases/search?q=`
    — Postgres `ILIKE` substring match on the denormalised `title`
    (cap 50, wildcards escaped); blank `q` → `400`.
  - [x] Make the `check-duplicates` in-memory scan cap a named,
    documented const (`CHECK_DUPLICATES_SCAN_CAP` = 1 000) with a
    `tracing::warn!` on hit.
  - [ ] Tantivy full-text / fuzzy search over the JSONB payload.
  - [ ] Replace the 1 000-row in-memory scan in `check-duplicates` with
    search-blocked candidates (NFR-1 / NFR-2; OQ-2).
  - **Acceptance:** `check-duplicates` latency test passes at
    1 000 000 stored cases.
- [ ] **T-11 — Front-end search box + audit / event views.**
  - [ ] A search box on `/` calling `GET /api/cases/search?q=`.
  - [ ] Audit-trail and event views (consume `/{pid}/audit`,
    `/audit/recent`, `/events/recent`).
  - **Acceptance:** the UI surfaces search results and a case's audit
    trail.
- [ ] **T-12 — Durable event bus.**
  - [ ] Replace the in-process ring buffer with a durable broker so peer
    registries and analytics can subscribe across replicas.
  - **Acceptance:** events survive a replica restart and are delivered
    cross-replica.
- [ ] **T-13 — Thicken crate docs.**
  - [ ] Add a service `AGENTS/` reference set (`models.md`,
    `matching.md`, `restful.md`, `testing.md`,
    `spec-driven-development.md`) matching the sibling shape; split any
    single-file crate specs into the numbered layout.
  - **Acceptance:** every link in this entity spec resolves to a real
    file.
