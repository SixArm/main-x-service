## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [x] **2026-08-22 — Geo coordinates as exact decimals (`f64` →
  `BigDecimal`, `DOUBLE PRECISION` → `NUMERIC`).**
  `GeoCoordinates::latitude` / `longitude` / `elevation` and
  `places.geo_latitude` / `.geo_longitude` / `.geo_elevation` (migration
  `m20260822_000001_geo_coordinates_to_numeric`). A coordinate is a
  decimal quantity: `DOUBLE PRECISION` cannot hold `40.7829` (it holds
  `40.78289999999999793…`) and cannot distinguish it from
  `40.78290000000000001`. Applied after the equivalent change in
  event-service, but **not** a bug fix here: event's `Location` is an
  internally-tagged enum whose `f64` fields broke under `serde_json`'s
  `arbitrary_precision`, whereas place-service has no tagged enum or
  flattened struct in its request path. Same correctness argument, for
  its own sake and for consistency between the two services that model
  geography. **Wire format deliberately unchanged** — the fields use
  `bigdecimal::impl_serde::arbitrary_precision[_option]`, so JSON stays a
  number, including on the FHIR `Location.position` surface (FHIR
  `decimal` is arbitrary-precision by spec). `distance_to` and the
  matcher adapter convert to `f64` at the Haversine boundary and yield
  `NaN` for an unrepresentable coordinate, so proximity fails closed.
  `GeoCoordinates::new` keeps its `f64` signature (92 call sites, all
  literal constants, no production callers) and stores the decimal each
  literal denotes via the shortest round-tripping string — **not**
  `BigDecimal::from_f64`, which would expand the binary approximation to
  forty-six digits. Privacy masking now rounds exactly
  (`with_scale_round(2, HalfUp)`) instead of `(x * 100.0).round() /
  100.0`. Adds `MAX_COORDINATE_SCALE` (10 places), replacing the digit
  bound `f64` provided by accident, and closes a latent hole: `NaN`
  compared false against both range bounds, so a `NaN` latitude passed
  validation — a decimal cannot represent one. **Acceptance:**
  coordinates serialize as JSON numbers, not strings; exact round-trip
  including `40.78290000000000001`; `new` stores `40.7829` as `40.7829`;
  absent elevation stays `null`; range bounds inclusive and enforced a
  hair outside; over-scale → `422`; non-finite → panic in `new`. §4,
  §5.2.1, §5.3, §6, §10.1. Verified: 212 unit + integration + 47
  doctests, DB-gated suite green against Postgres 18 with all three
  columns confirmed `numeric` and `idx_places_geo` intact, clippy `-D
  warnings`, fmt, MSRV 1.95, bench link, `cargo deny`.

- [x] **SEC-M1 (security): input-size caps on the `Place` payload.**
  `validate_place` bounds scalar text (`MAX_TEXT_LEN = 1024`, incl. nested
  `address.*`), string-array cardinality + per-entry (`MAX_ARRAY_LEN = 256`
  / `MAX_ITEM_LEN = 512`), and struct-array (`identifiers`,
  `amenity_features`, `opening_hours`) inner text + cardinality →
  field-scoped `422` before persist/match, closing the O(n·m) matcher
  `DoS`. GLN + `opening_hours` times keep stricter bounds; geo ranges
  untouched. Factored into `place_size_caps`/`cap_*`. Unit tested. (Repo
  tasks.md Phase 5 SEC-M1.)

- [x] **T-0 — Prometheus metrics scrape endpoint.**
  - [x] Serve `crate::metrics::METRICS.render()` at the root path
    `GET /metrics.prom` (`text/plain; version=0.0.4`), not under `/api`,
    so a default scraper finds it; handler
    `api::rest::handlers::metrics_prom`, registered via
    `api::rest::metrics_routes()` and in the `create_router` Axum surface.
  - [x] Add the path to the OpenAPI document (`observability` tag).
  - **Acceptance:** DB-free tests — `metrics::tests` (registry render +
    counter) and `api::rest::tests::openapi_includes_metrics_prom_path`
    (OpenAPI advertises `/metrics.prom`).
- [ ] **T-1 — PostGIS-backed spatial queries.**
  - [ ] Add `geometry(Point, 4326)` column on `place_geo_coordinates`.
  - [ ] GiST index + `ST_DWithin` for geo-radius search.
  - **Acceptance:** geo-radius search ≤ 200 ms p50 at 1 M places.
- [ ] **T-2 — Recursive CTE for place hierarchy depth queries.**
  - [ ] Replace linear walk with `WITH RECURSIVE`.
  - **Acceptance:** "list all descendants of `place_id`" returns
    correctly for ≥ 5 levels deep, ≤ 100 ms p50.
- [ ] **T-3 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag.
  - **Acceptance:** integration test publishes a `PlaceCreated`
    record end-to-end.
- [ ] **T-4 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `PlaceService.GetPlace`
    round-trips a record.
- [ ] **T-5 — OSM import pipeline.**
  - [ ] Streaming PBF reader → Place upserts.
  - [ ] Idempotency via OSM ID.
  - **Acceptance:** import a small `.osm.pbf` extract; spot-checks
    pass against the canonical OSM web view.
- [ ] **T-6 — Reverse-geocoding endpoint.**
  - [ ] `GET /api/places/reverse-geocode?lat=&lon=`.
  - **Acceptance:** known NYC coords return the corresponding
    administrative-area place.
- [ ] **T-7 — GeoJSON export.**
  - [ ] `GET /api/places/{id}.geojson` (Feature with point geometry +
    properties).
  - [ ] `GET /api/places/search.geojson?bbox=` (FeatureCollection).
  - **Acceptance:** `jq -e '.type == "Feature"'` passes.
- [ ] **T-8 — Authentication / authorisation.**
  - [x] Peer PASETO verification *(done 2026-07-04)* — offline PASETO
    v4.public (Ed25519) verification via the `authentication-verifier`
    crate 0.2 (path dep; per
    [authentication-sessions](../../../agents/share/authentication-sessions.md)
    §5). `AuthUser` extractor + `GET /api/whoami` verify bearer tokens
    offline — signature, footer `kid`, `iss`, `aud`, `exp` — via
    `bearer_claims` in `src/api/rest/auth.rs`. Verifier built from env
    at boot (`PLACE_PASETO_KEYS` key set as published at
    `/.well-known/paseto-keys`; `PLACE_TOKEN_ISSUER` /
    `PLACE_TOKEN_AUDIENCE`, defaults `authentication-service` /
    `main-x-service`); absent key set ⇒ empty set, every token
    rejected, service still boots. Acceptance met: DB-free unit tests
    in `src/api/rest/auth.rs` mint `v4.public` tokens in-process
    (throwaway Ed25519 key) and pin valid / missing / non-bearer /
    expired / tampered / no-key outcomes — `cargo test --lib` green.
  - [x] Blanket enforcement middleware on `/api/*` *(done 2026-07-04)*
    — a valid PASETO bearer token is required on every route except
    the public allow-list (`auth::PUBLIC_PATHS` +
    `PUBLIC_PATH_PREFIXES`: `/api/health`, `/_health`, `/_ping`,
    `/api-docs/openapi.json`, `/swagger-ui*`, `/metrics.prom`), gated
    by the default-off `PLACE_REQUIRE_AUTH` env flag (lenient parse:
    `1`/`true`/`yes`/`on` ⇒ on; unset/blank/`0`/junk ⇒ off; read at
    router construction — restart to change). The pure
    `auth::enforce` decision is wired as an Axum
    `from_fn_with_state` middleware on **both** router surfaces
    (`create_router` and the loco router in `app.rs::after_routes`).
    Acceptance met: DB-free unit tests in `src/api/rest/auth.rs` pin
    the full matrix — off + no token ⇒ Ok; on + public paths ⇒ Ok;
    on + protected + no token ⇒ `401`; on + protected + valid ⇒ Ok;
    on + expired/tampered ⇒ `401`; `parse_bool` parser — `cargo test
    --lib` green.
  - [x] ABAC authorization *(done 2026-07-05; supersedes the earlier
    roles/RBAC sketch — editor / curator / read-only / service — per
    [authorization-attributes](../../../agents/share/authorization-attributes.md))*
    — inside the blanket guard (so only when `PLACE_REQUIRE_AUTH` is
    on), a verified token's `attrs` claim is evaluated by the shared
    engine in `authentication-verifier` 0.3: the action is derived
    from the HTTP method + this crate's destructive named POSTs
    (`auth::DESTRUCTIVE_POST_SUFFIXES`: `/merge`, `/deduplicate`,
    `/import`), and the policy — `PLACE_ABAC_POLICY` (inline JSON) /
    `PLACE_ABAC_POLICY_FILE` (path), unset/unparsable ⇒ warn-log +
    built-in default policy, read once at router construction —
    decides first-match-wins with default allow-read / deny-mutation.
    `401` = missing/bad credential; `403` = valid credential, policy
    denied (body carries the deciding rule). Acceptance met: DB-free
    unit tests in `src/api/rest/auth.rs` pin the §7 matrix — action
    derivation; empty `attrs` ⇒ GET ok / POST 403; `access=write` ⇒
    POST/PUT ok, DELETE + merge 403; `access=admin` ⇒ destructive ok;
    `svc=true` ⇒ everything; configured deny beats later allow;
    401-vs-403 split; bad policy JSON falls back to the default —
    `cargo test --lib` green.
  - [x] Fetch the key set over HTTP from the auth service at boot
    *(done 2026-07-04)* — when the new `PLACE_PASETO_KEYS_URL` env var
    is set (non-blank), `app.rs::after_routes` (async boot context)
    calls `state::boot_verifier`, which fetches the key-set JSON once
    via `Verifier::from_paseto_keys_url` (the `authentication-verifier`
    `fetch` feature, now enabled on the path dep). A successful fetch
    **wins** over any `PLACE_PASETO_KEYS` env value (info-logged with
    the source URL); any fetch failure warn-logs and falls back to the
    env path (else the empty reject-all set) — the service **always
    boots**. Unset/blank URL ⇒ prior behaviour exactly. The fetched
    verifier is installed via `AppState::with_verifier` **before** the
    enforcement middleware / shared store capture the state, so both
    router surfaces consult the fetched key set. Fetch happens once at
    boot; no refresh loop (rotation re-fetch is roadmap — §15).
    Acceptance met: DB-free tokio tests in `src/api/rest/auth.rs` — a
    local ephemeral-port HTTP listener serves the in-process key set
    and the fetch-built verifier accepts a token signed by that key;
    a fast-failing URL (`http://127.0.0.1:1/`) falls back to the
    env/empty path without panic — `cargo test --lib` green.
  - **Acceptance (met):** valid token whose attributes satisfy the
    policy gets `2xx`; a valid token the policy denies gets `403`;
    no/bad token gets `401`. T-8 is complete; activation
    (`PLACE_REQUIRE_AUTH=1`) remains the operational decision.
- [x] **T-9 — Geo-radius `nearby` HTTP endpoint + search `offset`.**
  - [x] Added `GET /api/places/nearby?lat=&lon=&radius_km=&limit=&offset=`.
    New `matching::geo::bounding_box` computes a rectangular lat/lon
    box (derived from the **same** mean Earth radius
    `GeoCoordinates::distance_to` uses, so the two agree on one
    sphere — see the constant's doc comment for the property test that
    caught the mismatch when they didn't); `db::PlaceRepository::
    list_in_bbox` runs it as a plain SQL `BETWEEN` range query over
    `idx_places_geo` (capped at 5,000 candidate rows — SEC-M1), and the
    existing `matching::geo::within_radius` Haversine check narrows
    those candidates to the true within-radius set, nearest-first.
    Follows the family pagination convention (`agents/share/
    restful.md`): `limit` clamped to 100, `offset` beyond 10,000 is a
    `400`, and `X-Total-Count`/`X-Limit`/`X-Offset` response headers
    (`X-Total-Count` is every in-radius match, ignoring the page
    window).
  - [x] Added `offset` to `SearchQuery` — it was genuinely missing, not
    merely undocumented (`GET /api/places/search` had no `offset`
    field at all). `search::SearchEngine::search_page` adds a true
    `X-Total-Count` (Tantivy's `Count` collector, not the page length)
    alongside the existing `q`/`limit`/`fuzzy`/`mask_sensitive`
    handling; `search()`/`fuzzy_search()` are unchanged call sites,
    refactored to share the same query-building as `search_page`.
  - **Acceptance (met):** `tests/api_nearby_and_search_offset.rs`
    (DB-gated, `#[ignore]`) posts places then confirms
    `GET /api/places/nearby` returns only those within the radius (and
    a place at ~95%/105% of the radius is included/excluded — the
    edge case a too-tight bounding box would get wrong); `search?
    offset=` skips the requested rows and `X-Total-Count` stays the
    true total across pages; an `offset` past the bound is `400` on
    both endpoints. Unit-level: `matching::geo` gains `bounding_box`
    tests (straddles center, grows with radius, clamps near a pole,
    a zero radius collapses to a point, and — the boundary case —
    every point on the true Haversine circle at exactly `radius_km`
    falls inside the box) plus `within_radius_boundary_is_inclusive`
    (a point placed at exactly the radius via the spherical
    destination-point formula is included; the `<=` cutoff is
    inclusive). `search::SearchEngine` gains `search_page` offset/total
    tests. One real defect found and fixed along the way, worth a
    regression test of its own
    (`search::tests::overlong_single_token_is_not_indexed`): Tantivy's
    default tokenizer drops any single unbroken token over 40
    characters, which silently dropped a compound test-fixture "unique
    token" (a literal-prefix-concatenated-with-a-32-hex-char-UUID, no
    separator) from the index — a sharp edge worth knowing about
    beyond this one test file. `cargo fmt --check` / `cargo clippy
    --all-targets -- -D warnings` / `cargo test --lib` (221, was 212)
    / `scripts/ci-check.sh test-db` all green.
- [ ] **T-10 — Bulk import / export.**
  - [ ] `bulk_jobs` migration (per shared §3 schema).
  - [ ] Five endpoints (shared §4): `POST /api/places/import`,
    `GET /api/places/import/{id}`, `POST /api/places/export`,
    `GET /api/places/export/{id}`, `GET /api/places/bulk-jobs`.
  - [ ] `bg_pg` worker draining jobs `queued → running →
    completed | completed_with_errors | failed` with progress updates.
  - [ ] JSONL (lossless reference), CSV (§10.3 column set), and Parquet
    (export-first, feature-gated) codecs.
  - [ ] Per-row pipeline reusing the single-create validators + the
    Place matcher + review queue (`provenance = import`); upsert on the
    §10.3 stable key (GLN / OSM / GNIS / FIPS / `same_as` URL / `pid`),
    keyless rows → duplicate detection → review queue.
  - [ ] Downloadable per-row error report
    (`row_number, source_line, field, code, message`).
  - [ ] Export masking (light default per §10.3) + per-export audit row;
    `include_soft_deleted` gated.
  - **Acceptance:** tests for idempotent re-import (re-running a file
    upserts, does not duplicate), per-row error report, dedupe-to-review
    with `provenance = import`, masked vs full export, and that a
    (zero-row included) export writes an audit row.
- [x] **T-11 — FHIR R5 API** (`Location`) — adopt the family contract ([`agents/share/fhir.md`](../../../agents/share/fhir.md)). Map the stored `place_matcher` DTO to a FHIR **`Location`** resource (§3, `high` fidelity): name → `name`, postal address → `address`, geo latitude/longitude/altitude → `position`, identifiers (GLN, …) → `identifier` (token `system|value`), place type → `type`, containing place → `partOf` reference; `active`/`status`. New `src/fhir/` module (resource structs, `to_fhir_location`/`from_fhir_location`, `FhirOperationOutcome`, searchset `Bundle`, search-param parsing) + a mounted `src/controllers/fhir.rs` (`routes()` added in `app.rs`): read/create/update/delete/search at `/fhir/Location{,/{id}}` + `GET /fhir/metadata` `CapabilityStatement`. Reuses the native model helpers, validators, event/audit path, and the blanket auth+ABAC guard (§8; `/fhir/*` guarded, action from HTTP method). Supported search params: `_id`, `_lastUpdated`, `_count`, `identifier`, `name`, `address`, `address-city`, `address-postalcode`. Tests: DTO↔`Location` round-trip, each interaction, search→Bundle, `OperationOutcome` on 404/400/422, `CapabilityStatement` matches mounted routes.
  - Done (2026-07-07): added `src/fhir/{mod,resources,search}.rs`, `src/controllers/{mod,fhir}.rs`; wired `pub mod controllers; pub mod fhir;` in `lib.rs` and `controllers::fhir::routes()` in `app.rs`. Endpoints: `GET/POST/PUT/DELETE /fhir/Location{,/{id}}`, `GET /fhir/Location?…` (searchset Bundle), `GET /fhir/metadata` (CapabilityStatement); all `application/fhir+json`, `OperationOutcome` on every non-2xx. Adapted to place's actual architecture (handlers take `State<AppState>`, reuse the native `PlaceRepository` + `SearchEngine` + `EventPublisher` + `AuditLogRepository`); the mapped DTO is the crate's native `models::place::Place` (place stores a rich normalized `Place`, **not** a matcher type as JSONB). Identifier scheme↔`system` round-trip map (GLN canonical, others `urn:mxi:place:*`); scalar GLN/branch-code + `identifiers` vec all surface as `identifier` tokens. 11 new DB-free lib tests (scheme round-trip, DTO↔`Location` round-trip, missing-name reject, soft-delete⇒`inactive`, 5 search-predicate, CapabilityStatement). `cargo test --lib` 162 passed; `cargo clippy --lib` clean. Fidelity gaps (documented in code): `partOf`/`contained_in_place`, `keywords`, `amenity_features`, `opening_hours`, and the boolean access flags have no FHIR `Location` home; only the first `alias`/`type` are recovered on inbound.

- [x] **T-12 — Durable event bus, Phase 2 (transactional outbox).** Adopt the family contract ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §3–§5) alongside — not replacing — the legacy in-memory `PlaceEvent` publisher. Add the `event_outbox` table + SeaORM entity, the canonical versioned `Envelope`/`EventKind`/`EventView` (`src/streaming/envelope.rs`), and the `OutboxInsert` write/relay surface (`src/db/outbox.rs`). The `SeaOrmPlaceRepository` gains a `transport` field (`with_transport`) and an `enqueue_outbox<C: ConnectionTrait>` that writes one `event_outbox` row **inside each write's own transaction** so a committed place change always has its event and vice versa: wired into `create` (`created`), `update` (`updated`), `soft_delete` (`deleted`, wrapped in a tx under outbox), and a new `merge(survivor, duplicate_id)` repo method emitting `Merged` (carrying the duplicate's pid via `merged_from`) + `Deleted` for the duplicate atomically in one transaction (the `/api/places/merge` handler now calls it). Transport is gated by `PLACE_EVENT_TRANSPORT` (`memory` default keeps today's behaviour; `outbox` durable), read once via `crate::streaming::transport()` and wired in `AppState::new`. Config: `PLACE_EVENT_TRANSPORT`, `PLACE_EVENT_RETENTION_DAYS` (outbox row TTL, consumed by the Phase-3 retention worker).
  - Done (2026-07-08): `migration/src/m20260708_000001_create_event_outbox.rs` + `migrations/2026070800000001_create_event_outbox/{up,down}.sql`; `db::models::event_outbox`; `src/db/outbox.rs`; `src/streaming/envelope.rs`. DB-free tests: envelope `for_place`/`for_merge`, `EventView` projection, `EventTransport::parse`, `OutboxInsert::from_envelope` field mapping + non-UUID-pid reject. DB-gated `#[ignore]` tests (`db::tests`, need `DATABASE_URL`): `create` writes one `created` row; `merge` writes one `merged` row carrying `merged_from` + one `deleted` row — both in one tx. `cargo test --lib` 180 passed, 2 ignored; `cargo clippy --lib --tests` clean.
- [x] **T-12b — Durable event bus, Phase 3 (outbox relay + retention).** Add the background relay worker (`src/relay.rs`): the `EventSink` trait (the bus seam) with the default no-broker `LoggingSink`; `drain_once` (poll `Model::unpublished` → `sink.send` → `Model::mark_published`, at-least-once, stops at the first send failure to preserve per-pid order); `purge_published` (delete published rows older than `PLACE_EVENT_RETENTION_DAYS`, now **enforced**); and `spawn`, wired in `App::after_routes` (`crate::relay::spawn(ctx.db.clone())`). The loop runs **only** when `PLACE_EVENT_TRANSPORT=outbox` **and** `PLACE_EVENT_RELAY` is truthy, so the default `memory` transport is unaffected. Config: `PLACE_EVENT_RELAY` (on/off), `PLACE_EVENT_RELAY_INTERVAL_SECS` (poll cadence, default 5), `PLACE_EVENT_RETENTION_DAYS` (now consumed). Error handling mirrors the repository/outbox code: functions return the crate `Result` and `.map_err(|e| crate::Error::Database(...))` on SeaORM calls (place has no `From<DbErr>`).
  - Done (2026-07-08): `src/relay.rs` (copy-adapted from the organization reference), `pub mod relay;` in `lib.rs`, spawn in `app.rs`. 3 DB-free unit tests (logging-sink smoke, capturing-sink contract, config defaults). `cargo test --lib` 183 passed, 2 ignored; `cargo clippy --lib --tests` clean.
  - **Remaining follow-up:** flipping `PLACE_EVENT_TRANSPORT=outbox` in deployment with the search-reindex consumer. Supersedes T-3's in-memory-only Fluvio publisher note. (The real `FluvioSink` follow-up landed as T-12c below.)
- [x] **T-12c — Durable event bus, Phase 3, `FluvioSink` (BUS-3).**
  *(done 2026-08-03)* Ports the case-service reference (BUS-1) onto this
  crate's `src/relay.rs`: the real-broker `impl EventSink`, behind this
  crate's own `fluvio` Cargo feature (off by default — the dependency
  tree and boot behaviour of a default build are unchanged). One
  producer per topic (`fluvio::Fluvio::connect_with_config` +
  `topic_producer`, held for the sink's lifetime), partitioned by record
  `pid` per `agents/share/event-bus.md` §7. Config: `PLACE_FLUVIO_ENDPOINT`
  (the broker's SC address; unset ⇒ `LoggingSink`, unchanged default
  behaviour) and `PLACE_EVENT_TOPIC` (default `mxi.place.events`). **No
  silent fallback**: an endpoint configured **without** the `fluvio`
  feature refuses to start the relay at all (logged at `error`), rather
  than a `LoggingSink` masquerade that would mark outbox rows
  `published_at` without ever reaching the broker the operator asked
  for — the same shape as the family's artifact-store "no fallback on an
  explicit backend choice" rule (`agents/share/bulk-import-export.md`
  §12). The initial connection retries indefinitely rather than falling
  back, for the same reason. `compose.fluvio.yaml` +
  `Dockerfile.fluvio-cli` provision a local SC+SPU broker (Fluvio's own
  documented Docker Compose layout, translated to this repo's Podman
  conventions, container names `mxi-place-fluvio-*`) for opt-in manual
  runs; **not** wired into any automated CI stage. Tests: `cargo
  build`/`clippy --all-targets -D warnings`/`fmt --check` clean under
  both default features and `--features fluvio` (the real `fluvio` 0.50
  API compiling is the actual verification of correct usage);
  `tests/fluvio_relay.rs` is a `#![cfg(feature = "fluvio")]`-gated,
  `#[ignore]`d round-trip (create under outbox transport → `FluvioSink`
  → `drain_once` → assert `published_at`) with its run command
  documented inline — it needs a live broker, which no automated run in
  this repo stands up, so it is verified by compiling under the feature,
  not by an actual execution (same posture as person's
  `s3_round_trip_against_a_live_endpoint`, BLK-4; and case's own
  `tests/fluvio_relay.rs`, BUS-1). This crate carries no
  `compliance/soup.tsv` (unlike case), so no SOUP register update was
  needed. BUS-2 (link-graph Fluvio consumer, already landed) and rolling
  `FluvioSink` to the remaining entity services continue independently.

- [x] **2026-07-19 — Stored review queue + decision endpoints.** Persist
  the batch-dedup candidates (`review_queue` migration + the shared
  raw-SQL `db/review_queue` module: normalized-pair upsert / list /
  first-writer-wins decide), report stored rows from the scan, and add
  `GET /api/places/review-queue` + `POST
  /api/places/review-queue/{id}/decision`. Front-end `/review` board
  loads the stored queue on mount and drag records decisions.
  **Acceptance:** serde pins for the decision wire tokens; the person
  crate's env-gated DB round-trip (`tests/review_queue_db.rs` — the
  module is byte-identical family-wide) green against Postgres 18;
  `cargo test --lib` + clippy pedantic clean; FE svelte-check / vitest /
  Playwright green.

- [x] **2026-07-27/28 — Keyed integrity verification (MAC + digests).**
  *Landed but never recorded here until this doc pass (2026-08-04)
  found the gap: shipped, tested, and reachable, with no `spec/13`
  entry, no `spec/14` row, and no `spec/09`/`agents/restful.md`
  endpoint listing.* Adds `src/compliance/` (`mac`, `record_integrity`,
  `audit_integrity`): SHA-256 + SHA3-256 digests and a keyed
  HMAC-SHA256 MAC (this crate's binding to the shared
  `integrity-mac` crate, HKDF-domain-separated per (service, domain))
  over each `Place` record and each `audit_log` row. Two read
  endpoints, guarded like every other `/api` route:
  `GET /api/records/verify` and `GET /api/audit/verify`. **Default
  off**: with no `PLACE_INTEGRITY_MAC_KEY` (or `_KEY_FILE`) configured,
  no MAC is written and affected rows report `mac_absent` rather than
  a mismatch — adopting the control on a populated table must not
  produce false accusations. Env vars: `PLACE_INTEGRITY_MAC_KEY`,
  `PLACE_INTEGRITY_MAC_KEY_FILE` (takes precedence),
  `PLACE_INTEGRITY_MAC_KEY_ID`, `PLACE_INTEGRITY_MAC_KEYS_RETIRED`.
  **Known limit, stated in the module docs**: unlike person / worker /
  care-pathway / case, this crate has **no hash chain** (`prev_hash` /
  `hash`) and takes no external-witness checkpoint — a MAC proves a
  row's content is unchanged since it was written, and says nothing
  about a row **deleted wholesale**. See
  `agents/share/runbooks/integrity-activation.md` for the family-wide
  activation runbook. 11 DB-free unit tests
  (`compliance::mac::tests` ×2, `compliance::record_integrity::tests`
  ×6, `compliance::audit_integrity::tests` ×3).
- [x] **T-13 (2026-08-30, PRO-H12 slice 2 of 7): OpenTelemetry OTLP
  export.** New `src/observability.rs` — this crate carried no *working*
  observability module before this change: `opentelemetry`/`opentelemetry-otlp`/
  `opentelemetry_sdk`/`tracing-opentelemetry` were declared in
  `Cargo.toml` at stale 0.27/0.28 pins with **zero consumers anywhere in
  `src/`** (confirmed by grep before touching them) — dead scaffolding
  from an earlier, since-deleted stub, not a live module to replace.
  Close port of person-service's `src/observability.rs` (itself ported
  from link-graph-service's, the family's original reference), bumping
  those stale deps to the family's settled 0.32/0.33 pins in the same
  change. `App::init_logger`/`on_shutdown` (`src/app.rs`) install/flush
  it; `observability::trace_mw` is layered as the outermost middleware
  on **both** of this crate's router-construction surfaces
  (`App::after_routes` and `api::rest::create_router`) — the same
  two-surface adaptation PRO-H9 and course (PRO-H12 slice 1) needed.
  **Correction to PRO-H12's own scoping note**: it assumed "per
  `overview.md`'s capability matrix only person/worker/event carry a
  gRPC stub, so most of the remaining seven likely need no
  `otlp-test-tonic` rename" — false for this crate. The capability
  matrix's gRPC row tracks whether a `src/api/grpc` **module** exists
  (place has none, hence its `–`), not whether `tonic` is a **declared**
  Cargo dependency; place already carries `tonic = "0.12"` +
  `tonic-build` in anticipation of the still-open T-4 (gRPC
  implementation, not yet started), and a declared-but-code-unused
  dependency collides in a dev-dependency's extern prelude exactly the
  same as a genuinely-used one (`E0464`) — Cargo doesn't know or care
  that no code calls `tonic::`. So this crate needed the **same**
  `otlp-test-tonic = { package = "tonic", version = "0.14" }` rename
  PRO-H9's three did, confirmed by first trying a plain dependency and
  watching it fail to compile before reverting to the rename. course
  (slice 1) needed no rename because it is the one crate among these
  that declares no `tonic` dependency at all, not because it lacks a
  gRPC *module* — thing (slice 3, next) needs checking on the same
  basis, not assumed from the capability matrix either.
  `tests/otlp_export.rs` + `tests/otlp_middleware.rs` +
  `tests/otlp_collector/` (ported from person-service) prove real
  export against a real in-process gRPC listener in a normal `cargo
  test` run. Verified independently: `cargo fmt --check` clean, `cargo
  clippy --all-targets -- -D warnings` clean, `cargo deny check` clean,
  MSRV check (`cargo +1.96 check --all-targets`) clean, `cargo bench
  --no-run` compiles clean, `cargo test --lib` 229/229 (was 221, +8
  new `observability::tests`), `cargo test --test otlp_export --test
  otlp_middleware` 4/4.

- [ ] **T-14 (M) — Wire the review-queue `score_breakdown` that already
  has a database column.** `migrations/2026071900000001_create_review_queue/up.sql`
  declares `score_breakdown JSONB NULL` and `db::review_queue::ReviewQueueRow`
  carries the field, but `handlers::batch_deduplicate` always builds
  `NewReviewItem { …, score_breakdown: None, … }` even though the
  `MatchResult` computed one line above (`state.matcher.score(&places[i],
  &places[j])`) has a real per-field breakdown, and the wire type
  `ReviewQueueItem` (`src/api/rest/handlers.rs`) has no `score_breakdown`
  field at all — so `review_row_to_item` cannot surface a value even if
  one were persisted. The front-end already anticipates this exact fix:
  `place-front-end-with-svelte`'s T-23 built its comparison-panel
  breakdown table against this column and documented the gap as "a
  candidate follow-up for `place-service-with-loco`'s own `spec/13-tasks.md`".
  *(verified: `grep -n score_breakdown src/api/rest/handlers.rs` shows
  only the hard-coded `None` at line 830 and no field on the
  `ReviewQueueItem` struct at lines 737–753; the column exists per
  `migrations/2026071900000001_create_review_queue/up.sql`.)*
  **Acceptance:** `serde_json::to_value(result.breakdown)` (or an
  equivalent per-field map) is persisted on `batch_deduplicate` and
  serialized on `ReviewQueueItem`; a DB-gated test round-trips a scan and
  asserts the returned `GET /api/places/review-queue` item's
  `score_breakdown` is non-null and matches the matcher's own component
  scores; `cargo test --lib` + clippy pedantic clean; three-part change
  (spec §9/§13 + code + test).

- [ ] **T-15 (M) — Mask sensitive fields on `check-duplicates` /
  create's `409` candidates.** `GET /api/places/search` already accepts
  `mask_sensitive` and masks results before returning them, but
  `check_duplicates` / `find_candidates` (`src/api/rest/handlers.rs`)
  return `ScoredCandidate { place: existing, .. }` — the full, unmasked
  stored record — with no masking option at all, on both the explicit
  `POST /api/places/check-duplicates` endpoint and the `409` body
  `POST /api/places` returns on a duplicate hit. Per
  `agents/share/security.md` invariant 5 ("masking on every read path…
  a bulk or aggregate read must never reveal more than the equivalent
  single read"), a caller who cannot see a place's full record via `GET`
  can still recover it by POSTing a near-duplicate probe. *(verified:
  `grep -n mask_sensitive src/api/rest/handlers.rs` shows it only on the
  `SearchQuery` struct and the `search_places` handler; `find_candidates`
  at line 544 and `check_duplicates` at line 596 have no masking
  parameter or call.)* **Acceptance:** `check-duplicates` (and the
  `409` path, sharing `find_candidates`) accept an optional
  `mask_sensitive` flag with the same default and masking function as
  `search`; a DB-free or DB-gated test asserts a masked duplicate-check
  response redacts the same fields `mask_place` redacts on `/masked`;
  clippy pedantic clean; three-part change (spec §9 + code + test).

- [x] **T-16 (S) — Guard `contained_in_place` against self-reference and
  cycles.** `spec/16-open-questions.md` OQ-2 states "validation rejects
  on insert" for hierarchy cycles, but no such check exists anywhere in
  the crate: `validate_place` (`src/validation/mod.rs`) never references
  `contained_in_place`, so a place can be created (or updated) with
  `contained_in_place == Some(self_id)`, and nothing prevents a
  multi-hop cycle (A contains B, B contains A) either. *(verified:
  `grep -n contained_in_place src/validation/mod.rs` returns nothing;
  `grep -rn "cycle\|self_containment" src/` finds no matching code
  anywhere in `src/`.)* At minimum, reject direct self-reference in
  `validate_place` (pure, no DB access needed); full multi-hop cycle
  detection needs a DB round-trip and belongs in the repository's
  `create`/`update`, alongside — not replacing — T-2's recursive-CTE
  descendant query. **Acceptance:** `validate_place` returns a `422` for
  `contained_in_place == Some(place.id)`; a DB-gated test creates A→B
  then attempts B→A and asserts a `409`/`422` rather than a silently
  persisted 2-cycle; unit + DB-gated tests green; three-part change
  (spec §6/§16 + code + test). *(resolved 2026-09-05.)*
  - **Resolved.** `validate_place` (`src/validation/mod.rs`) rejects
    `contained_in_place == Some(place.id)` with a `422`-shaped
    `ValidationError` on field `contained_in_place`. The multi-hop case
    is a new `SeaOrmPlaceRepository::ancestor_chain_contains` helper
    (`src/db/mod.rs`) — a bounded walk up the parent chain (a
    `visited` guard stops it looping on a pre-existing, unrelated
    cycle rather than assuming none exists) — called from both
    `create` and `update` before the write transaction opens; a hit
    returns `Error::Conflict` (`409`). Two new pure unit tests
    (`test_self_referencing_contained_in_place_is_rejected`,
    `test_contained_in_a_different_place_is_not_rejected`) plus one
    DB-gated test (`update_rejects_a_cycle_through_an_existing_ancestor`)
    creating A→B then updating A to be contained in B, asserting the
    `409` and that A's `contained_in_place` was **not** persisted.
    `spec/16-open-questions.md` OQ-2 updated to record the resolution
    (and to note the hierarchy is a tree — one `contained_in_place`
    per place — so "two paths from A to B" cannot arise; the walk
    needs no branching). Verified against a real Postgres 18 via
    `scripts/ci-check.sh test-db place/place-service-with-loco`: full
    DB-gated suite passes, 0 failed; `cargo test --lib`: 231 passed (up
    from 229, the two new unit tests), 0 failed; `cargo build`/`clippy
    --all-targets -- -D warnings` clean.

