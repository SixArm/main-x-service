## 13. Tasks

Spec-driven work breakdown. Each task has an acceptance criterion;
tick the box when an automated test or clearly described manual check
confirms the criterion is met. Tasks small enough to land in a single
PR; split larger tasks (`T-12a`, `T-12b`).

- [ ] **T-1 — Wire JWT middleware on `/api/*`.**
  - [ ] Add `jsonwebtoken` validator extractor.
  - [ ] Reject unauthenticated requests with `401`.
  - **Acceptance:** integration test posts without a token → `401`;
    posts with a valid signed token → `2xx`.
- [ ] **T-2 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag `fluvio`.
  - [ ] Document failover behaviour when the broker is unreachable.
  - **Acceptance:** integration test against a local Fluvio broker
    publishes a `PersonCreated` event end-to-end.
- [ ] **T-3 — Complete FHIR bundle handling.**
  - [ ] `Bundle` GET / POST / search wrapping.
  - [ ] OperationOutcome on malformed bundles.
  - **Acceptance:** Touchstone FHIR validator passes on a sample
    bundle round-trip.
- [ ] **T-4 — FHIR capability statement endpoint.**
  - [ ] `GET /fhir/metadata` returns a CapabilityStatement listing
    supported resources + interactions.
  - **Acceptance:** schema check against R5 CapabilityStatement.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [ ] **T-6 — gRPC implementation.**
  - [ ] Promote stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `PersonService.GetPerson`
    round-trips a record.
- [ ] **T-7 — Spec-drift CI check.**
  - [ ] Fail PR if `src/matching/**` or `src/models/person.rs`
    changes without a `spec.md` edit (allowlist in `.spec-allow`).
  - **Acceptance:** `bash scripts/spec-drift-check.sh main HEAD`
    exits non-zero on a code-only PR.
- [ ] **T-8 — `db::audit` rename clean-up.**
  - [ ] Verify no -era symbols remain in `src/db/audit.rs`.
  - **Acceptance:** `cargo check --lib` passes clean; legacy
    domain-specific symbols (e.g. `patient`, `mpi`) absent from
    `src/db/`.

