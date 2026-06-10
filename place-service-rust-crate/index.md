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
cd place-service-rust-crate

# Configure (Loco-style YAML in config/, env via PORT / DATABASE_URL):
cp config/development.yaml.example config/development.yaml || true

# Build and run.
cargo run --release

# Service binds on port 8080 by default:
curl http://localhost:8080/api/health
```

## API

REST routes mount under `/api/places/*`. See
[`AGENTS/restful.md`](AGENTS/restful.md) for the full list. All
endpoints return the standard `{success, data, error}` envelope.

Interactive OpenAPI 3 documentation:

- Swagger UI: `http://localhost:8080/swagger-ui`
- Raw spec: `http://localhost:8080/api-docs/openapi.json`

### Worked example

```bash
curl -X POST http://localhost:8080/api/places \
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

Geo-radius search via Haversine + bounding-box pre-filter:

```bash
curl "http://localhost:8080/api/places/search?lat=40.78&lon=-73.96&radius_km=2"
```

## Configuration

Configuration is loaded from `config/{development,test,production}.yaml`
(Loco convention). Environment-overridable variables:

| Variable             | Description                | Default                 |
| -------------------- | -------------------------- | ----------------------- |
| `PORT`               | REST bind port             | `8080`                  |
| `DATABASE_URL`       | Postgres connection string | per config file         |
| `SEARCH_INDEX_PATH`  | Tantivy index directory    | `./data/search_index`   |
| `MATCHING_THRESHOLD` | Probabilistic match cutoff | `0.85`                  |
| `RUST_LOG`           | tracing-subscriber filter  | `info`                  |
| `OTLP_ENDPOINT`      | OpenTelemetry collector    | `http://localhost:4317` |

## Testing

```bash
# 104 unit tests (models, matching components, validation, privacy).
cargo test --lib

# 14 bridge tests pinning the service ↔ canonical place-matcher
# contract (GLN deterministic short-circuit, OSM identifier routing,
# geo distance ranking, PlaceType mapping).
cargo test --test duplicate_detection

# 67 integration tests across the per-pipeline files.
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
