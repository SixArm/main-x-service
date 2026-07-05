## 13. Tasks

Spec-driven work breakdown. Each task has an acceptance criterion;
tick the box when an automated test or clearly described manual check
confirms the criterion is met. Tasks small enough to land in a single
PR; split larger tasks (`T-12a`, `T-12b`).

- [x] **T-1a — Flip peer verification to PASETO v4.public.** *(done
  2026-07-04)* Per
  [authentication-sessions.md](../../../agents/share/authentication-sessions.md)
  §5/§9: the family moved off RS256-JWT + JWKS.
  - [x] `authentication-verifier` 0.2 (path dep; PASETO-only) replaces
    the crates.io 0.1 RS256 version; direct `jsonwebtoken` dep dropped.
  - [x] [`AuthUser`] extractor + `GET /api/whoami` verify PASETO
    `v4.public` (Ed25519) bearer tokens offline — signature, footer
    `kid`, `iss`, `aud`, `exp` — via `bearer_claims` in
    `src/api/rest/auth.rs`.
  - [x] Verifier built from env at boot (`PERSON_PASETO_KEYS` key set as
    published at `/.well-known/paseto-keys`; `PERSON_TOKEN_ISSUER` /
    `PERSON_TOKEN_AUDIENCE`, defaults `authentication-service` /
    `main-x-service`); absent key set ⇒ empty set, every token rejected,
    service still boots.
  - **Acceptance:** DB-free unit tests in `src/api/rest/auth.rs` mint
    `v4.public` tokens in-process (throwaway Ed25519 key) and pin
    valid / missing / non-bearer / expired / tampered / no-key
    outcomes. Met: `cargo test --lib` green.
- [x] **T-1b — Blanket auth enforcement on `/api/*`.** *(done
  2026-07-04; remainders split to T-1c)*
  - [x] Require a valid PASETO bearer token on every route except the
    public allow-list (`/api/health`, loco `/_health` / `/_ping`,
    `/api-docs/openapi.json`, `/swagger-ui*`, `/metrics.prom`), gated
    by a default-off `PERSON_REQUIRE_AUTH` env flag with lenient
    parsing (`1`/`true`/`yes`/`on` ⇒ on; unset/blank/junk ⇒ off;
    family contract: `agents/share/jwt-enforcement.md`). Pure
    `auth::enforce` decision + `Enforcement` middleware state in
    `src/api/rest/auth.rs`, layered unconditionally on **both** router
    surfaces (`create_router` and the loco `after_routes` hook); the
    flag is snapshotted at router construction, so changing it
    requires a restart.
  - **Acceptance:** DB-free unit tests in `src/api/rest/auth.rs`
    (reusing the T-1a in-process token minting) pin the full
    enforcement matrix — off + no token ⇒ Ok; on + each public path ⇒
    Ok; on + protected + no token ⇒ `401`; on + protected + valid ⇒
    Ok; on + expired/tampered ⇒ `401` — plus the flag-parser
    semantics. Met: `cargo test --lib` green.
- [ ] **T-1c — Auth follow-ups: boot-time key fetch + roles/RBAC.**
  - [x] Fetch the key set over HTTP from the auth service at boot
    *(done 2026-07-04)*: new `PERSON_PASETO_KEYS_URL` env var —
    unset/blank ⇒ the `PERSON_PASETO_KEYS` env path exactly as before;
    set ⇒ fetch once at boot in `after_routes` via
    `Verifier::from_paseto_keys_url` (verifier `fetch` feature); on
    success the fetched key set **wins** over `PERSON_PASETO_KEYS`
    (`tracing::info!`); on any fetch failure `tracing::warn!` and fall
    back to the env path — the service **always boots**. Swapped into
    `AppState` via `with_verifier` **before** the enforcement
    middleware and shared-store state are built, so both router
    surfaces verify against it. One-shot fetch; no refresh loop
    (periodic refresh is a §15 roadmap note). Pinned by DB-free tokio
    tests in `src/api/rest/auth.rs`: fetch from a local ephemeral-port
    listener serving the in-process key set (minted token verifies),
    fallback on a dead port (no panic, token rejected), and the
    URL-unset ⇒ env-path precedence.
  - [ ] Roles / RBAC on top of the verified claims (`roles` / `scope`).
  - [ ] DB-gated request test (`#[ignore]`, Postgres): with
    `PERSON_REQUIRE_AUTH` set, an unauthenticated `GET /api/persons/…`
    returns `401` while `GET /api-docs/openapi.json` stays `200`.
  - **Acceptance:** integration test with enforcement on posts without
    a token → `401`; posts with a valid token → `2xx`. Key-set fetch
    from a stub auth service at boot: **met** via the local-listener
    tokio tests above (`cargo test --lib` green).
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
- [x] **T-8 — `db::audit` rename clean-up.** *(done 2026-06-15)*
  - [x] Verify no -era symbols remain in `src/db/audit.rs`.
  - **Acceptance:** `cargo check --lib` passes clean; legacy
    domain-specific symbols (e.g. `patient`, `mpi`) absent from
    `src/db/`. Met: a grep of `src/db/` finds zero `patient` / `mpi`
    symbols and `cargo check --lib` is clean.
- [ ] **T-9 — Cross-service entity links (write side).**
  See §5.4, §8.6, §9.1, §10.4 and
  [cross-service linking](../../../agents/share/cross-service-linking.md).
  - [ ] Migration creating the `entity_links` table (§10.4 schema, with
    the `UNIQUE (from_pid, kind, to_ref, valid_from)` upsert key).
  - [ ] `EntityRef` value type (parse / `Display` + `entity_type → service`
    map), copied per project (drift-accepted).
  - [ ] Link endpoints: `POST` / `GET` / `DELETE`
    `/api/v1/persons/{pid}/links`; create/upsert is optimistic (no
    cross-service call) and supports `same_identity` (person ↔ worker)
    and `works_at` / `member_of` (person → organization, temporal).
  - [ ] Emit `linked` / `unlinked` events on the existing event
    envelope via `EventProducer` (edge detail in `data`; no new transport).
  - [ ] Partition guard in `src/matching/adapter.rs`: `entity_links` are
    never projected into the matcher input.
  - **Acceptance:** integration test creates a `works_at` link
    (`2xx`, `linked` event published, row in `entity_links`), lists it
    via `GET`, deletes it (`unlinked` event, `deleted_at` set); a matcher
    unit test asserts an `entity_links` row never alters a match score.
- [ ] **T-10 — Bulk import / export.**
  See §9.2, §10.5 and
  [bulk import/export](../../../agents/share/bulk-import-export.md).
  - [ ] Migration creating the `bulk_jobs` table (shared doc §3 schema,
    with the `UNIQUE (entity, kind, idempotency_key)` key).
  - [ ] The five endpoints (§9.2): `POST`/`GET` `/api/v1/persons/import`,
    `POST`/`GET` `/api/v1/persons/export`, `GET /api/v1/persons/bulk-jobs`.
  - [ ] `bg_pg` worker draining jobs `queued → running →
    completed | completed_with_errors | failed`, with progress updates.
  - [ ] JSONL (lossless reference) + CSV (flattening per §9.2: dotted
    single-nested, JSON-in-cell arrays) codecs; Parquet **export-only**,
    feature-gated.
  - [ ] Per-row pipeline reusing the single-create validators + matcher +
    review queue: upsert by stable key (national/health identifier or
    `pid`, §9.2); keyless / unmatched rows → duplicate detection →
    review queue with `provenance = import`; events + audit not bypassed.
  - [ ] Downloadable per-row error report
    (`row_number, source_line, field, code, message`); one bad row never
    aborts the load; counts reconcile
    (`rows_total = created + upserted + to_review + errored`).
  - [ ] Export masking + audit: `masking_profile` (masked default, full
    gated), `include_soft_deleted` gated, every export audited (even
    zero-row); single-record GDPR export becomes the `filter = one pid`
    special case.
  - **Acceptance:** integration tests cover idempotent re-import (same
    file re-upserts to the same state), the per-row error report, a
    keyless dedupe-to-review row (`provenance = import`), masked vs full
    export, and that a zero-row export still writes an audit record.

