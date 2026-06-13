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

