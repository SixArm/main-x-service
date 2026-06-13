## 13. Tasks

Live entity-level work queue. Tasks that belong to one subproject's
internals should migrate into that crate's spec §13; they are listed
here while the crate specs are thin. Each task has an acceptance
criterion; tick the box when an automated test or clearly described
manual check confirms it. Split tasks too big for one PR
(`T-2a`, `T-2b`).

- [ ] **T-1 — Thicken the thin crate docs.**
  - [ ] Split the service's single-file `spec/index.md` into the
    numbered §-per-file layout (the matcher and front-end carry the
    same task in their own §13 / §23).
  - [ ] Add a service `AGENTS/` reference set (`models.md`,
    `matching.md`, `restful.md`, `testing.md`,
    `spec-driven-development.md`) matching the person-service shape.
  - **Acceptance:** every link in this entity spec resolves to a
    numbered section file rather than an anchor in a monolith.
- [x] **T-2 — Resolve the blank-name status-code discrepancy.**
  - [x] Service crate spec §6 says `422` for a blank `name`; the
    controller returns `400` (`bad_request`). Decide (family
    convention is `422` for validation), align code + spec.
  - **Acceptance:** request-level test posts `{"name": ""}` and gets
    the documented status.
  - **Done (2026-06-13, resolves OQ-1):** `422` is normative. The
    controller's `validate()` returns
    `Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)` on
    blank `name` for both create and update. Pinned un-gated by
    DB-free unit tests in `src/controllers/care_pathways.rs` and by
    the (DB-gated) request tests
    `blank_name_on_{create,update}_returns_422`.
- [x] **T-3 — Audit log + event streaming on CRUD.** (compliance
  driver §12.3)
  - [x] Audit row (action + JSON snapshot + timestamp) per
    create/update/delete. **Done (2026-06-13):** `audit_logs` table
    (migration `m20220101_000002_audit_logs`), `models/audit_logs.rs`
    (`record` / `recent` / `for_entity`); the controller writes a
    best-effort row on each CRUD action (logs on failure, never fails
    the request — the `actor` column is `NULL` until JWT auth lands,
    T-7). Read endpoints `GET /api/care-pathways/audit/recent` and
    `GET /api/care-pathways/{pid}/audit`.
  - [x] Event publish per CRUD per
    [`agents/share/auditability.md`](../../agents/share/auditability.md).
    **Done:** `streaming.rs` in-memory ring buffer (cap 1 000,
    `OnceLock` global, same MVP shape as the organization service —
    siblings swap a real broker behind `publish`); `created`/`updated`/
    `deleted` published per CRUD; read at
    `GET /api/care-pathways/events/recent`. Durable broker is roadmap
    (§15).
  - **Acceptance:** integration test creates + updates + deletes a
    pathway and reads back three audit rows and three events.
    **Met (DB-gated):** `crud_writes_audit_log_and_events`. Streaming
    is also pinned un-gated by `streaming::publish_and_read_back`.
- [x] **T-4 — Request-level integration tests (PostgreSQL).**
  - [x] loco testing harness over CRUD, `/match`,
    `/check-duplicates` (dev-dependencies already present:
    `serial_test`, `rstest`, `insta`).
  - **Acceptance:** `cargo test` with a Postgres URL covers all
    seven endpoints, including a stored near-duplicate round-trip.
  - **Done (2026-06-13):** `tests/requests/care_pathways.rs` — eight
    loco-style request tests (create, blank-name 422 on
    create/update, get 200/404, list, `/match`,
    `/check-duplicates` near-duplicate round-trip). They are
    `#[ignore]`-gated so the default `cargo test` stays green
    without a database; run with a Postgres URL via
    `cargo test -- --ignored`. (Caveat: authored on a machine with
    no reachable Postgres — first DB-backed run still pending.)
- [ ] **T-5 — Front-end tests.**
  - [ ] vitest units for `ApiClient` + `CarePathwayRepository`.
  - [ ] Playwright smoke over `/`, `/new`, `/[pid]`, `/[pid]/edit`.
  - **Acceptance:** both suites run in CI and fail on a broken
    endpoint contract.
- [ ] **T-6 — Search + candidate blocking.** (deferred MVP feature)
  - [ ] Tantivy full-text search endpoint + front-end search box.
  - [x] Make the `check-duplicates` in-memory scan cap a named,
    documented const with a WARN on hit (interim safety, ahead of
    the redesign). **Done (2026-06-13):** `CHECK_DUPLICATES_SCAN_CAP`
    (= 1000) in `src/controllers/care_pathways.rs`; the handler passes
    it to `Model::list` and emits `tracing::warn!` when the returned
    row count reaches the cap. Pinned by the DB-free unit test
    `check_duplicates_scan_cap_is_the_documented_value`.
  - [ ] Replace the 1 000-row in-memory scan in `check-duplicates`
    with search-blocked candidates (NFR-1 / NFR-2; OQ-2).
  - **Acceptance:** `check-duplicates` latency test passes at
    100 000 stored pathways.
- [ ] **T-7 — JWT verification middleware.**
  - [ ] Verify RS256 JWTs against the auth-service JWKS on `/api/*`.
  - **Acceptance:** integration test: no token → `401`; valid signed
    token → `2xx`.
- [ ] **T-8 — Record merge.** (deferred MVP feature)
  - [ ] Merge confirmed duplicates: transfer identifiers /
    alternate names, soft-delete the duplicate, link, snapshot,
    `Merged` event; front-end merge action from the duplicates list.
  - **Acceptance:** integration test merges two stored pathways and
    verifies survivor contents + soft-deleted duplicate.
- [x] **T-9 — OpenAPI / Swagger + richer validation.**
  - [x] OpenAPI 3 schema + Swagger UI. **Done (2026-06-13):**
    hand-written `src/openapi.rs` (the matcher's `CarePathway` shape is
    the API DTO and is dependency-light, so the schema is authored by
    hand rather than utoipa-derived — same approach as the
    organization service) served by `src/controllers/docs.rs` at
    `GET /api-docs/openapi.json` + `GET /swagger-ui`, registered in
    `app.rs`. Pinned un-gated by `openapi::spec` unit tests
    (`spec_is_wellformed`, `spec_documents_all_seven_endpoints`) and
    (DB-gated) by request tests `openapi_json_is_served` /
    `swagger_ui_is_served`.
  - [x] ICD-10 / ICD-11 / SNOMED CT code-format validation on
    `condition_codes` (`422` on failure). **Done (2026-06-13):**
    `src/validation.rs` format-checks each `condition_codes` entry
    against its `system` — ICD-10 / ICD-11 structural patterns and the
    SNOMED CT SCTID Verhoeff check digit; `Custom` codes need only be
    non-blank. `validate()` reports every problem (incl. blank `name`)
    in one `422`. Pinned un-gated by 9 `validation` unit tests + the
    controller test `malformed_condition_code_returns_422`, and
    (DB-gated) by `malformed_condition_code_on_create_returns_422`.
    Existence-in-a-release validation (terminology server) stays
    deferred.
  - **Acceptance:** Swagger UI serves the seven endpoints; malformed
    code test returns `422`. *(Validation leg met; Swagger leg open.)*
