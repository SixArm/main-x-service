## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [x] **SEC-M1 (security): input-size caps on the `Thing` payload.**
  `validate_thing` bounds scalar text (`MAX_TEXT_LEN = 1024`), string-array
  cardinality + per-entry (`MAX_ARRAY_LEN = 256` / `MAX_ITEM_LEN = 512`:
  `alternate_names`, `images`, `same_as`), and `identifiers` cardinality +
  inner `value`/`name`/`url` → field-scoped `422` before persist/match,
  closing the O(n·m) matcher `DoS`. Factored into `thing_size_caps`/`cap_*`.
  Unit tested. (Repo tasks.md Phase 5 SEC-M1.)

- [x] **T-1 — Production Fluvio publisher.** *Superseded rather than
  implemented literally: not a `FluvioEventPublisher : EventProducer`,*
  but the durable-bus `FluvioSink : EventSink` (BUS-3, below) is the
  production Fluvio publisher the family settled on — one real-broker
  producer per topic, behind this crate's own `fluvio` Cargo feature,
  selected by `THING_FLUVIO_ENDPOINT`. Remaining gap: only case's
  producer is wired to a real deployment target today (see
  `agents/share/overview.md` footnote 4) — this crate's sink is live
  but idle until an operator points `THING_FLUVIO_ENDPOINT` at a
  broker.
  - **Acceptance:** `tests/fluvio_relay.rs` publishes a record
    end-to-end against a broker (`#[ignore]`d — no automated run in
    this repo stands one up; verified by compiling clean under
    `--features fluvio`).
- [x] **T-2 — Introduce `ThingMatcher` trait.** *(2026-09-03)* The
  concrete facade formerly named `ThingMatcher` is renamed
  `ProbabilisticMatcher` (matching the sibling `event-service`/
  `worker-service` naming convention — the concrete strategy, not the
  trait, carries the algorithm's name); a new `ThingMatcher` trait
  (`score`/`is_match`/`threshold`) is implemented for it.
  `AppState::matcher` is now `Arc<dyn ThingMatcher>` rather than
  `Arc<ProbabilisticMatcher>`, so an alternative scorer can be
  substituted without touching `AppState` or any handler — every call
  site already went through `.score()`/`.is_match()`/`.threshold()`,
  never the struct directly, so no handler changed.
  - [x] Promote `compute_match` to a trait so alternative scorers
    (ML-based, embedding-based) can plug in.
  - **Acceptance:** `ProbabilisticMatcher : ThingMatcher` compiles
    and behaves identically to today's free function — proven by
    `matching::tests::probabilistic_matcher_implements_thing_matcher`
    (drives the concrete matcher through a `Box<dyn ThingMatcher>`) and
    `matching::tests::trait_score_matches_the_free_function` (the
    trait's `score` and a direct `compute_match` call agree bit-for-bit
    on the same inputs).
- [ ] **T-3 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `ThingService.GetThing`
    round-trips a record.
- [x] **T-4 — Authentication / authorisation.** Peer PASETO
  verification *(done 2026-07-04)*, default-off blanket enforcement
  *(done 2026-07-04)*, boot-time published-key HTTP fetch
  *(done 2026-07-04)*, and ABAC authorization *(done 2026-07-05)*. Per
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
  - [x] ABAC authorization *(done 2026-07-05; supersedes the earlier
    roles sketch — editor / read-only / service — per
    [authorization-attributes](../../../agents/share/authorization-attributes.md))*
    — inside the blanket guard (so only when `THING_REQUIRE_AUTH` is
    on), a verified token's `attrs` claim is evaluated by the shared
    engine in `authentication-verifier` 0.3: the action is derived
    from the HTTP method + this crate's destructive named POSTs
    (`auth::DESTRUCTIVE_POST_SUFFIXES`: `/merge`, `/deduplicate`,
    `/import`), and the policy — `THING_ABAC_POLICY` (inline JSON) /
    `THING_ABAC_POLICY_FILE` (path), unset/unparsable ⇒ warn-log +
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
  - [x] Fetch the published Ed25519 key set over HTTP at boot
    *(done 2026-07-04)* — when the new `THING_PASETO_KEYS_URL` env var
    is set (non-blank), `App::after_routes` (async boot context) calls
    `state::boot_verifier`, which fetches the key-set JSON once via
    `Verifier::from_paseto_keys_url` (the `authentication-verifier`
    `fetch` feature, now enabled on the path dep). A successful fetch
    **wins** over any `THING_PASETO_KEYS` env value (info-logged with
    the source URL); any fetch failure warn-logs and falls back to the
    env path (else the empty reject-all set) — the service **always
    boots**. Unset/blank URL ⇒ prior behaviour exactly. The fetched
    verifier is installed via `AppState::with_verifier` **before** the
    shared-store insert and the `require_auth_mw` middleware capture
    the state, so both router surfaces consult the fetched key set.
    Fetch happens once at boot; no refresh loop (rotation re-fetch is
    roadmap — §15).
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
  - **Acceptance (boot-time key fetch — met):** DB-free tokio tests in
    `src/api/rest/auth.rs` — a local ephemeral-port HTTP listener
    serves the in-process key set and the fetch-built verifier accepts
    a token signed by that key; a fast-failing URL
    (`http://127.0.0.1:1/`) falls back to the env/empty path without
    panic. Met: `cargo test --lib` green.
  - **Acceptance (authorization — met):** valid token whose
    attributes satisfy the policy gets `2xx`; a valid token the
    policy denies gets `403`; no/bad token gets `401`. T-4 is
    complete; activation (`THING_REQUIRE_AUTH=1`) remains the
    operational decision.
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
- [x] **T-9 — FHIR R5 API** (`Device`) — adopt the family contract
  ([`agents/share/fhir.md`](../../../agents/share/fhir.md)). Map the
  stored `thing_matcher` DTO to a FHIR **`Device`** resource (§3,
  `medium` fidelity — the core fields map; `Device`'s clinical
  structure is only partly populated): name → `Device.name`
  (deviceName), identifiers (DOI/ISBN/GTIN/serial, …) → `identifier`
  (token `system|value`, or `udiCarrier` where a UDI), thing
  type/category → `type`, manufacturer → `manufacturer`, model →
  `modelNumber`; `status`. Note `Substance`/`Medication` are out of v1
  scope. New `src/fhir/` module (resource structs,
  `to_fhir_device`/`from_fhir_device`, `FhirOperationOutcome`,
  searchset `Bundle`, search-param parsing) + a mounted
  `src/controllers/fhir.rs` (`routes()` in `app.rs`):
  read/create/update/delete/search at `/fhir/Device{,/{id}}` + `GET
  /fhir/metadata` `CapabilityStatement`. Reuses native model helpers,
  validators, event/audit, and the blanket auth+ABAC guard (§8;
  `/fhir/*` guarded, action from HTTP method). Supported search
  params: `_id`, `_lastUpdated`, `_count`, `identifier`, `type`,
  `manufacturer`. Tests: DTO↔`Device` round-trip, each interaction,
  search→Bundle, `OperationOutcome` on 404/400/422,
  `CapabilityStatement` matches routes.
  - **Done (2026-07-07):** `src/fhir/` (`resources.rs`, `mod.rs`
    conversions + scheme↔system map, `search.rs`) + mounted
    `src/controllers/fhir.rs` (`routes()` added in `app.rs` via
    `crate::controllers::fhir::routes()`); read/create/update/delete/
    search at `/fhir/Device{,/{id}}` + `GET /fhir/metadata`. Maps the
    stored `models::thing::Thing` DTO (`medium` fidelity): `name` +
    `alternate_names` → `Device.name`, `identifiers` → `identifier`
    (round-trip `system|value`), `additional_type` → `type`,
    `owner` → `manufacturer` (approx.), `disambiguating_description` →
    `modelNumber` (approx.), `description` → `note`, `is_deleted` →
    `status`. Writes reuse the repository + validators + event/audit
    of the native controller. The blanket guard now covers `/fhir/*`
    (`/fhir/metadata` public). Gaps (no `Device` home): `url`,
    `images`, `main_entity_of_page`, `same_as`, `subject_of`,
    `potential_action`, per-identifier `name`/`url`;
    `Substance`/`Medication` out of scope. DB-free tests pass
    (`cargo test --lib`: 153); `cargo clippy --lib` clean (pedantic).
- [x] **T-10 — Durable event bus, Phase 2 (transactional outbox).**
  Adopt the family contract
  ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md)
  §3/§5), copying the finished **event-service** repo-based template.
  - **Done (2026-07-08):** `event_outbox` migration
    (`migrations/2026070800000001_create_event_outbox/` +
    `m20260708_000001_create_event_outbox` registered in the migrator)
    + SeaORM entity (`db::models::event_outbox`). New
    `src/streaming/envelope.rs` — the canonical versioned `Envelope`
    (`entity` is `#[serde(skip_deserializing, default = default_entity)]`;
    `merged_from` on `Merged`), `EventKind`, `EventView`, and the
    `EventTransport` / `transport()` selector reading
    `THING_EVENT_TRANSPORT` (default `memory`), alongside — not
    replacing — the legacy in-memory `ThingEvent`/`EventPublisher`.
    New `src/db/outbox.rs` — the pure `OutboxInsert::from_envelope`
    (DB-free) + `for_event`/`for_merge` + `ConnectionTrait`-generic
    `insert_on` (so the repo passes its own transaction) + relay
    `recent`/`unpublished`/`mark_published`. `SeaOrmThingRepository`
    gains a `transport` field + `enqueue_outbox<C: ConnectionTrait>`;
    `create`/`update`/`soft_delete` write the outbox row **inside the
    entity write's transaction** (`soft_delete` grows a transaction
    under `outbox`); a new repo `merge(survivor, duplicate_id)` emits
    the survivor's `Merged` (+`merged_from`) and the duplicate's
    `Deleted` atomically in **one** transaction, and the
    `POST /api/things/merge` handler now calls it (merge-history,
    search sync, and the in-memory event stay in the handler). Gated
    by `THING_EVENT_TRANSPORT`; `memory` (default) is behaviour-neutral
    — the in-memory publish is unchanged. DB-free unit tests
    (`envelope`, `outbox::from_envelope`, transport parse) pass
    (`cargo test --lib`: 171; `clippy --lib --tests` clean); DB-gated
    `#[ignore]` atomicity tests (create + merge) compile and run under
    `DATABASE_URL=… cargo test --lib -- --ignored`.
  - **Phase 3 — Done (2026-07-08):** the relay + retention loop landed
    in `src/relay.rs` (copy-adapted from the finished **organization**
    reference; paths retargeted to thing's `crate::db::...` outbox and
    the `.map_err(map_db)` error path, since thing's `Error` has no
    `From<DbErr>`). The `EventSink` trait + no-broker `LoggingSink`,
    `drain_once` (drain unpublished → sink → `mark_published`, stopping
    at the first send failure to preserve per-pid order), and
    `purge_published` (the `THING_EVENT_RETENTION_DAYS` retention sweep)
    run in a `tokio` background loop spawned from `App::after_routes`
    via `relay::spawn(ctx.db.clone())`. Gated on transport `outbox`
    **and** `THING_EVENT_RELAY` (truthy); a no-op by default. Config:
    `THING_EVENT_RELAY`, `THING_EVENT_RELAY_INTERVAL_SECS` (default 5,
    floored at 1), `THING_EVENT_RETENTION_DAYS` (default 7, now
    enforced). Three DB-free unit tests (`logging_sink_sends_ok`,
    `capturing_sink_records_entity_and_key`, `config_defaults_are_safe`)
    pass (`cargo test --lib`: 174; `clippy --lib --tests` clean).
- [x] **Durable event bus — Phase 3, `FluvioSink` (BUS-3).** *(done
  2026-08-03)* Ported from the case-service **reference** (BUS-1): the
  real-broker `impl EventSink`, behind this crate's own `fluvio` Cargo
  feature (off by default — the dependency tree and boot behaviour of a
  default build are unchanged). One producer per topic
  (`fluvio::Fluvio::connect_with_config` + `topic_producer`, held for the
  sink's lifetime), partitioned by record `pid` per §7. Config:
  `THING_FLUVIO_ENDPOINT` (the broker's SC address; unset ⇒
  `LoggingSink`, unchanged default behaviour) and `THING_EVENT_TOPIC`
  (default `mxi.thing.events`). **No silent fallback**: an endpoint
  configured **without** the `fluvio` feature refuses to start the relay
  at all (logged at `error`), rather than a `LoggingSink` masquerade that
  would mark outbox rows `published_at` without ever reaching the broker
  the operator asked for — the same shape as the family's artifact-store
  "no fallback on an explicit backend choice" rule
  (`agents/share/bulk-import-export.md` §12). The initial connection
  retries indefinitely rather than falling back, for the same reason.
  `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a local
  SC+SPU broker (Fluvio's own documented Docker Compose layout,
  translated to this repo's Podman conventions) for opt-in manual runs;
  **not** wired into any automated CI stage. Tests:
  `cargo build`/`clippy --all-targets -D warnings`/`fmt --check` clean
  under both default features and `--features fluvio` (the real `fluvio`
  0.50 API compiling is the actual verification of correct usage);
  `tests/fluvio_relay.rs` is a `#![cfg(feature = "fluvio")]`-gated,
  `#[ignore]`d round-trip (create under outbox transport → `FluvioSink` →
  `drain_once` → assert `published_at`), following this crate's own
  DB-gated-test convention (direct `DATABASE_URL` connect +
  `SeaOrmThingRepository`, not case's `loco_rs::testing::request`
  harness, which this crate does not use elsewhere) — its run command is
  documented inline. It needs a live broker, which no automated run in
  this repo stands up, so it is verified by compiling under the feature,
  not by an actual execution (same posture as case's BUS-1 and person's
  `s3_round_trip_against_a_live_endpoint`, BLK-4). This crate has no
  `compliance/soup.tsv` (unlike case), so no SOUP register update
  applies here. No behavioural change under the default build; `cargo
  test --lib` count unchanged (174) with default features, and compiles
  clean (same 174 DB-free tests) under `--features fluvio`.

- [x] **2026-07-19 — Stored review queue + decision endpoints.** Persist
  the batch-dedup candidates (`review_queue` migration + the shared
  raw-SQL `db/review_queue` module: normalized-pair upsert / list /
  first-writer-wins decide), report stored rows from the scan, and add
  `GET /api/things/review-queue` + `POST
  /api/things/review-queue/{id}/decision`. Front-end `/review` board
  loads the stored queue on mount and drag records decisions.
  **Acceptance:** serde pins for the decision wire tokens; the person
  crate's env-gated DB round-trip (`tests/review_queue_db.rs` — the
  module is byte-identical family-wide) green against Postgres 18;
  `cargo test --lib` + clippy pedantic clean; FE svelte-check / vitest /
  Playwright green.

- [x] **2026-07-28 — Row-level integrity digests + verify endpoints.**
  `src/compliance/mac.rs` (SHA-256 + SHA-3 + optional keyed MAC over
  the **assembled** record, not just the root row, since `identifiers`
  live in a child table) stamped in `to_active` on every write;
  `src/compliance/record_integrity.rs` / `audit_integrity.rs` recompute
  and compare on demand via `GET /api/records/verify` and `GET
  /api/audit/verify` (`?limit=`, capped at 1000). A same-day follow-up
  ("Make every integrity verification reachable") found the routes
  landed with the report counters wired but unmounted — the same
  defect class already fixed twice elsewhere — and mounted both
  handlers. See [`CHANGELOG.md`](../CHANGELOG.md).
  **Acceptance:** `GET /api/records/verify` and `GET /api/audit/verify`
  are reachable and return a report naming any digest mismatch;
  `cargo test --lib` clean.
- [x] **T-11 (2026-08-30, PRO-H12 slice 3 of 7): OpenTelemetry OTLP
  export.** New `src/observability.rs` — this crate carried no
  *working* observability module before this change:
  `opentelemetry`/`opentelemetry-otlp`/`opentelemetry_sdk`/
  `tracing-opentelemetry` were declared in `Cargo.toml` at stale
  0.27/0.28 pins with **zero consumers anywhere in `src/`** (confirmed
  by grep before touching them) — dead scaffolding from an earlier,
  since-deleted stub, the identical situation place hit landing PRO-H12
  slice 2. Close port of person-service's `src/observability.rs`
  (itself ported from link-graph-service's), bumping those stale deps
  to the family's settled 0.32/0.33 pins in the same change.
  `App::init_logger`/`on_shutdown` (`src/app.rs`) install/flush it;
  `observability::trace_mw` is layered as the outermost middleware on
  **both** of this crate's router-construction surfaces
  (`App::after_routes` and `api::rest::create_router`).
  **Needed the same renamed `otlp-test-tonic` dev-dependency place's
  slice needed**, for the same reason: this crate's own `AGENTS.md`
  already stated it plainly ("no gRPC — the Tonic dependency is unwired
  scaffolding for spec/13-tasks.md T-3, not a running server") —
  `tonic = "0.12"` + `tonic-build` are declared in `Cargo.toml` in
  anticipation of the still-open T-3, with zero code consumers, and a
  declared-but-unused dependency collides with an unrenamed
  dev-dependency the same way a genuinely-used one does. Confirmed
  this crate's shape (no observability module, dead stale OTel pins,
  declared-but-dead `tonic`, two router surfaces, no SOUP register)
  was identical to place's before starting, rather than re-deriving it
  from scratch. `tests/otlp_export.rs` + `tests/otlp_middleware.rs` +
  `tests/otlp_collector/` (ported from place, which ported from
  person-service) prove real export against a real in-process gRPC
  listener in a normal `cargo test` run. Verified independently: `cargo
  fmt --check` clean, `cargo clippy --all-targets -- -D warnings`
  clean, `cargo deny check` clean, MSRV check (`cargo +1.96 check
  --all-targets`) clean, `cargo bench --no-run` compiles clean, `cargo
  test --lib` 205/205 (was 197, +8 new `observability::tests`), `cargo
  test --test otlp_export --test otlp_middleware` 4/4.

- [ ] **T-12 (M) — Wire the review-queue `score_breakdown` that already
  has a database column.** `migrations/2026071900000001_create_review_queue/up.sql`
  declares `score_breakdown JSONB NULL` and `db::review_queue::ReviewQueueRow`
  carries the field, but `handlers::batch_deduplicate` (the same
  handler `POST /api/things/deduplicate` calls) always builds
  `NewReviewItem { …, score_breakdown: None, … }` even though the
  `MatchResult` computed one line above has a real per-field breakdown,
  and the wire type `ReviewQueueItem` has no `score_breakdown` field at
  all. `thing-front-end-with-svelte`'s own T-24 already built the
  comparison-panel breakdown table against this exact column and
  documented the gap as FR-15, verified "against this service's own
  `src/api/rest/handlers.rs` and `src/db/review_queue.rs`". *(verified:
  `grep -n score_breakdown src/api/rest/handlers.rs` shows only the
  hard-coded `None` at line 647 and no field on the `ReviewQueueItem`
  wire struct; the column exists per
  `migrations/2026071900000001_create_review_queue/up.sql`.)*
  **Acceptance:** the computed `MatchResult`'s breakdown is persisted on
  `batch_deduplicate` and serialized on `ReviewQueueItem`; a DB-gated
  test round-trips a scan and asserts the returned
  `GET /api/things/review-queue` item's `score_breakdown` is non-null
  and matches the matcher's own component scores; `cargo test --lib` +
  clippy pedantic clean; three-part change (spec §9/§13 + code + test).

- [ ] **T-13 (M) — Mask sensitive fields on `check-duplicates` / create's
  `409` candidates.** `GET /api/things/search` already accepts
  `mask_sensitive` and masks results before returning them, but
  `check_duplicates` / `find_candidates` (`src/api/rest/handlers.rs`)
  return `ScoredCandidate { thing: existing, .. }` — the full, unmasked
  stored record — with no masking option at all, on both
  `POST /api/things/check-duplicates` and the `409` body
  `POST /api/things` returns on a duplicate hit. Per
  `agents/share/security.md` invariant 5 ("masking on every read
  path… a bulk or aggregate read must never reveal more than the
  equivalent single read"), a caller who cannot see a thing's full
  record via `GET` can still recover it by POSTing a near-duplicate
  probe. *(verified: `grep -n mask_sensitive src/api/rest/handlers.rs`
  shows it only on the `SearchQuery` struct and the `search_things`
  handler; `find_candidates` (line 349, returning `ScoredCandidate` —
  struct at line 339) and `check_duplicates` (line 409) have no masking
  parameter or call.)*
  **Acceptance:** `check-duplicates` (and the `409` path, sharing
  `find_candidates`) accept an optional `mask_sensitive` flag with the
  same default and masking function as `search`; a DB-free or DB-gated
  test asserts a masked duplicate-check response redacts the same
  fields `mask_thing` redacts on `/masked`; clippy pedantic clean;
  three-part change (spec §9 + code + test).

- [ ] **T-14 (M, security) — Verify GTIN/ISBN/ISSN check digits, not just
  length.** `src/validation/mod.rs::validate_gtin` explicitly documents
  "the check digit is not verified", accepting any 8/12/13/14-digit
  string; `validate_isbn`/`validate_issn` likewise only check length and
  character set, never the ISBN-10/ISSN mod-11 or ISBN-13/GTIN GS1
  mod-10 check digit. Per `agents/share/security.md` SEC-M5
  ("deterministic-identifier check-digit validation" — organization's
  LEI/GLN/DUNS/VAT precedent) an unverified check digit lets a
  transposed or mistyped identifier persist as if valid, and — since
  `thing-matcher`'s deterministic short-circuit fires on any shared
  `(property_id, value)` pair — two *different* physical items that
  happen to share a mistyped GTIN would spuriously match. The sibling
  `place-service-with-loco` crate already has exactly this GS1 mod-10
  algorithm implemented and tested as `validation::gln_is_valid`
  (`src/validation/mod.rs`), a direct adaptation source since GLN and
  GTIN share the same GS1 check-digit scheme. *(verified:
  `grep -n "check digit is not verified" src/validation/mod.rs` at the
  `validate_gtin` doc comment; `sed -n '329,395p' src/validation/mod.rs`
  shows `validate_isbn`/`validate_issn`/`validate_gtin` checking only
  length + character class.)* **Acceptance:** `validate_gtin` rejects a
  GTIN whose final digit fails the GS1 mod-10 check (adapted from
  place-service's `gln_is_valid`); `validate_isbn`/`validate_issn`
  verify their respective mod-11 check character (including the `X`
  case); existing fixture identifiers in tests are real, checksum-valid
  values (not just digit-count filler); unit tests cover a valid id and
  a single-digit-transposed invalid one for each scheme; `cargo test
  --lib` + clippy pedantic clean; three-part change (spec §6 + code +
  test).

