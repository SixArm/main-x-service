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
  - [ ] Blanket enforcement middleware on `/api/*` — require a valid
    PASETO bearer token on every route except public paths (health,
    OpenAPI/Swagger, metrics), gated by a default-off
    `PLACE_REQUIRE_AUTH` env flag — with editor / curator / read-only
    / service roles; fetch the key set over HTTP from the auth service
    at boot (today it is injected via `PLACE_PASETO_KEYS`).
  - **Acceptance (remainder):** with enforcement on, unauthenticated
    requests get `401`; valid token + role gets `2xx`.
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

