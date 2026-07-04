## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [ ] **T-1 — FHIR R5 mapping decision + implementation.**
  - [ ] Decide Encounter vs Appointment vs other event-pattern (OQ-1).
  - [ ] Implement bidirectional conversion for the chosen resource.
  - **Acceptance:** `POST /fhir/Event` round-trips through the chosen
    resource; OperationOutcome on errors.
- [ ] **T-2 — Time-zone-aware fuzzy matching.**
  - [ ] Replace naive UTC offsets with `chrono-tz` conversions in the
    date-proximity scorer.
  - **Acceptance:** unit test where one event in `America/New_York`
    matches another in `UTC` at the same wall-clock instant.
- [ ] **T-3 — RFC 5545 RRULE recurrence support.**
  - [ ] Add `recurrence_rule: Option<String>` to `Event`.
  - [ ] Implement expansion for search + dedup.
  - **Acceptance:** weekly RRULE expanded into 52 occurrences for
    range queries.
- [ ] **T-4 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag.
  - **Acceptance:** integration test publishes an `EventCreated`
    record end-to-end.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [ ] **T-6 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `EventService.GetEvent`
    round-trips a record.
- [ ] **T-7 — iCalendar import / export.**
  - [ ] `POST /api/v1/events/import.ics`, `GET /api/v1/events/{id}.ics`.
  - **Acceptance:** Apple Calendar imports the exported `.ics`
    without warnings.
- [ ] **T-8 — Authentication / authorisation.**
  - [x] Offline PASETO v4.public verification. *(done 2026-07-04)* Per
    [authentication-sessions](../../../agents/share/authentication-sessions.md)
    §5/§9:
    - [x] `authentication-verifier` 0.2 (path dep; PASETO-only) added.
    - [x] [`AuthUser`] extractor + `GET /api/v1/whoami` verify PASETO
      `v4.public` (Ed25519) bearer tokens offline — signature, footer
      `kid`, `iss`, `aud`, `exp` — via `bearer_claims` in
      `src/api/rest/auth.rs`.
    - [x] Verifier built from env at boot (`EVENT_PASETO_KEYS` key set
      as published at `/.well-known/paseto-keys`;
      `EVENT_TOKEN_ISSUER` / `EVENT_TOKEN_AUDIENCE`, defaults
      `authentication-service` / `main-x-service`); absent key set ⇒
      empty set, every token rejected, service still boots.
    - **Acceptance:** DB-free unit tests in `src/api/rest/auth.rs` mint
      `v4.public` tokens in-process (throwaway Ed25519 key) and pin
      valid / missing / non-bearer / expired / tampered / no-key
      outcomes. Met: `cargo test --lib` green.
  - [x] Blanket enforcement middleware on `/api/v1/*` *(done
    2026-07-04)* — env-gated by `EVENT_REQUIRE_AUTH`, **default off**
    (`1`/`true`/`yes`/`on` case-insensitive ⇒ on; unset/blank/junk ⇒
    off; read once at `AppState` construction — restart to change).
    The pure `auth::enforce` decision + `auth::require_auth_mw`
    middleware require a valid PASETO bearer token on every
    `/api/v1/*` route except the public allow-list `/api/v1/health`
    (`auth::PUBLIC_API_PATHS`); root-level `/_health`, `/_ping`,
    `/api-docs/openapi.json`, `/swagger-ui*`, `/metrics.prom`, and the
    `/fhir/*` `501` stubs are outside the `/api/v1` scope and stay
    public. Wired on both router surfaces (`create_router` and the
    loco router in `App::after_routes`) via
    `axum::middleware::from_fn_with_state`, inside the CORS layer.
    Family contract:
    [jwt-enforcement](../../../agents/share/jwt-enforcement.md).
    - **Acceptance (enforcement middleware — met):** DB-free unit
      tests in `src/api/rest/auth.rs` pin the `enforce` matrix — off +
      no token ⇒ pass; on + public/out-of-scope paths (incl. `/fhir/*`)
      ⇒ pass; on + protected + no token ⇒ `401`; on + valid ⇒ pass;
      on + expired / tampered ⇒ `401` — plus the lenient `parse_bool`
      flag parser. Met: `cargo test --lib` green.
  - [ ] Scheduler / admin / read-only / service roles.
  - [ ] Keys fetched from the authentication-service
    `/.well-known/paseto-keys` at boot (today the key set is injected
    via `EVENT_PASETO_KEYS`).
  - **Acceptance (roles — open):** valid token + role gets `2xx`;
    insufficient role gets `403`.
- [ ] **T-9 — Bulk import / export.** (§9.1, §10.3;
  [bulk import/export](../../../agents/share/bulk-import-export.md))
  - [ ] `bulk_jobs` migration (family-wide schema, shared doc §3).
  - [ ] The five endpoints (shared doc §4) under `/api/v1/events/*`:
    `POST/GET import`, `POST/GET export`, `GET bulk-jobs`.
  - [ ] `bg_pg` worker draining `queued → running →
    completed|completed_with_errors|failed` with progress updates.
  - [ ] JSONL (lossless reference), CSV (flattening per §9.1), and
    feature-gated Parquet (export-first) codecs.
  - [ ] Per-row pipeline reusing the single-create validators + the
    event matcher + the review queue; upsert by stable key
    (scheme-scoped `event_ids` `(scheme, value)` pair or event `id`),
    keyless rows → duplicate detection → review queue with
    `provenance = import`.
  - [ ] Downloadable per-row error report
    (`row_number, source_line, field, code, message`).
  - [ ] Export masking profiles + `include_soft_deleted` gating +
    per-export audit (written even for a zero-row export).
  - **Acceptance:** integration tests cover idempotent re-import
    (re-submitting a file is a no-op), per-row error report,
    dedupe-to-review (`provenance = import`), masked vs full export,
    and the export audit row.

