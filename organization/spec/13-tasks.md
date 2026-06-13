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
- [ ] **T-6 — Record merge with link tracking (service).**
  - [ ] Survivor + duplicate, former-name alias, `Replaces` link,
    transferred-data snapshot, soft-delete duplicate, `Merged` event
    — parity with [`agents/share/merge.md`](../../agents/share/merge.md).
  - **Acceptance:** integration test merges two records and verifies
    snapshot + soft delete + event.
- [ ] **T-7 — Scale the duplicate check beyond the 1 000-row scan.**
  - [ ] Blocking / candidate pre-selection (name trigram or
    identifier lookup) before scoring; lift the cap.
  - **Acceptance:** check-duplicates returns identical top results on
    a seeded corpus with and without blocking, with bounded latency.
- [ ] **T-8 — Tantivy full-text search replacing `ILIKE`.**
  - [ ] Family-standard fuzzy + phonetic search; keep `/search`
    stable. (Also queued in the service crate's §13.)
  - **Acceptance:** fuzzy query (`"Acmee"`) finds `"Acme, Inc."`.
- [ ] **T-9 — JWT verification end to end.**
  - [ ] Service middleware consuming the auth-service JWKS; populate
    audit `actor` from the token subject; front-end bearer wiring.
  - **Acceptance:** unauthenticated POST → `401`; audit rows carry
    the actor.
- [ ] **T-10 — Durable event bus.**
  - [ ] Replace the in-memory ring buffer behind the same publish
    call (the buffer is the documented swap point in
    [`src/streaming.rs`](../organization-service-rust-crate/src/streaming.rs));
    unblocks NFR-2 scale-out.
  - **Acceptance:** events survive a process restart; two replicas
    see one stream.
- [ ] **T-11 — Front-end catch-up: search box, audit views, tests.**
  - [ ] Search box over `/search`; audit view over the audit
    endpoints; vitest + Playwright (queued in the front-end's §13).
  - **Acceptance:** front-end spec §13 boxes ticked.
- [x] **T-12 — Remove loco scaffolding leftovers (service).** *(done
  2026-06-13)*
  - [x] Deleted `src/workers/` (the `DownloadWorker` TODO stub and its
    `connect_workers` registration in `app.rs`) and the empty
    `src/data/` + `src/tasks/` modules; `lib.rs` trimmed to match.
  - **Acceptance met:** `cargo build` + clippy clean; no TODO-stub
    workers remain.
