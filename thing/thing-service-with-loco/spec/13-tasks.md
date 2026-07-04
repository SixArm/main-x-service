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
- [ ] **T-4 — Authentication / authorisation.** Peer PASETO
  verification *(done 2026-07-04)* and default-off blanket enforcement
  *(done 2026-07-04)*; roles + published-key HTTP fetch still open. Per
  [authentication-sessions](../../../agents/share/authentication-sessions.md)
  §5: the family moved off RS256-JWT + JWKS.
  - [x] Offline PASETO `v4.public` (Ed25519) verification via the
    `authentication-verifier` crate 0.2 (path dep; key set as
    published at the authentication-service
    `/.well-known/paseto-keys`): `AuthUser` extractor + `GET
    /api/whoami` verify bearer tokens offline — signature, footer
    `kid`, `iss`, `aud`, `exp` — via `bearer_claims` in
    `src/api/rest/auth.rs`.
  - [x] Verifier built from env at boot (`THING_PASETO_KEYS` key set
    as published at `/.well-known/paseto-keys`; `THING_TOKEN_ISSUER` /
    `THING_TOKEN_AUDIENCE`, defaults `authentication-service` /
    `main-x-service`); absent key set ⇒ empty set, every token
    rejected, service still boots.
  - [x] Blanket enforcement middleware on `/api/*` *(done 2026-07-04)*
    — env-gated by `THING_REQUIRE_AUTH`, **default off**
    (`1`/`true`/`yes`/`on` case-insensitive ⇒ on; unset/blank/junk ⇒
    off; read once at `AppState` construction — restart to change).
    The pure `auth::enforce` decision + `auth::require_auth_mw`
    middleware require a valid PASETO bearer token on every `/api/*`
    route except the public allow-list `/api/health`
    (`auth::PUBLIC_API_PATHS`); root-level `/_health`, `/_ping`,
    `/api-docs/openapi.json`, `/swagger-ui*`, and `/metrics.prom` are
    outside the `/api` scope and stay public. Wired on both router
    surfaces (`create_router` and the loco router in
    `App::after_routes`) via `axum::middleware::from_fn_with_state`,
    inside the CORS layer. Family contract:
    [jwt-enforcement](../../../agents/share/jwt-enforcement.md).
  - [ ] Editor / read-only / service roles.
  - [ ] Fetch the published Ed25519 key set over HTTP at boot (today:
    `THING_PASETO_KEYS` env injection).
  - **Acceptance (verification — met):** DB-free unit tests in
    `src/api/rest/auth.rs` mint `v4.public` tokens in-process
    (throwaway Ed25519 key) and pin valid / missing / non-bearer /
    expired / tampered / no-key outcomes. Met: `cargo test --lib`
    green.
  - **Acceptance (enforcement middleware — met):** DB-free unit tests
    in `src/api/rest/auth.rs` pin the `enforce` matrix — off + no
    token ⇒ pass; on + public/out-of-scope paths ⇒ pass; on +
    protected + no token ⇒ `401`; on + valid ⇒ pass; on + expired /
    tampered ⇒ `401` — plus the lenient `parse_bool` flag parser. Met:
    `cargo test --lib` green.
  - **Acceptance (roles — open):** valid token + role gets `2xx`;
    insufficient role gets `403`.
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
- [ ] **T-8 — Bulk import / export.**
  - [ ] `bulk_jobs` migration (per
    [`../../../agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md)
    §3).
  - [ ] The five endpoints (§4 of the shared doc): `POST/GET
    /api/things/import`, `POST/GET /api/things/export`, `GET
    /api/things/bulk-jobs`.
  - [ ] `bg_pg` worker draining `queued → running →
    completed | completed_with_errors | failed`.
  - [ ] JSONL (reference, lossless), CSV (flattening per §10.3),
    Parquet (feature-gated, export-first) codecs.
  - [ ] Per-row pipeline reusing the single-create validators +
    `ThingMatcher` + review queue — upsert on a deterministic
    `(property_id, value)` / `pid` stable key (§10.3), else
    duplicate-detect → review queue with `provenance = import`.
  - [ ] Downloadable per-row error report
    (`row_number, source_line, field, code, message`).
  - [ ] Export masking profile + `include_soft_deleted` gating +
    per-export audit row (written even for a zero-row export).
  - **Acceptance:** tests cover idempotent re-import (same file ⇒ same
    state), per-row error report, keyless-row dedupe-to-review,
    masked vs full export, and the export audit row.
- [x] **T-7 — Expose Prometheus metrics endpoint.**
  - [x] Add a `handlers::metrics_prom` handler rendering
    `thing::metrics::METRICS` as `text/plain; version=0.0.4`.
  - [x] Mount it at the application **root** `/metrics.prom` (not
    under `/api`) via `api::rest::metrics_routes` (registered in
    `App::routes`) and on the hand-written `create_router`; add it to
    the `OpenAPI` document under an `observability` tag.
  - **Acceptance:** DB-free tests pin the `/metrics.prom` `OpenAPI`
    path and the root loco-route binding (`api::rest::tests`); the
    registry render test lives in `metrics::tests`.

