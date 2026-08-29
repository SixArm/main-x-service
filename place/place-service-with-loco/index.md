# Place Service

A high-performance, enterprise-grade Place Service system built with Rust.

A registry of **place identities** based on
[schema.org/Place](https://schema.org/Place). The Place Service is
the canonical store for geographic places — sites, branches, civic
structures, landforms, administrative areas, business locations —
anything modelled by schema.org/Place.

## Quick start

Prerequisites: Rust 1.95+ (2024 edition), PostgreSQL 18+ with PostGIS.

```bash
cd place/place-service-with-loco   # within the main-x-service monorepo

# Build and run (loco.rs): point DATABASE_URL at the database, then
# start. Migrations run automatically in development (auto_migrate).
export DATABASE_URL=postgres://localhost/place_service_development
cargo loco start            # or: cargo run -- start

# Service binds on the configured port (config/development.yaml):
curl http://localhost:5150/api/health
```

## API

REST routes mount under `/api/places/*`. See
[`agents/restful.md`](agents/restful.md) for the full list. All
endpoints return the standard `{success, data, error}` envelope.
Duplicate handling includes a stored review queue:
`GET /api/places/review-queue` (filter `status`, `limit`) and
`POST /api/places/review-queue/{id}/decision` (decide a pending item,
`confirmed` / `rejected`).

An HL7 FHIR R5 surface (`src/controllers/fhir.rs`) maps places to the
FHIR `Location` resource: `GET /fhir/metadata` (CapabilityStatement),
`POST /fhir/Location` (create), `GET /fhir/Location` (search),
and `GET` / `PUT` / `DELETE /fhir/Location/{id}`.

Keyed integrity verification: `GET /api/records/verify` and
`GET /api/audit/verify` check the stored SHA-256 / SHA3-256 digests and
HMAC-SHA256 MAC over place records and the audit log. Default off — set
`PLACE_INTEGRITY_MAC_KEY` (or `_KEY_FILE`) to start writing MACs;
unkeyed rows report `mac_absent`, not a mismatch.

Prometheus metrics are exposed at the root path `GET /metrics.prom`
(text-exposition format, `text/plain; version=0.0.4`) — not under
`/api` — so a default scraper (`metrics_path: /metrics.prom`) finds it.

Interactive OpenAPI 3 documentation:

- Swagger UI: `http://localhost:5150/swagger-ui`
- Raw spec: `http://localhost:5150/api-docs/openapi.json`

### Worked example

Create a place:

```bash
curl -X POST http://localhost:5150/api/places \
  -H 'content-type: application/json' \
  -d '{
    "name": "Central Park",
    "place_type": "Park",
    "address": {
      "street_address": "14 E 60th St",
      "address_locality": "New York",
      "address_region": "NY",
      "address_country": "US",
      "postal_code": "10022"
    },
    "geo": { "latitude_as_decimal_degrees": 40.7829, "longitude_as_decimal_degrees": -73.9654 }
  }'
```

Full-text / fuzzy search (query params: `q`, `limit`, `fuzzy`,
`mask_sensitive`):

```bash
curl "http://localhost:5150/api/places/search?q=Central+Park&fuzzy=true&limit=10"
```

> A geo-radius `nearby` endpoint (`lat` / `lon` / `radius_km`) and
> `offset` pagination are **not yet delivered** — see
> [`spec/13-tasks.md`](spec/13-tasks.md) T-9. The `within_radius`
> Haversine primitive exists in `src/matching/geo.rs` but is not yet
> wired to an HTTP route.

## Configuration

Configuration is loaded from `config/{development,test,production}.yaml`
(Loco convention). Environment-overridable variables:

| Variable             | Description                | Default                 |
| -------------------- | -------------------------- | ----------------------- |
| `PORT`               | REST bind port             | `5150` (dev config)     |
| `DATABASE_URL`       | Postgres connection string | per config file         |
| `SEARCH_INDEX_PATH`  | Tantivy index directory    | `./data/search_index`   |
| `MATCHING_THRESHOLD` | Probabilistic match cutoff | `0.85`                  |
| `GRPC_PORT` | gRPC server port (Tonic stub) | `50051` |
| `SEARCH_CACHE_SIZE_MB` | Tantivy cache budget in MB | `512` |
| `OTLP_SERVICE_NAME` | service.name sent to the collector | `place-service` |
| `OTLP_ENDPOINT` | OTLP collector endpoint | `http://localhost:4317` |
| `STREAMING_BROKER_URL` | Event-broker connection URL | `localhost:9003` |
| `STREAMING_TOPIC` | Topic events publish to | `place-events` |
| `RUST_LOG`           | tracing-subscriber filter  | `info`                  |
| `OTLP_ENDPOINT`      | OpenTelemetry collector    | `http://localhost:4317` |
| `PLACE_EVENT_TRANSPORT` | Event transport: `memory` (default) or `outbox` (durable event bus, Phase 2 — writes an `event_outbox` row inside each write's transaction) | `memory` |
| `PLACE_EVENT_RELAY` | Phase-3 outbox relay worker on/off (truthy `1`/`true`/`yes`/`on`); runs only when transport is also `outbox` | off |
| `PLACE_EVENT_RELAY_INTERVAL_SECS` | Relay drain poll interval in seconds (floored at 1) | `5` |
| `PLACE_EVENT_RETENTION_DAYS` | Outbox row TTL, enforced by the Phase-3 relay's retention purge | `7` |
| `PLACE_FLUVIO_ENDPOINT` | Real-broker relay sink (`FluvioSink`, needs the `fluvio` Cargo feature) | unset (no-broker `LoggingSink`) |
| `PLACE_INTEGRITY_MAC_KEY` / `_KEY_FILE` | Keyed integrity MAC root key (hex) / a file holding it (`_KEY_FILE` wins) — unset means no MAC is written and `/api/records/verify` + `/api/audit/verify` report `mac_absent` | unset |

## Testing

```bash
# 212 unit tests (models, matching, validation, privacy, search,
# streaming, auth, FHIR, compliance/integrity, config, db, relay);
# run for the live count.
cargo test --lib

# 14 bridge tests pinning the service ↔ canonical place-matcher
# contract (GLN deterministic short-circuit, OSM identifier routing,
# geo distance ranking, PlaceType mapping).
cargo test --test duplicate_detection

# 86 integration tests in tests/ (72 in integration_*.rs + 14 bridge).
cargo test --tests

# 16 criterion benchmarks (matching, validation, search, privacy).
cargo bench
```

See [`agents/testing.md`](agents/testing.md) for the layout.

## Matching at a glance

| Component       | Weight | Algorithm                                              |
| --------------- | -----: | ------------------------------------------------------ |
| Name            |   0.35 | Jaro-Winkler + Soundex phonetic bonus                  |
| Geo coordinates |   0.25 | Haversine distance, sigmoid decay                      |
| Address         |   0.20 | Weighted postal / locality / street / region / country |
| Place type      |   0.10 | Exact enum match                                       |
| Identifier      |   0.10 | Type + value exact                                     |

Deterministic short-circuit: exact GLN match → 1.0.

See [`agents/matching.md`](agents/matching.md) for the per-component
detail.

## Compliance

- **GDPR**: right of access via `GET /api/places/{id}/export`;
  right to erasure via soft-delete + `/masked` view.
- **Data protection**: phone / fax / coordinate masking for places
  representing private residences; full audit trail.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only
OR GPL-3.0-only.
