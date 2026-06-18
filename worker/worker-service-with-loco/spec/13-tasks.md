## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [ ] **T-1 — JWT middleware on `/api/*`.**
  - [ ] Add `jsonwebtoken` validator extractor with HR-admin /
    credentialing-officer / read-only / service roles.
  - **Acceptance:** unauthenticated requests get `401`; valid signed
    token with sufficient role gets `2xx`.
- [ ] **T-2 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag `fluvio`.
  - **Acceptance:** integration test publishes a `WorkerCreated`
    record end-to-end against a local Fluvio broker.
- [ ] **T-3 — FHIR capability statement + bundle handling.**
  - [ ] `GET /fhir/metadata` returns a CapabilityStatement listing
    the `Worker` resource (the wire `resourceType` this server
    actually emits — see §6.8).
  - [ ] `Bundle` GET / POST / search wrapping.
  - **Acceptance:** Touchstone FHIR validator passes on a sample
    bundle round-trip.
- [ ] **T-4 — FHIR Organization resource.**
  - [ ] Bidirectional Organization mapping.
  - **Acceptance:** `POST /fhir/Organization` round-trips a record.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [ ] **T-6 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `WorkerService.GetWorker`
    round-trips a record.
- [ ] **T-7 — Credential-expiry warning workflow.**
  - [ ] Background scan: `IdentityDocument.expiry_date` within 30
    days → publish `CredentialExpiringSoon` event.
  - [ ] Custom metric `credential_expiry_within_30d`.
  - **Acceptance:** integration test seeding a credential with
    `expiry_date = today + 25d` produces the event + metric.
- [ ] **T-8 — Role + assignment history timeline.**
  - [ ] Per-worker timeline of role / organisation assignments.
  - **Acceptance:** new assignment creates a timeline entry visible
    via `GET /api/workers/{id}/timeline`.
- [x] **T-9 — Mount the FHIR routes on the loco router.** *(Done
  2026-06-13.)*
  - [x] The `/fhir/Worker` handlers in `src/api/fhir/handlers.rs` are
    now registered: `App::routes` adds `workers_routes()` +
    `fhir_routes()` + `metrics_routes()`, and `create_router` mirrors
    the `/fhir/Worker` surface for the integration-test harness. The
    new `fhir_routes()` `Routes` group serves GET/POST `/fhir/Worker`
    and GET/PUT/DELETE `/fhir/Worker/{id}`; its handlers extract
    `AppState` via `FromRef` exactly like the REST surface.
  - [x] **Acceptance:** the mount is pinned by two route tests in
    `tests/api_integration_test.rs` — `test_fhir_worker_route_is_mounted`
    (un-gated; a malformed UUID makes the `Path<Uuid>` extractor return
    `400`, proving the route matched a handler rather than a route-level
    `404`) and `test_fhir_worker_not_found_returns_operation_outcome`
    (DB-gated; a valid-but-absent id returns a FHIR `OperationOutcome`
    `404`). Closes entity-level task T-1.
- [ ] **T-10 — Cross-service entity links (write side).** Implements
  domain model §5.4, architecture §8.6, API §9.1, persistence §10.3 —
  per [cross-service linking](../../../agents/share/cross-service-linking.md)
  (rollout §11 step 2, the `same_identity` backbone + `employed_by`).
  - [ ] Migration adding the `entity_links` table (§10.3 schema) with the
    `UNIQUE (from_pid, kind, to_ref, valid_from)` idempotent-upsert key.
  - [ ] Copy the `EntityRef` value type + the §9 edge-kind registry into
    the crate (drift-accepted; `parse` / `Display` + `entity_type →
    service` map); validate `kind` ∈ {`same_identity`, `employed_by`} and
    the `to_ref` entity type matches the kind's endpoint.
  - [ ] `POST` / `GET` / `DELETE /api/v1/workers/{pid}/links` controllers
    (optimistic upsert / list / soft-delete; **no** cross-service call).
  - [ ] Emit `linked` / `unlinked` on the existing event envelope via the
    existing `EventProducer` (envelope `entity` = `worker`, edge detail in
    `data`).
  - [ ] **Matcher-adapter partition guard:** assert in
    `src/matching/adapter.rs` that `entity_links` is never projected into
    matcher input (the partition rule, §5.1).
  - **Acceptance:** integration test creates an `employed_by` edge
    (with `role` + `valid_from`), lists it, soft-deletes it, and asserts a
    `linked` then `unlinked` event is published; a unit test asserts the
    matcher input excludes `entity_links` and that match scores are
    unchanged by adding a cross-service edge.
- [ ] **T-11 — Bulk import / export.** Implements persistence §10.4 —
  per [bulk import / export](../../../agents/share/bulk-import-export.md)
  (the uniform contract; only Worker's stable keys + CSV columns +
  export sensitivity differ, §10).
  - [ ] Migration adding the `bulk_jobs` table (shared §3 schema) with the
    `UNIQUE (entity, kind, idempotency_key)` retried-submit key.
  - [ ] The five endpoints (shared §4): `POST` / `GET
    /api/v1/workers/import`, `POST` / `GET /api/v1/workers/export`, and
    `GET /api/v1/workers/bulk-jobs` (list + by-id).
  - [ ] `bg_pg` background worker draining `queued → running →
    completed | completed_with_errors | failed` with progress updates.
  - [ ] JSONL (lossless reference) + CSV (the §10.4 column set /
    flattening) codecs; Parquet **export-first, feature-gated** (import
    is roadmap).
  - [ ] Per-row import pipeline reusing the **single-create validators +
    worker-matcher + review queue** verbatim: parse + validate →
    upsert-by-stable-key (person-level identifier `(identifier_type,
    system, value)` / `tax_id` / `pid`, §10.4) → else duplicate detection
    → likely-duplicate to review queue with `provenance = import`, else
    create. Emit the normal `WorkerCreated` / `WorkerUpdated` event +
    audit record per row (no bulk bypass).
  - [ ] Downloadable per-row **error report** (`row_number, source_line,
    field, code, message`); one bad row never aborts the load; final
    counts reconcile (`rows_total = created + upserted + to_review +
    errored`).
  - [ ] Export **masking + audit**: `masking_profile` (masked default;
    full/unmasked gated to HR-admin / credentialing-officer),
    `include_soft_deleted` gated, scoped by the existing list/search
    filter; **every export audited** (actor, filter, format, count,
    profile) — even a zero-row export.
  - **Acceptance:** integration tests cover idempotent re-import (same
    file → same state, no duplicates), per-row error report (one invalid
    row skipped + reported, valid rows commit), keyless-row dedupe → review
    queue with `provenance = import`, masked vs full export (full requires
    elevated role), and that an export — including a zero-row export —
    writes an audit record.

