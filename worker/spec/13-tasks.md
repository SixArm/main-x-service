## 13. Tasks

Entity-level work queue: items that span subprojects or police the
seams. Crate-internal work belongs in the owning subproject's queue
([service §13](../worker-service-with-loco/spec/13-tasks.md),
[matcher §23](../worker-matcher-rust-crate/spec/23-tasks-and-acceptance-criteria.md),
[front-end §13](../worker-front-end-with-svelte/spec/13-tasks.md)).
Tick the box when an automated test or clearly described manual check
confirms the acceptance criterion.

- [x] **T-1 — Reconcile the FHIR resource path discrepancy.** *(Done
  2026-06-13.)*
  - [x] Service [spec §6.8 / §9](../worker-service-with-loco/spec/09-api-surface.md)
    say `/fhir/Practitioner`; service
    [`AGENTS/restful.md`](../worker-service-with-loco/AGENTS/restful.md)
    documents `/fhir/Worker/{id}`. Determine which the code serves,
    fix the loser. *(Done 2026-06-13: the code's handlers and wire
    `resourceType` are `Worker` / `/fhir/Worker` — the spec was the
    loser; §2/§6.8/§8/§9/§12/§13/§14 now say `/fhir/Worker`.)*
  - [x] Pin with a route test. *(Done 2026-06-13: the previously
    unmounted FHIR handlers are now registered via `fhir_routes()` in
    `App::routes` — service [§13 T-9](../worker-service-with-loco/spec/13-tasks.md)
    — and pinned by `tests/api_integration_test.rs::test_fhir_worker_route_is_mounted`
    (un-gated) plus `::test_fhir_worker_not_found_returns_operation_outcome`
    (DB-gated).)*
  - **Acceptance:** spec, AGENTS doc, and an integration test agree on
    one path. ✓
- [x] **T-2 — Refresh the matcher spec banner version.** *(Done
  2026-06-13.)*
  - [x] [matcher `spec/index.md`](../worker-matcher-rust-crate/spec/index.md)
    banner says `Version: 0.3.0`; `Cargo.toml` says `0.6.1`. *(Banner
    now 0.6.1; the stale "(0.6.0)" dependency-list label in
    `AGENTS/release.md` was also refreshed.)*
  - **Acceptance:** banner matches `Cargo.toml` ✓; release checklist in
    [matcher `AGENTS/release.md`](../worker-matcher-rust-crate/AGENTS/release.md)
    gains a banner-bump step ✓.
- [ ] **T-3 — Full-trio end-to-end test (seam 2, §11.3).**
  - [ ] Compose recipe that starts PostgreSQL + service, then runs a
    Playwright suite against the real API (create → 409 surface →
    detail → merge → audit).
  - **Acceptance:** one CI-runnable command exercises front-end →
    service → matcher → database and passes.
- [ ] **T-4 — SSO wiring across the trio.**
  - [ ] Blocked on service §13 T-1 (JWT middleware).
  - [ ] Front-end: bearer-token header in `ApiClient`; sign-in
    redirect to the [authentication front-end](../../authentication/authentication-front-end-with-svelte/).
  - [ ] Service: JWKS fetch + cache from the authentication service.
  - **Acceptance:** unauthenticated UI call gets `401` and the UI
    redirects to sign-in; authenticated round-trip succeeds.
- [ ] **T-5 — Surface the privacy endpoints in the operator UI.**
  - [ ] Masked-view toggle on detail (front-end T-19) and GDPR-export
    download (front-end T-20) close the §12.2 operator gap.
  - **Acceptance:** detail page can render `/masked` data; export
    button downloads the `/export` JSON.
- [ ] **T-6 — Keep the entity-root schema snapshot honest.**
  - [ ] [`worker-service-schema.sql`](../worker-service-schema.sql)
    has no documented regeneration step; add one (script or Make
    target) and note it in §10.1.
  - **Finding (2026-06-13):** drift is no longer hypothetical —
    §10.1.1 now documents that the snapshot is missing seven
    migration-created tables (`postcode_geography` + the six
    reference/codesystem tables) and carries one table
    (`worker_consents`) that no migration creates. Still no
    regeneration command; task stays open.
  - **Acceptance:** documented command regenerates the file from the
    migrations; CI or checklist flags drift.
- [x] **T-7 — Decide the ODS identifier's matcher-side fate.** *(Done
  2026-06-13.)*
  - [x] `IdentifierType::ODS` falls through the adapter unmapped
    (§5.3). Either propose an ODS parser to the matcher crate
    (its §23) or record the fall-through as permanent in both specs.
    *(Decision: permanent fall-through, recorded on both sides.)*
  - **Finding (2026-06-13):** the matcher has **no suitable scheme**
    — all 42 identifier slots are person-level national schemes,
    while an ODS code identifies an organisation/site shared by every
    worker at the same practice (an exact-match short-circuit would
    declare colleagues the same person); the matcher's `local_id` is
    deliberately never scored. The fall-through is recorded with
    rationale in service
    [spec §6.2](../worker-service-with-loco/spec/06-functional-requirements.md)
    and the adapter's routing comment, and pinned by two bridge tests
    (`ods_organisation_code_falls_through_unmapped`,
    `shared_ods_code_does_not_make_different_workers_match`).
  - **Second leg (2026-06-13):** the matcher side now formally declares
    organisation-level identifiers permanently out of scope —
    [matcher spec §2](../worker-matcher-rust-crate/spec/02-scope.md)
    gains an "Out of scope (permanently): organisation-level
    identifiers" paragraph, and two matcher integration tests (§16a,
    `test_local_id_difference_does_not_lower_score`,
    `test_shared_local_id_adds_no_signal_between_unrelated_workers`)
    pin that the unscored `local_id` field — where such a code would
    live — adds no signal in either direction. The decision is thus
    recorded and test-backed on **both** sides of the seam.
  - **Acceptance:** decision recorded; if mapped, a bridge test pins
    the routing. ✓ *(Recorded as permanent on both specs; bridge tests
    pin the service-side fall-through and matcher-side non-scoring.)*
- [ ] **T-8 — Close the front-end verification gaps.**
  - [ ] Front-end [§14](../worker-front-end-with-svelte/spec/14-implementation-status.md)
    still lists `pnpm install` / `pnpm test` verification and the
    live operator walkthrough as pending.
  - **Acceptance:** front-end §14 rows flip to ✅ with the run output
    noted in its CHANGELOG.
