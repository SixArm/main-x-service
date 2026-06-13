## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [ ] **T-1 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag.
  - **Acceptance:** integration test publishes a `ThingCreated`
    record end-to-end.
- [ ] **T-2 — Introduce `ThingMatcher` trait.**
  - [ ] Promote `compute_match` to a trait so alternative scorers
    (ML-based, embedding-based) can plug in.
  - **Acceptance:** `ProbabilisticMatcher : ThingMatcher` compiles
    and behaves identically to today's free function.
- [ ] **T-3 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `ThingService.GetThing`
    round-trips a record.
- [ ] **T-4 — Authentication / authorisation.**
  - [ ] JWT middleware on `/api/*` with editor / read-only / service
    roles.
  - **Acceptance:** unauthenticated requests get `401`; valid token
    + role gets `2xx`.
- [ ] **T-5 — Embedding-based similarity (optional / experimental).**
  - [ ] Vector index via `pg_vector`.
  - [ ] `compute_match` augmented with cosine-similarity score.
  - **Acceptance:** A/B harness shows ≥ 2 % uplift on a labelled
    duplicate set.
- [ ] **T-6 — Spec-drift CI guard.**
  - [ ] Fail PR if `src/matching/**` or `src/models/thing.rs`
    changes without a `spec.md` edit.
  - **Acceptance:** `bash scripts/spec-drift-check.sh main HEAD`
    exits non-zero on a code-only PR.

