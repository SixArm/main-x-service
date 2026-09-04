## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [x] **2026-08-22 — Geo coordinates as exact decimals (`f64` →
  `BigDecimal`, `DOUBLE PRECISION` → `NUMERIC`).** `Place::latitude` /
  `Place::longitude` and `event_locations.latitude` / `.longitude`
  (migration `m20260822_000001_location_coordinates_to_numeric`). A
  coordinate is a decimal quantity: `DOUBLE PRECISION` cannot hold
  `37.87` (it holds `37.869999999999997`) and cannot distinguish it from
  `37.8700000000000001`. Forced by a real break, not preference — the
  repository adopted `serde_json`'s `arbitrary_precision`
  (`spec/serde-json-float-roundtrip-arbitrary-precision`), under which
  serde's `Content` buffer represents a number as a map, so an `f64`
  field inside an internally-tagged enum fails to deserialize. Both
  `Location` (`tag = "kind"`) and the bus envelope `EventEvent` (`tag =
  "event_type"`) are such enums; `POST /api/events` with coordinates and
  every bus consumer of an event with a venue position were affected.
  **Wire format deliberately unchanged** — the fields use
  `bigdecimal::impl_serde::arbitrary_precision_option`, so JSON stays a
  number (`"latitude":37.87`, `null` when absent) and the SvelteKit
  front-end (`number | null`) needs no change. Matching converts to
  `f64` at the Haversine boundary only. Adds `MAX_COORDINATE_SCALE`
  (10 places), replacing the digit bound `f64` used to provide by
  accident. **Acceptance:** coordinates serialize as JSON numbers, not
  strings; a fractional coordinate round-trips through both tagged
  enums; `37.8700000000000001` survives verbatim; absent stays `null`;
  range bounds inclusive; over-scale → `422`. §5.2.1, §5.3, §10.1.
  Verified: 159/159 unit + integration, DB-gated suite green against
  Postgres 18 with the column confirmed `numeric`, clippy `-D warnings`,
  fmt, MSRV 1.95, bench link, `cargo deny`.

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
- [x] **2026-07-28 — Keyed integrity verification (MAC + digests).**
  *Landed but never recorded here until this DOC-2 pass (2026-08-04)
  found the gap: shipped, tested, and reachable, with no `spec/13`
  entry, no `spec/14` row, and no `spec/09`/`agents/restful.md`
  endpoint listing.* Adds `src/compliance/` (`mac`, `record_integrity`,
  `audit_integrity`): SHA-256 + SHA3-256 digests and a keyed
  HMAC-SHA256 MAC (this crate's binding to the shared `integrity-mac`
  crate, HKDF-domain-separated per (service, domain)) over each `Event`
  record and each `audit_log` row. Two read endpoints, guarded like
  every other `/api` route: `GET /api/records/verify` and
  `GET /api/audit/verify`. **Default off**: with no
  `EVENT_INTEGRITY_MAC_KEY` (or `_KEY_FILE`) configured, no MAC is
  written and affected rows report `mac_absent` rather than a mismatch
  — adopting the control on a populated table must not produce false
  accusations. Env vars: `EVENT_INTEGRITY_MAC_KEY`,
  `EVENT_INTEGRITY_MAC_KEY_FILE` (takes precedence),
  `EVENT_INTEGRITY_MAC_KEY_ID`, `EVENT_INTEGRITY_MAC_KEYS_RETIRED`.
  **Known limit, stated in the module docs**: unlike person / worker /
  care-pathway / case, this crate has **no hash chain** (`prev_hash` /
  `hash`) and takes no external-witness checkpoint — a MAC proves a
  row's content is unchanged since it was written, and says nothing
  about a row **deleted wholesale**. See
  `agents/share/runbooks/integrity-activation.md` for the family-wide
  activation runbook.

- [x] **T-2 — Time-zone-aware fuzzy matching.**
  *(2026-09-03 — the literal ask, "replace naive UTC offsets … in the
  date-proximity scorer," did not describe a live defect: checked
  directly rather than assumed, `Event::start_date`/`end_date` are
  `DateTime<Utc>` — an absolute instant already resolved from whatever
  offset the input carried at the parse boundary
  (`chrono::DateTime::parse_from_rfc3339` + `.with_timezone(&Utc)`,
  used uniformly by the REST, gRPC, and FHIR intake paths;
  `Event::time_zone` is documented "storage is always UTC"). The
  scorer (`matching::algorithms::time_matching`) never sees a naive
  local time to get wrong. What was genuinely missing was the proof —
  no test exercised a cross-timezone instant, and nothing in the crate
  used `chrono-tz` at all.)*
  - [x] Prove the date-proximity scorer is timezone-correct against the
    real IANA tz database (`chrono-tz`, added as a dev-dependency),
    not merely a fixed-offset string.
  - **Acceptance:** unit test where one event in `America/New_York`
    matches another in `UTC` at the same wall-clock instant — met by
    `matching::algorithms::tests::cross_timezone_same_instant_matches_exactly`,
    which constructs a `2026-03-01 09:00 America/New_York` instant (EST,
    UTC−5, before that year's spring-forward) via `chrono-tz`, confirms
    it equals `2026-03-01 14:00 UTC`, and confirms `match_start_dates`
    scores the pair ≈1.0.
- [ ] **T-3 — RFC 5545 RRULE recurrence support.**
  - [ ] Add `recurrence_rule: Option<String>` to `Event`.
  - [ ] Implement expansion for search + dedup.
  - **Acceptance:** weekly RRULE expanded into 52 occurrences for
    range queries.
- [x] **T-4 — Production Fluvio publisher.** *(superseded by T-11,
  done 2026-08-03 — reworded during this DOC-2 pass; the literal ask
  below was never built as written)* T-4 as originally scoped meant
  implementing `FluvioEventPublisher : EventProducer` — a
  Fluvio-backed impl of the legacy ring-buffer `EventProducer` trait
  (`src/streaming/mod.rs`/`producer.rs`). That never happened: the
  crate instead solved the same underlying need — durable production
  delivery to Fluvio — via a different architecture, T-11's
  transactional outbox (`event_outbox` table, written in the same
  transaction as the entity change) + relay (`src/relay.rs`) +
  `EventSink` trait, with `FluvioSink` as the real-broker
  implementation (BUS-3, behind the `fluvio` Cargo feature, off by
  default). **`FluvioProducer`** (`src/streaming/producer.rs`) was
  therefore dead code: it still carried its original `todo!()` body,
  was not constructed anywhere (`AppState` only ever builds
  `InMemoryEventPublisher`), and was not reachable from any router.
  The follow-up cleanup PR promised in the prior DOC-2 pass landed
  (PRO-H4): `FluvioProducer` and the equally-dead `FluvioConsumer`
  (`src/streaming/consumer.rs`, same `todo!()` shape, also never
  constructed) are both deleted, along with the now-empty
  `consumer.rs` module. `EventProducer`/`EventConsumer` and
  `InMemoryEventPublisher` are unaffected; the acceptance below is
  retargeted to what actually ships.
  - **Acceptance:** met via T-11 — `tests/fluvio_relay.rs` is a
    `#[cfg(feature = "fluvio")]`, `#[ignore]`d round-trip (create under
    `EventTransport::Outbox` → `FluvioSink` → `drain_once` → assert
    `published_at`) against a real broker; see T-11 for the full
    acceptance record.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [~] **T-6 — gRPC implementation.** **Landed 2026-09-02 (repo
  `tasks.md` PRO-H11 — following person-service's and worker-service's
  reference implementations).**
  - [x] Promoted the stub to a working Tonic server:
    `proto/event.proto` (package `event`) + `build.rs` (`tonic-build`,
    already correctly pinned to 0.12 in this crate's manifest, same as
    worker's — but still dead scaffolding with no `build.rs`/`proto/`
    until now) + `src/api/grpc/service.rs` (`EventGrpcService`),
    covering `CreateEvent` / `GetEvent` / `ListEvents` / `DeleteEvent`.
    Deliberately not the full REST surface: no `UpdateEvent` RPC, no
    match/merge/search/FHIR over gRPC. `ListEvents` calls
    `EventRepository::list_active` directly — this crate has **no REST
    list endpoint at all** to mirror (confirmed by grep, not assumed),
    unlike person's/worker's gRPC slices which each mirrored a real
    `GET /api/<plural>`. The proto `Event` message is also a
    deliberate **partial** projection (id, name, start/end date,
    `event_status`, timestamps) — not every field the schema.org/Event
    domain model carries (identifiers, location, organizer, performer,
    offers, …); extending it is follow-up.
  - [x] **No duplicated business logic.** Every RPC delegates into the
    exact functions REST already calls: `crate::validation::validate_event`
    (`CreateEvent`), the shared duplicate-detection core
    (`check_duplicates_internal`, bumped from private to `pub(crate)`
    rather than copied), the same `EventRepository` trait methods
    (`create`/`get_by_id`/`list_active`/`delete` — this crate's
    repository takes no `AuditContext`, like worker's). `event_status`
    parses via the domain enum's existing `serde` implementation
    (`serde_json`, in both directions — `EventStatus` has no `Display`
    impl unlike `WorkerType`, so there is no shortcut for the output
    side either) rather than a hand-rolled second mapping.
  - [x] **Auth parity, and a genuine simplification confirmed by
    reading REST, not assumed.** `grpc_enforce` mirrors this crate's
    blanket-guard `require_auth_mw`, gated by the same
    `EVENT_REQUIRE_AUTH` flag. Unlike person's/worker's gRPC slices,
    there is **no record-level ABAC pass** to add: this crate's own
    `create_event`/`get_event`/`delete_event` REST handlers apply only
    the blanket guard too, with no `authorize_record` call to mirror —
    confirmed by reading them, not assumed absent. **Documented, not
    silently missing:** `UpdateEvent` has no RPC yet.
  - [x] **Verified live, not merely compiled.**
    `tests/grpc_integration_test.rs` binds a real
    `tonic::transport::Server` on an OS-assigned port and drives it
    with a real `EventServiceClient` over an actual HTTP/2 connection:
    a Create→Get→List→Delete→Get(`NOT_FOUND`) round trip against the
    same database/search-index REST integration tests use, plus a
    blank-name → `INVALID_ARGUMENT` proof, an unrecognised
    `event_status` → `INVALID_ARGUMENT` proof, and a malformed-id →
    `INVALID_ARGUMENT` proof (not `INTERNAL`). All four pass against a
    real Postgres (`scripts/ci-check.sh test-db
    event/event-service-with-loco`, full suite green). `grpcurl` was
    not additionally run by hand — unavailable in this sandbox — but
    the automated test proves the identical claim the spec's original
    acceptance criterion named, repeatably.
  - **Acceptance:** `grpcurl` against `EventService.GetEvent`
    round-trips a record — satisfied by `tests/grpc_integration_test.rs`
    (above); a literal `grpcurl` CLI run is optional local confirmation,
    not additionally exercised.
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
  - **Acceptance (Phase 2/3):** DB-free unit tests in `src/relay.rs`
    (logging-sink send, capturing-sink contract, config-parser defaults)
    pass; the drain poll/ack seams are DB-gated via the outbox suite.
    `cargo test --lib` green; `cargo clippy --lib --tests` clean. Default
    (no `EVENT_EVENT_TRANSPORT=outbox` + `EVENT_EVENT_RELAY`) ⇒ no relay
    loop.
  - [x] **Phase 3, `FluvioSink` (BUS-3).** *(done 2026-08-03)* Ported
    from case-service's BUS-1 reference implementation
    (`case/case-service-with-loco/src/relay.rs`). The real-broker `impl
    EventSink`, behind this crate's own `fluvio` Cargo feature (off by
    default — the dependency tree and boot behaviour of a default build
    are unchanged). One producer per topic
    (`fluvio::Fluvio::connect_with_config` + `topic_producer`, held for
    the sink's lifetime), partitioned by record `pid` per §7. Config:
    `EVENT_FLUVIO_ENDPOINT` (the broker's SC address; unset ⇒
    `LoggingSink`, unchanged default behaviour) and `EVENT_EVENT_TOPIC`
    (default `mxi.event.events`, matching this crate's existing doubled
    `EVENT_EVENT_*` naming for domain-event settings). **No silent
    fallback**: an endpoint configured **without** the `fluvio` feature
    refuses to start the relay at all (logged at `error`), rather than a
    `LoggingSink` masquerade that would mark outbox rows `published_at`
    without ever reaching the broker the operator asked for — the same
    shape as the family's artifact-store "no fallback on an explicit
    backend choice" rule (`agents/share/bulk-import-export.md` §12). The
    initial connection retries indefinitely rather than falling back,
    for the same reason. `compose.fluvio.yaml` + `Dockerfile.fluvio-cli`
    provision a local SC+SPU broker (ports 9203/9210/9211, distinct from
    case's 9103/9110/9111 so both can run at once) for opt-in manual
    runs; **not** wired into any automated CI stage. Tests: `cargo
    build`/`clippy --all-targets -D warnings`/`fmt --check` clean under
    both default features and `--features fluvio` (the real `fluvio`
    0.50 API compiling is the actual verification of correct usage);
    `cargo test --lib` green under both configs (152 passed, 1 ignored,
    identical count). `tests/fluvio_relay.rs` is a `#![cfg(feature =
    "fluvio")]`-gated, `#[ignore]`d round-trip (create under
    `EventTransport::Outbox` → `FluvioSink` → `drain_once` → assert
    `published_at`) with its run command documented inline — it needs a
    live broker, which no automated run in this repo stands up, so it is
    verified by compiling under the feature, not by an actual execution
    (same posture as case's BUS-1 test and person's
    `s3_round_trip_against_a_live_endpoint`, BLK-4). This crate carries
    no `compliance/soup.tsv`, so no SOUP register update applies here
    (unlike case's BUS-1 landing). Full DB-gated suite
    (`scripts/ci-check.sh test-db event/event-service-with-loco`) green,
    zero regressions. `cargo deny check` carries the same single
    pre-existing `RUSTSEC-2023-0071` (via `loco-rs` → `jsonwebtoken` →
    `rsa`, unrelated to `fluvio`) with and without the feature —
    confirmed by diffing the check's output against the pre-change
    `Cargo.lock`; `fluvio`'s own dependency tree introduces no new
    advisory, duplicate, or license warning. **Deviation from the BUS-1
    template:** the `tests/fluvio_relay.rs` round-trip drives
    `SeaOrmEventRepository::with_transport(EventTransport::Outbox)`
    directly rather than case's `request::<App, _, _>` + process-wide
    `CASE_EVENT_TRANSPORT` env mutation — this crate keeps the
    person/worker-style hand-rolled `AppState`/repository layout (see
    `agents/share/architecture.md` "person-style"), which has no loco
    request-test helper; the two approaches are functionally
    equivalent (an outbox row enqueued in the same transaction as the
    create) and this one avoids the `serial_test` dependency case's
    version needs. BUS-2 (link-graph Fluvio consumer) and rolling
    `FluvioSink` to the remaining services continue elsewhere.

- [ ] **T-12 — Wire or retire the `event-matcher` crate's second-opinion adapter.**
  `src/matching/adapter.rs::to_matcher_event` and the `matcher_lib`
  re-export (`pub use ::event_matcher as matcher_lib` in
  `src/matching/mod.rs`) exist to score two service `Event`s through
  the canonical `event-matcher` crate as "a second, independent
  opinion" (per that module's doc comment), but production duplicate
  detection (create-time check, `POST /api/events/deduplicate`) goes
  entirely through the crate-local `scoring::ProbabilisticScorer` /
  `DeterministicScorer` in `src/matching/scoring.rs`. `to_matcher_event`
  is called only from its own module's `#[cfg(test)]` block — it is
  dead in every production code path. *(verified: `grep -rn
  "to_matcher_event\|matcher_lib\|calculate_score" src/` — the only
  non-doc-comment call sites for `to_matcher_event` are inside
  `src/matching/adapter.rs`'s own `mod tests`, lines 438–508.)* Either
  surface the canonical-crate score as an actual second opinion (e.g.
  in the score breakdown, or a config-gated comparison path) or remove
  the adapter + re-export and record why in `agents/matching.md`.
  **Acceptance:** `to_matcher_event`/`matcher_lib` is either called
  from a real request path with a test proving it, or removed with a
  spec note explaining the two-implementation history.

- [ ] **T-13 — Resolve the unused `match_window_overlap` time scorer.**
  `matching::algorithms::time_matching::match_window_overlap` (Jaccard
  ratio of two events' `[start, end]` windows) is implemented and unit
  tested (`algorithms.rs::window_overlap`), but never called from
  `scoring::ProbabilisticScorer::calculate_score`, which instead scores
  `start_date` and `end_date` independently via separate `START`/`END`
  weights. *(verified: `grep -rn "match_window_overlap" src/` — only
  the definition and its own unit test; `scoring.rs`'s weight table has
  no `WINDOW_OVERLAP` entry.)* This is exactly the open question
  event-matcher's own `spec/10-open-questions.md` OQ-C leaves
  unresolved, but here it is half-built: the function exists with no
  caller. Decide whether window-overlap replaces, blends with, or stays
  dead alongside the independent start/end scoring, and either wire it
  behind a `MatchingConfig` flag or delete it.
  **Acceptance:** `match_window_overlap` has at least one production
  call site with an integration/unit test proving its effect on a real
  match score, or is removed and the decision recorded in
  `agents/matching.md`.

- [ ] **T-14 — Retire or wire the dead `organizations` table and
  `Organization` model.** `src/models/organization.rs::Organization`
  (a standalone venue-operator/promoter record, per its doc comment) is
  referenced nowhere outside its own module and the blanket
  `models::mod.rs` re-export. Separately, `db::models::organizations`
  (a SeaORM entity backed by migration
  `m20241228_000001_create_organizations`) is wired only as a
  `belongs_to` foreign-key target from party rows — nothing ever
  inserts into it. *(verified: `grep -rn "models::organization\|use.*organization::" src/`
  — only self-references; `grep -n "organizations::ActiveModel\|organizations::Entity::insert\|Organization::insert" src/db/*.rs`
  — zero hits.)* The table is migrated and permanently empty in every
  deployment. Either add the CRUD path that populates it (so party
  `organization_id` FKs can resolve to a real record) or drop the
  domain model + table and repoint party rows accordingly.
  **Acceptance:** `organizations` rows are created by some code path
  with a test proving it, or the model/table/migration are removed
  (with a down-migration and CHANGELOG entry) and `models::mod.rs`'s
  doc comment updated to stop describing it as delivered.

- [ ] **T-15 — Persist the dedup review queue (promotes OQ-4 to a
  task).** `POST /api/events/deduplicate` computes `ReviewQueueItem`s
  on the fly and returns them in the response body; there is no
  `review_queue` table, no `GET .../review-queue` listing endpoint, and
  no `POST .../review-queue/{id}/decision` endpoint — unlike person /
  worker / place / thing / organization, which all persist the queue
  per
  [match-search-merge.md](../../../agents/share/match-search-merge.md).
  *(verified: `grep -rn "review_queue\|ReviewQueue" src/` — the type
  lives only in `src/models/review_queue.rs` and is consumed inline by
  `handlers::deduplicate_events`; §16 OQ-4 already documents this gap
  as "tracked as future work, not scheduled" but it has never been
  given a `T-` id.)* A returned `AutoMerged` item today is a label
  only — no merge is actually performed, since there is no persisted
  row a follow-up call could reference by id. Add the `review_queue`
  table (normalized-pair `UNIQUE` upsert, mirroring person's schema),
  the two endpoints, and wire `AutoMerged` items to actually invoke the
  merge path.
  **Acceptance:** a batch dedup scan persists candidate pairs;
  `GET /api/events/review-queue` lists `pending` items;
  `POST /api/events/review-queue/{id}/decision` transitions
  `pending → confirmed|rejected` (first-writer-wins, only `pending`
  decidable); an `AutoMerged` item results in an actual merge, not
  just a label.

- [x] **T-CFG — `Config::from_env` loads the environment.** *(done
  2026-07-23)* The function was a stub that returned `Config::default()`
  and ignored the process environment, so every documented variable
  (`DATABASE_URL`, `SERVER_PORT`, `SEARCH_INDEX_PATH`, …) was inert —
  the integration-test harness, which builds its state from
  `Config::from_env()`, could never be pointed at a test database.
  - Env → `.env` (best-effort) → default precedence, over the 14
    variables in the `from_env` doc table (the family's 11 plus
    `SEARCH_CACHE_SIZE_MB`, `STREAMING_BROKER_URL`, `STREAMING_TOPIC`,
    which were previously unreachable config fields).
  - Blank / whitespace-only ⇒ **unset** (an empty `SERVER_HOST` must
    not bind the server to nothing); a malformed typed value ⇒
    `Error::Config` naming the variable and its raw value, never a
    silent default.
  - The overlay lives in a pure `Config::from_source(lookup)` seam so
    it is testable without mutating process env — `std::env::set_var`
    is `unsafe` in the 2024 edition, which this crate forbids.
  - **Acceptance:** five unit tests (defaults, every variable applied,
    blank-as-unset, malformed-refused-by-name, whitespace tolerance)
    green on a bare `cargo test --lib`; clippy clean. The same seam and
    tests landed in all six `*-service-with-loco` crates that carry a
    `Config`, so the family is uniform.
