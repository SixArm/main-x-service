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
- [ ] **T-3 — Audit log + event streaming on CRUD.** (deferred MVP
  feature; compliance driver §12.3)
  - [ ] Audit row (old/new JSON, user context, timestamp) per
    create/update/delete.
  - [ ] Event publish per CRUD per
    [`agents/share/auditability.md`](../../agents/share/auditability.md).
  - **Acceptance:** integration test creates + updates + deletes a
    pathway and reads back three audit rows and three events.
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
- [ ] **T-9 — OpenAPI / Swagger + richer validation.** (deferred)
  - [ ] utoipa schema + Swagger UI.
  - [ ] ICD-10 / ICD-11 / SNOMED CT code-format validation on
    `condition_codes` (`422` on failure).
  - **Acceptance:** Swagger UI serves the seven endpoints; malformed
    code test returns `422`.
