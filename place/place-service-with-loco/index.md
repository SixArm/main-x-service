# Place Service

A high-performance, enterprise-grade Place Service system built with Rust.

A registry of **place identities** based on
[schema.org/Place](https://schema.org/Place). The Place Service is
the canonical store for geographic places — sites, branches, civic
structures, landforms, administrative areas, business locations —
anything modelled by schema.org/Place.

## Quick start

Prerequisites: Rust 1.93+ (2024 edition), PostgreSQL 18+ with PostGIS.

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
[`AGENTS/restful.md`](AGENTS/restful.md) for the full list. All
endpoints return the standard `{success, data, error}` envelope.

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
    "geo": { "latitude": 40.7829, "longitude": -73.9654 }
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
| `RUST_LOG`           | tracing-subscriber filter  | `info`                  |
| `OTLP_ENDPOINT`      | OpenTelemetry collector    | `http://localhost:4317` |

## Testing

```bash
# 125 unit tests (models, matching components, validation, privacy,
# search, streaming, metrics, api).
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

See [`AGENTS/testing.md`](AGENTS/testing.md) for the layout.

## Matching at a glance

| Component       | Weight | Algorithm                                              |
| --------------- | -----: | ------------------------------------------------------ |
| Name            |   0.35 | Jaro-Winkler + Soundex phonetic bonus                  |
| Geo coordinates |   0.25 | Haversine distance, sigmoid decay                      |
| Address         |   0.20 | Weighted postal / locality / street / region / country |
| Place type      |   0.10 | Exact enum match                                       |
| Identifier      |   0.10 | Type + value exact                                     |

Deterministic short-circuit: exact GLN match → 1.0.

See [`AGENTS/matching.md`](AGENTS/matching.md) for the per-component
detail.

## Compliance

- **GDPR**: right of access via `GET /api/places/{id}/export`;
  right to erasure via soft-delete + `/masked` view.
- **Data protection**: phone / fax / coordinate masking for places
  representing private residences; full audit trail.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only
OR GPL-3.0-only.
