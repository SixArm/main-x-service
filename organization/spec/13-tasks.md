## 13. Tasks

Entity-level work queue: cross-subproject items, documentation gaps,
and integration-contract fixes. Work that is internal to one
subproject belongs in that subproject's own §13 (service / front-end)
or §23 (matcher) — entries here reference, not duplicate, those
queues. Each task has an acceptance criterion; tick the box when an
automated test or clearly described manual check confirms it. Split
oversized tasks (`T-2a`, `T-2b`).

- [ ] **T-1 — Bring the service crate's doc set up to house shape.**
  - [ ] Split `spec/index.md` into the 18-file `spec/` layout used by
    the mature entities.
  - [ ] Add the `AGENTS/` doc set (`index.md`,
    `spec-driven-development.md`, `models.md`, `matching.md`,
    `restful.md`, `testing.md`) per the root `AGENTS.md` contract.
  - **Acceptance:** the service crate matches the per-crate doc list
    in the root [`AGENTS.md`](../../AGENTS.md); links resolve.
- [x] **T-2 — Resolve the blank-name status-code drift.** *(done
  2026-06-13)*
  - [x] Decided for the family convention: `422 Unprocessable Entity`
    for validation failures (blank `name` on create **and** replace;
    `loco_rs::Error::CustomError(422, …)`). Code, OpenAPI, crate spec
    §6/§9/§11, and §9 here all agree. Unknown pid now maps to `404`
    (was a 500 via loco's default `ModelError::EntityNotFound`
    mapping).
  - **Acceptance met:** request-level test
    (`tests/requests/organizations.rs::blank_name_returns_422`) posts
    `{"name":" "}` and asserts `422`; a DB-free unit test in
    `src/controllers/organizations.rs` pins the same mapping.
- [x] **T-3 — Fix wire-format naming in service docs.** *(done
  2026-06-13)*
  - [x] Docs corrected to the actual snake_case wire format
    (`legal_name`, `same_as`, `founding_date`, …): service
    `README.md` / `index.md` / `AGENTS.md` and front-end `README.md`.
    No serde rename added — snake_case is canonical (OQ-1 resolved,
    §16).
  - **Acceptance met:** docs match an actual `GET /{pid}` response
    body (pinned by the create round-trip request test).
- [x] **T-4 — Request-level integration tests (service).** *(done
  2026-06-13)*
  - [x] Standard loco harness (`loco_rs::testing`, `serial_test`) in
    `tests/requests/organizations.rs`: create round-trip, blank-name
    `422` on create + update, unknown-pid `404`, search (+ blank `q`
    `400`), check-duplicates ranking. Tests are `#[ignore]`-gated
    (family convention, cf. person-service) so the default
    `cargo test` stays green without a database; run with
    `cargo test -- --ignored`. Audit-endpoint coverage can grow with
    T-9 (actor wiring).
  - **Acceptance:** `cargo test -- --ignored` runs the suite against
    Postgres (verified locally 2026-06-13, 6/6 green); wire into CI
    when a CI pipeline exists.
- [ ] **T-5 — Privacy layer (service): masking + GDPR export.**
  - [ ] Per-field masking honouring the §12 split (contact fields and
    sole-trader records protected; register identity open); export
    endpoint.
  - **Acceptance:** masked view hides `telephone` / `email`; export
    returns the full stored payload + audit trail.
- [x] **T-6 — Record merge (service).**
  - [x] Survivor + duplicate: union list fields, former-name alias,
    transferred-data snapshot, soft-delete duplicate, `Merged` event
    — parity with [`agents/share/merge.md`](../../agents/share/merge.md).
    **Done (2026-06-13):** pure `src/merge.rs` (`merge_orgs`) +
    `POST /api/organizations/merge` and
    `GET /api/organizations/merges/recent`; migration
    `m20220101_000003_merge_records` + `models/merge_records.rs`. Equal
    pids → `422`, unknown pid → `404`. (A typed `Replaces` link between
    survivor and duplicate is not modelled — the `merge_records` row
    captures the relationship; a link table is a follow-up if needed.)
    `actor` is `NULL` until token auth (T-9).
  - **Acceptance:** integration test merges two records and verifies
    snapshot + soft delete + event. **Met (DB-gated):**
    `merge_folds_duplicate_into_survivor`, `merge_with_equal_pids_is_422`,
    `merge_unknown_pid_is_404`; algorithm pinned un-gated by five
    `merge::tests` cases.
- [ ] **T-7 — Scale the duplicate check beyond the 1 000-row scan.**
  - [x] Observable cap: the scan limit is the named constant
    `CHECK_DUPLICATES_SCAN_CAP` (= 1 000) with a doc comment, the
    handler emits a `WARN` when the scan saturates the cap (silent
    truncation becomes observable), and a unit test pins the constant.
    (spec §6 FR-3, §7 NFR-1)
  - [ ] Blocking / candidate pre-selection (name trigram or
    identifier lookup) before scoring; lift the cap.
  - [ ] *Test gap:* the cap-boundary truncation behaviour (scan
    saturates `CHECK_DUPLICATES_SCAN_CAP` ⇒ WARN + silent miss beyond
    the cap) has only a DB-free constant pin; add a Postgres-gated
    request test that seeds > 1 000 rows and asserts the WARN / observed
    truncation, so the code comment's claim of coverage holds.
  - **Acceptance:** check-duplicates returns identical top results on
    a seeded corpus with and without blocking, with bounded latency.
- [ ] **T-8 — Tantivy full-text search replacing `ILIKE`.**
  - [ ] Family-standard fuzzy + phonetic search; keep `/search`
    stable. (Also queued in the service crate's §13.)
  - **Acceptance:** fuzzy query (`"Acmee"`) finds `"Acme, Inc."`.
- [x] **T-9 — Offline token verification (service).**
  > Credential model is now **PASETO v4.public** per
  > [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
  > which supersedes the RS256-JWT + JWKS model the items below shipped
  > against. The `[x]` items record what landed in Rust; the migration
  > to PASETO is the open sub-task.
  - [x] Offline verification consuming the auth-service published key;
    populate audit `actor` (and merge `actor`) from the token subject.
    **Done (2026-06-13, against RS256-JWT/JWKS):** `src/auth.rs` embeds the
    [`authentication-verifier`](../../authentication/authentication-verifier-rust-crate)
    crate behind a process-wide `Verifier`. `AuthUser`
    (required) + `MaybeAuthUser` (optional) extractors; `GET
    /api/organizations/whoami` is protected; create/update/delete/merge
    stamp the audit + merge `actor` from the token when present.
  - [ ] **Switch `src/auth.rs` to verify PASETO v4.public** per
    [`authentication-sessions.md`](../../agents/share/authentication-sessions.md):
    `authentication-verifier` `from_paseto_keys_value` /
    `from_paseto_keys_url` (was `from_jwks_*`); same `Claims` shape
    (`kid`/`iss`/`aud`/`exp`, `kid` in the footer); `Verifier` built from
    `ORGANIZATION_PASETO_KEYS` / `ORGANIZATION_TOKEN_ISSUER` /
    `ORGANIZATION_TOKEN_AUDIENCE` (was `ORGANIZATION_JWKS` /
    `ORGANIZATION_JWT_ISSUER` / `ORGANIZATION_JWT_AUDIENCE`). Token rides
    in `Authorization: Bearer v4.public.…`.
  - **Acceptance:** unauthenticated `whoami` → `401`; audit rows carry
    the actor when a token is sent. **Met:** `whoami_without_token_is_401`
    (DB-gated) + six un-gated crypto unit tests in `auth::tests`.
  - [x] *Follow-up — blanket `/api/*` enforcement.* **Done:**
    `auth::enforce` is wired as an `axum::middleware::from_fn` layer in
    `App::after_routes`, gated by `ORGANIZATION_REQUIRE_AUTH` (lenient
    bool, **default-off**); public paths (health/ping, OpenAPI/Swagger,
    `/metrics.prom`) stay open. Pure decision unit-tested in
    `auth::tests`; DB-gated `require_auth_gate_blocks_unauthed_list_but_allows_openapi`.
    `enforce()` shape is unchanged under PASETO — only `bearer_claims`
    verifies a PASETO token. Family contract:
    [`agents/share/jwt-enforcement.md`](../../agents/share/jwt-enforcement.md).
  - [ ] *Follow-up — paseto-keys-over-HTTP fetch at boot* (currently
    injected via env `ORGANIZATION_PASETO_KEYS`); fetch + cache the
    auth-service `/.well-known/paseto-keys` at startup behind the
    `authentication-verifier` `fetch` feature.
  - [ ] *Follow-up — request-level whoami token-accepted (200) test.*
    The token-rejected (`401`) path is DB-gated; the crypto accept path
    is only unit-tested in `auth::tests`. A request-level 200 test needs
    a test-only token-mint helper whose keys match the app's
    env-configured `Verifier` (boot with `ORGANIZATION_PASETO_KEYS` set to
    a throwaway key set, send a token minted from the matching private key).
- [ ] **T-10 — Durable event bus.**
  - [ ] Replace the in-memory ring buffer behind the same publish
    call (the buffer is the documented swap point in
    [`src/streaming.rs`](../organization-service-with-loco/src/streaming.rs));
    unblocks NFR-2 scale-out.
  - **Acceptance:** events survive a process restart; two replicas
    see one stream.
- [ ] **T-11 — Front-end catch-up: search box, audit views, tests.**
  - [x] vitest + Playwright. **Done (2026-06-13):** `tests/unit/`
    (16 — `ApiClient` + `OrganizationRepository`, incl. a
    `check-duplicates` path regression) and `tests/e2e/smoke.spec.ts`
    (4 routes, API stubbed, runs on `vite preview`). Also fixed two
    scaffold copy artifacts (`client.ts` "Authentication Service"
    header, `app.html` "Course Service" description).
  - [ ] Search box over `/search`; audit view over the audit endpoints.
  - **Acceptance:** front-end spec §13 boxes ticked. *(tests ticked;
    search-box / audit-view UI remain.)*
- [x] **T-12 — Remove loco scaffolding leftovers (service).** *(done
  2026-06-13)*
  - [x] Deleted `src/workers/` (the `DownloadWorker` TODO stub and its
    `connect_workers` registration in `app.rs`) and the empty
    `src/data/` + `src/tasks/` modules; `lib.rs` trimmed to match.
  - **Acceptance met:** `cargo build` + clippy clean; no TODO-stub
    workers remain.
- [ ] **T-13 — Cross-service link **target** readiness (service).**
  Organization is a v1 link *target* only (§8.6;
  [`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md)
  §9) — no `entity_links` write-side table, no `/links` surface. The
  small code follow-ups make it a *good* target:
  - [ ] Confirm the `created` / `deleted` / `merged` events carry the
    fields the aggregator's `entity_presence` oracle and merge-repoint
    handler need (that doc §5, §5.3): `pid` on create/delete; `pid` +
    `merged_from` on merge — so a deleted org can flip inbound edges to
    `dangling` and a merge can repoint them centrally. (The current
    `Envelope` carries `pid`; verify `merged` exposes `merged_from`.)
  - [ ] Confirm the matcher adapter path never sees cross-service links:
    only the within-entity `relationships[]` reach
    `MatchingEngine` (§5 partition rule); there is no `entity_links`
    feed into matching.
  - **Acceptance:** an event-shape test asserts `created`/`deleted`
    carry `pid` and `merged` carries `merged_from`; a matcher test
    confirms no cross-service edge field is read by scoring.
- [ ] **T-14 — Bulk import / export (service).** Adopt the family-wide
  contract in [`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md);
  the organization-specific declarations (stable key, CSV columns, export
  sensitivity) are §8.7.
  - [ ] `bulk_jobs` migration (per the shared doc §3 table).
  - [ ] The five endpoints (shared doc §4): `POST /api/v1/organizations/import`,
    `GET …/import/{id}`, `POST …/export`, `GET …/export/{id}`,
    `GET …/bulk-jobs`. (§9.1.)
  - [ ] `bg_pg` background worker draining queued → running → terminal,
    with progress + count updates.
  - [ ] JSONL / CSV / Parquet codecs (JSONL lossless reference; CSV per the
    §8.7 column set — `address.*` dotted, `identifiers` / `alternate_names`
    / `same_as` / `keywords` / `tags` / `relationships` JSON-in-cell;
    Parquet export-first, feature-gated).
  - [ ] Per-row pipeline reusing the **single-create validators** + the
    embedded **organization-matcher** + the **review queue**
    (`provenance = import`): upsert by the §8.7 stable key (deterministic
    scheme-scoped identifier or `pid`), else duplicate-detect → create or
    review.
  - [ ] Downloadable per-row **error report**
    (`row_number, source_line, field, code, message`); reconcile final
    counts; `completed` vs `completed_with_errors`.
  - [ ] **Export masking + audit**: `masking_profile` (light default —
    protect `telephone` / `email` + sole-trader, parity with T-5), gated
    `include_soft_deleted`, every export audited even at zero rows.
  - **Acceptance:** tests for idempotent re-import (re-run upserts to the
    same state), per-row error report, keyless-row dedupe-to-review with
    `provenance = import`, masked vs full export, and the export audit row.
