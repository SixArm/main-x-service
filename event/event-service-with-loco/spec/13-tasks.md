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
  - [x] ABAC authorization *(done 2026-07-05; supersedes the earlier
    roles/RBAC sketch — scheduler / admin / read-only / service — per
    [authorization-attributes](../../../agents/share/authorization-attributes.md))*
    — inside the blanket guard (so only when `EVENT_REQUIRE_AUTH` is
    on), a verified token's `attrs` claim is evaluated by the shared
    engine in `authentication-verifier` 0.3: the action is derived
    from the HTTP method + this crate's destructive named POSTs
    (`auth::DESTRUCTIVE_POST_SUFFIXES`: `/merge`, `/deduplicate`,
    `/import`), and the policy — `EVENT_ABAC_POLICY` (inline JSON) /
    `EVENT_ABAC_POLICY_FILE` (path), unset/unparsable ⇒ warn-log +
    built-in default policy, read once at `AppState` construction —
    decides first-match-wins with default allow-read / deny-mutation.
    `401` = missing/bad credential; `403` = valid credential, policy
    denied (body carries the deciding rule). Acceptance met: DB-free
    unit tests in `src/api/rest/auth.rs` pin the §7 matrix — action
    derivation; empty `attrs` ⇒ GET ok / POST 403; `access=write` ⇒
    POST/PUT ok, DELETE + merge 403; `access=admin` ⇒ destructive ok;
    `svc=true` ⇒ everything; configured deny beats later allow;
    401-vs-403 split; bad policy JSON falls back to the default —
    `cargo test --lib` green.
  - [x] Keys fetched from the authentication-service
    `/.well-known/paseto-keys` at boot *(done 2026-07-04)* — when the
    new `EVENT_PASETO_KEYS_URL` env var is set (non-blank),
    `App::after_routes` (async boot context) calls
    `state::boot_verifier`, which fetches the key-set JSON once via
    `Verifier::from_paseto_keys_url` (the `authentication-verifier`
    `fetch` feature, now enabled on the path dep). A successful fetch
    **wins** over any `EVENT_PASETO_KEYS` env value (info-logged with
    the source URL); any fetch failure warn-logs and falls back to the
    env path (else the empty reject-all set) — the service **always
    boots**. Unset/blank URL ⇒ prior behaviour exactly. The fetched
    verifier is installed via `AppState::with_verifier` **before** the
    shared-store insert and the `require_auth_mw` middleware capture
    the state, so both router surfaces consult the fetched key set.
    Fetch happens once at boot; no refresh loop (rotation re-fetch is
    roadmap — §15).
    - **Acceptance (boot-time key fetch — met):** DB-free tokio tests
      in `src/api/rest/auth.rs` — a local ephemeral-port HTTP listener
      serves the in-process key set and the fetch-built verifier
      accepts a token signed by that key; a fast-failing URL
      (`http://127.0.0.1:1/`) falls back to the env/empty path without
      panic. Met: `cargo test --lib` green.
  - **Acceptance (met):** valid token whose attributes satisfy the
    policy gets `2xx`; a valid token the policy denies gets `403`;
    no/bad token gets `401`. T-8 is complete; activation
    (`EVENT_REQUIRE_AUTH=1`) remains the operational decision.
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

