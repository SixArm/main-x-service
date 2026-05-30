# Main Thing Service

The Main Thing Service is a generic asset / object registry — one entry per "thing" — modeled loosely on [schema.org/Thing](https://schema.org/Thing). It shares its architecture with the other Main X Index crates: REST + gRPC + server-rendered web UI on top of PostgreSQL, Tantivy, and an event stream.

@agents/share/overview.md

## Features

### Thing identity management

- CRUD on Thing records with soft delete
- Multiple identifiers per thing (GLN, branch code, FIPS, GNIS, OSM ID, custom)
- Multiple names (`name`, `alternate_name`) and `description`
- Structured address (`PostalAddress`)
- Geo coordinates (latitude, longitude, optional elevation)
- Thing type classification (`ThingType`)
- Hierarchy via `contained_in_thing`
- Contact information (telephone, fax, URL)
- Opening hours (`OpeningHoursSpecification`)
- Amenity features (`AmenityFeature`) and accessibility flags
- Automatic event publishing on every CRUD operation

### Thing matching

- **Probabilistic** — weighted name + address + geo + type + identifier
- **Deterministic** — GLN exact match short-circuits to `1.0`
- **Score breakdown** — per-component scores in every match response
- Algorithms: Jaro-Winkler, Levenshtein, Soundex phonetic, Haversine geo

### Data quality & validation

- Required-field enforcement (name)
- Coordinate validation (`latitude ∈ [-90, 90]`, `longitude ∈ [-180, 180]`)
- GLN format validation (13-digit + check digit)
- URL & telephone format checks
- Address validation (locality, postal code, or country required)
- Address standardization (title-case locality, uppercase region/country)
- Coordinate normalization (decimal degrees, WGS 84)
- Validation runs on create / update (returns `422`)

### Web UI (Loco / Tera / HTMX / Alpine / Lily)

See [`agents/share/web-stack.md`](../agents/share/web-stack.md) for the shared web-tier reference.

| Path | Returns |
|------|---------|
| `GET /` | Home page |
| `GET /things` | Thing index |
| `GET /things/search/partial?q=…` | HTMX fragment |
| `GET /static/*` | Lily CSS, HTMX JS, Alpine JS |

@AGENTS/index.md
@AGENTS/matching.md
@AGENTS/models.md
@AGENTS/restful.md
@AGENTS/testing.md

@agents/share/auditability.md
@agents/share/availability.md
@agents/share/match-search-merge.md
@agents/share/observability.md
@agents/share/privacy.md
@agents/share/restful.md
@agents/share/technology.md
@agents/share/web-stack.md

## Quick start

### Local dev

**Prerequisites:** Rust 1.75+, PostgreSQL 15+ (optional for in-memory paths)

```bash
git clone https://github.com/sixarm/main-thing-service-rust-crate.git
cd main-thing-service-rust-crate

# REST API
cargo run --release

# Web UI
cargo run --bin web    # → http://0.0.0.0:5150

# Tests
cargo test --lib
```

## API examples

**Create a thing**

```bash
curl -X POST http://localhost:8080/api/things \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Central Park",
    "alternate_name": "The Central Park",
    "description": "Urban park in Manhattan, New York City",
    "thing_type": "Park",
    "address": {
      "street_address": "14 E 60th St",
      "address_locality": "New York",
      "address_region": "NY",
      "address_country": "US",
      "postal_code": "10022"
    },
    "geo": { "latitude": 40.7829, "longitude": -73.9654 },
    "telephone": "+1-212-310-6600",
    "url": "https://www.centralparknyc.org",
    "is_accessible_for_free": true,
    "public_access": true
  }'
```

**Check for duplicates**

```bash
curl -X POST http://localhost:8080/api/things/check-duplicates \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Central Park",
    "address": { "address_locality": "New York", "address_region": "NY" },
    "geo": { "latitude": 40.7829, "longitude": -73.9654 }
  }'
```

**Search**

```bash
curl "http://localhost:8080/api/things/search?q=Central+Park&limit=10&offset=0&fuzzy=true&mask_sensitive=true"
```

**Match**

```bash
curl -X POST http://localhost:8080/api/things/match \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Centrl Park",
    "address": { "address_locality": "New York" },
    "geo": { "latitude": 40.783, "longitude": -73.965 },
    "threshold": 0.7
  }'
```

**Merge**

```bash
curl -X POST http://localhost:8080/api/things/merge \
  -H "Content-Type: application/json" \
  -d '{
    "main_thing_id": "uuid-main",
    "duplicate_thing_id": "uuid-dup",
    "merge_reason": "Confirmed duplicate"
  }'
```

**Batch deduplication**

```bash
curl -X POST http://localhost:8080/api/things/deduplicate \
  -H "Content-Type: application/json" \
  -d '{ "threshold": 0.7, "auto_merge_threshold": 0.95, "max_candidates": 50 }'
```

## Configuration

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | — |
| `DATABASE_MAX_CONNECTIONS` | Pool max | 10 |
| `DATABASE_MIN_CONNECTIONS` | Pool min | 2 |
| `SERVER_HOST` | REST bind address | `0.0.0.0` |
| `SERVER_PORT` | REST port | `8080` |
| `PORT` | Web UI port | `5150` |
| `SEARCH_INDEX_PATH` | Tantivy index dir | `./search_index` |
| `MATCHING_THRESHOLD` | Default match threshold | `0.7` |
| `RUST_LOG` | Log filter | `info` |

## Testing

```bash
cargo test --lib                 # All unit tests
cargo test --tests               # All integration tests
cargo test --test integration_matching
cargo bench                      # Benchmarks
```

## Security & compliance

- Audit logging for every CRUD operation
- Soft delete (records never truly deleted)
- Non-root containers
- Environment-based secrets (no secrets in code/images)
- Configurable CORS
- Per-field privacy masking
- GDPR data export endpoint
- Consent model with type/status tracking
- Validation on create/update

## License

Dual-licensed: Apache-2.0 OR BSD-3-Clause OR GPL-2-or-later OR MIT.
