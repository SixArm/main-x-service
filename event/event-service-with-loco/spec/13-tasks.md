## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [x] **SEC-M1 (security): input-size caps on the `Event` payload.**
  `validate_event` bounds scalar text (`MAX_TEXT_LEN = 1024`), string-array
  cardinality + per-entry (`MAX_ARRAY_LEN = 256` / `MAX_ITEM_LEN = 512`), and
  the inner text + cardinality of the nested object arrays (`identifiers`,
  `location` union, the six party lists, references, `offers`, `sub_events`,
  `links`) → field-scoped `422` before persist/match, closing the O(n·m)
  matcher `DoS`. `price_currency`/`in_language` keep stricter bounds;
  time-window checks untouched. Factored into `event_size_caps`/`cap_*`. Unit
  tested. (Repo tasks.md Phase 5 SEC-M1.)

- [x] **T-1 — FHIR R5 mapping decision + implementation.**
  *(done via T-10, 2026-07-07 — superseded)*
  - [x] Decide Encounter vs Appointment vs other event-pattern (OQ-1):
    **`Appointment`** chosen as the default (`Encounter` roadmap).
  - [x] Implement bidirectional conversion for the chosen resource.
  - **Acceptance:** `POST /fhir/Appointment` round-trips through the
    `Appointment` resource; `OperationOutcome` on errors. See T-10.
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
  - [ ] `POST /api/events/import.ics`, `GET /api/events/{id}.ics`.
  - **Acceptance:** Apple Calendar imports the exported `.ics`
    without warnings.
- [ ] **T-8 — Authentication / authorisation.**
  - [x] Offline PASETO v4.public verification. *(done 2026-07-04)* Per
    [authentication-sessions](../../../agents/share/authentication-sessions.md)
    §5/§9:
    - [x] `authentication-verifier` 0.2 (path dep; PASETO-only) added.
    - [x] [`AuthUser`] extractor + `GET /api/whoami` verify PASETO
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
  - [x] Blanket enforcement middleware on `/api/*` *(done
    2026-07-04)* — env-gated by `EVENT_REQUIRE_AUTH`, **default off**
    (`1`/`true`/`yes`/`on` case-insensitive ⇒ on; unset/blank/junk ⇒
    off; read once at `AppState` construction — restart to change).
    The pure `auth::enforce` decision + `auth::require_auth_mw`
    middleware require a valid PASETO bearer token on every
    `/api/*` **and** `/fhir/*` route except the public allow-list
    `/api/health` and `/fhir/metadata` (`auth::PUBLIC_API_PATHS`);
    root-level `/_health`, `/_ping`, `/api-docs/openapi.json`,
    `/swagger-ui*`, and `/metrics.prom` are outside the enforced scope
    and stay public. Wired on both router surfaces (`create_router` and the
    loco router in `App::after_routes`) via
    `axum::middleware::from_fn_with_state`, inside the CORS layer.
    Family contract:
    [jwt-enforcement](../../../agents/share/jwt-enforcement.md).
    - **Acceptance (enforcement middleware — met):** DB-free unit
      tests in `src/api/rest/auth.rs` pin the `enforce` matrix — off +
      no token ⇒ pass; on + public/out-of-scope paths (incl.
      `/fhir/metadata`) ⇒ pass; on + protected `/fhir/*` + no token ⇒
      `401`; on + protected + no token ⇒ `401`; on + valid ⇒ pass;
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
  - [ ] The five endpoints (shared doc §4) under `/api/events/*`:
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
- [x] **T-10 — FHIR R5 API** (`Appointment` default; `Encounter`
  roadmap) — adopt the family contract
  ([`agents/share/fhir.md`](../../../agents/share/fhir.md)),
  which replaced the former unmapped FHIR placeholder. **Best-effort
  mapping** (§3, `low` fidelity — schema.org/Event has no clean FHIR
  analog): map the stored `event_matcher` DTO to a FHIR
  **`Appointment`** — `start_date`/`end_date` → `start`/`end`;
  `name` → `description`; `event_status` → `status`; parties
  (`organizers`/`performers`/`attendees`) → `participant`;
  `location` → a contained/`reference`; `identifiers` →
  `identifier`. `Encounter` is a roadmap alternative (resolves
  OQ-1 / T-1 for the default resource).
  - [x] New `src/fhir/` module: resource structs, `to_fhir_appointment`
    / `from_fhir_appointment` over the stored DTO, `FhirOperationOutcome`,
    searchset `Bundle`, and search-param parsing.
  - [x] Mounted `src/controllers/fhir.rs` (`routes()` added in
    `app.rs::routes()`): read / create / update / delete / search at
    `/fhir/Appointment{,/{id}}` + `GET /fhir/metadata`
    `CapabilityStatement` that honestly declares the partial,
    best-effort surface.
  - [x] Reuse the native model helpers, validators, event/audit paths,
    and the blanket auth + ABAC guard (§8): `/fhir/*` is guarded (not on
    the public allow-list), the action derived from the HTTP method.
  - [x] Supported search params: `_id`, `_lastUpdated`, `_count`,
    `identifier`, `status`, `date`.
  - **Acceptance:** DTO↔`Appointment` round-trip, each interaction
    (read/create/update/delete/search), search → `Bundle`,
    `OperationOutcome` on 404/400/422, and `CapabilityStatement`
    matches the mounted routes.
  - **Done (2026-07-07):** copy-adapted the organization reference into
    `src/fhir/{resources,mod,search}.rs` + `src/controllers/fhir.rs`
    (loco `routes()` wired in `app.rs::routes()`, replacing the former
    placeholder FHIR routes; the old prototype FHIR module under
    `src/api/` was deleted). Writes go through the
    native `EventRepository` (audit + event stream) + Tantivy index like
    the REST handlers. Identifier category ↔ `urn:mxi:event:*` system,
    `EventStatus` ↔ `Appointment.status`, and party-role participant
    codings all round-trip. DB-free unit tests (scheme/status round-trip,
    DTO↔resource round-trip, missing `description`/`start` rejected,
    search predicates) pass; `cargo test --lib` = 108 passed,
    `cargo clippy --lib` clean. **Documented gaps** (`low` fidelity):
    event `description`/`keywords`/`image`/`same_as`/`url`/`offers`/
    capacity/audience/`sponsors`/`funders`/`contributors`/`about`/
    `works`/`super_event`/`sub_events`/`door_time`/`duration`/
    `time_zone` and per-party `email`/`url` are not emitted; locations
    survive only as a display label (`Location::Text` on the way back);
    `MovedOnline`/`Rescheduled` fold onto `booked`. `Encounter` remains a
    roadmap alternative.
- [x] **T-11 — Durable event bus (transactional outbox + relay).** Adopt
  the family contract ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md)).
  - [x] **Phase 2 (transactional outbox).** `event_outbox` table + SeaORM
    entity (`db::models::event_outbox`); the canonical `Envelope` /
    `EventKind` / `EventView` + `EventTransport`/`transport()` selector
    (`src/streaming/envelope.rs`); `OutboxInsert` (pure envelope→row map +
    `insert_on` on a caller-supplied `ConnectionTrait`, so the entity write
    and its outbox row share one transaction) and the relay poll/ack seams
    (`Model::unpublished` / `mark_published`) in `src/db/outbox.rs`. Gated
    by `EVENT_EVENT_TRANSPORT` (default `memory` ⇒ ring buffer, today's
    behaviour; `outbox` ⇒ transactional outbox; unrecognised ⇒ `memory`,
    fail-safe; read once at boot).
  - [x] **Phase 3 (relay + retention).** *(done 2026-07-08)* `src/relay.rs`:
    the `EventSink` trait + default no-broker `LoggingSink`, `drain_once`
    (poll `unpublished` → `send` → `mark_published`, at-least-once, stops
    on first send failure to keep per-pid order) and `purge_published`
    (retention). A background loop (`relay::spawn`, started in
    `App::after_routes` alongside the auth/version layering) ticks every
    `EVENT_EVENT_RELAY_INTERVAL_SECS` (default 5, floored at 1) and purges
    every 60 ticks — **gated by `EVENT_EVENT_TRANSPORT=outbox` AND
    `EVENT_EVENT_RELAY`** (truthy `1`/`true`/`yes`/`on`), so it is a no-op
    by default. `EVENT_EVENT_RETENTION_DAYS` (default 7) is now **enforced**
    by `purge_published` (deletes rows with `published_at < now() -
    INTERVAL '<n> days'`). Copy-adapted from the organization reference
    (`src/relay.rs`), repathed to event's repository-based outbox
    (`db::outbox::Model` + `db::models::event_outbox`), `crate::Result`
    error type, `i64` ids, and `time::OffsetDateTime` retention cutoff.
  - **Broker-gated follow-up:** a real **`FluvioSink`** (`impl EventSink`
    behind a future `fluvio` cargo feature) is the only remaining piece;
    the `EventSink` seam means the drain loop + retention never change when
    it lands (see also T-4).
  - **Acceptance:** DB-free unit tests in `src/relay.rs` (logging-sink
    send, capturing-sink contract, config-parser defaults) pass; the drain
    poll/ack seams are DB-gated via the outbox suite. `cargo test --lib`
    green; `cargo clippy --lib --tests` clean. Default (no
    `EVENT_EVENT_TRANSPORT=outbox` + `EVENT_EVENT_RELAY`) ⇒ no relay loop.

