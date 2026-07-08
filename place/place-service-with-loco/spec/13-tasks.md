## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

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
- [ ] **T-9 — Geo-radius `nearby` HTTP endpoint + search `offset`.**
  - [ ] Add `GET /api/places/nearby?lat=&lon=&radius_km=` wiring the
    existing `matching::geo::within_radius` Haversine primitive with a
    bounding-box pre-filter.
  - [ ] Add an `offset` field to `SearchQuery` for paginated search.
  - **Acceptance:** an integration test posts places, then
    `GET /api/places/nearby` returns only those within the radius;
    `search?offset=` skips the requested number of results.
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
  - **Remaining follow-up:** a real `FluvioSink` behind a `fluvio` cargo feature (another `impl EventSink`; the trait is the seam so the drain loop is unchanged), and flipping `PLACE_EVENT_TRANSPORT=outbox` in deployment with the search-reindex consumer. Supersedes T-3's in-memory-only Fluvio publisher note.

