# Place Service — Index

Centralised registry of geographic places: hospitals, clinics,
branches, civic structures, landforms, administrative areas, business
locations. Domain model aligned with
[schema.org/Place](https://schema.org/Place). Probabilistic +
deterministic matching, geo-radius search (Haversine + bounding-box),
real-time and batch deduplication, GDPR Article 15 export, audit
trail.

This page is a **navigation aid with worked examples**. For canonical
behaviour, read [`spec.md`](spec.md).

## Documentation map

| File | Role |
|------|------|
| [`spec.md`](spec.md) | **Single source of truth.** What the system does, how it is built, NFRs, tasks (§13), open questions (§16). |
| [`README.md`](README.md) / [`CLAUDE.md`](CLAUDE.md) | User-facing intro — must stay consistent with the spec. |
| [`AGENTS.md`](AGENTS.md) | Agent-facing entry point — `AGENTS/*` directory + shared docs. |
| [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md) | The SDD discipline this crate practises. |
| [`AGENTS/models.md`](AGENTS/models.md) | Field-by-field domain model reference. |
| [`AGENTS/matching.md`](AGENTS/matching.md) | Match weights, components, deterministic rules, Soundex. |
| [`AGENTS/restful.md`](AGENTS/restful.md) | Endpoint catalogue + library API. |
| [`AGENTS/testing.md`](AGENTS/testing.md) | Unit / integration / benchmark layout. |
| [`agents/share/*`](../agents/share/) | Project-wide cross-crate references. |

## Quick start

```bash
# REST API
cargo run --release

# Web UI (Loco / Tera / HTMX / Alpine / Lily)
cargo run --bin web                    # → http://0.0.0.0:5150
PORT=5180 cargo run --bin web

# Tests
cargo test --lib                       # 104 unit
cargo test --tests                     # 67 integration
cargo bench                            # 16 Criterion benchmarks
```

## URL surface (REST)

| Method | Path | Notes |
|---|---|---|
| GET | `/api/health` | Liveness |
| POST | `/api/places` | Create — `409` on detected duplicate |
| GET | `/api/places/{id}` | Read |
| PUT | `/api/places/{id}` | Update |
| DELETE | `/api/places/{id}` | Soft delete |
| GET | `/api/places/search` | Full-text + fuzzy |
| GET | `/api/places/nearby?lat=&lon=&radius_km=` | **Geo-radius search** |
| POST | `/api/places/match` | Score against candidates |
| POST | `/api/places/duplicates` | Real-time dup check |
| POST | `/api/places/merge` | Merge survivor + duplicate |
| POST | `/api/places/deduplicate` | Batch dedup scan |
| GET | `/api/places/{id}/masked` | Privacy view |
| GET | `/api/places/{id}/export` | GDPR Art. 15 export |
| GET | `/api/places/{id}/audit` | Per-record audit |
| GET | `/api/audit/recent` | System-wide recent audit |
| GET | `/api/audit/user` | Per-user audit |

This crate does **not** expose a FHIR R5 surface. See
[`spec.md §9`](spec.md#9-api-surface).

## Worked examples

### Create a place (Central Park)

```bash
curl -X POST http://localhost:8080/api/places \
  -H 'content-type: application/json' \
  -d '{
    "name": "Central Park",
    "alternate_name": "The Central Park",
    "description": "Urban park in Manhattan, New York City",
    "place_type": "Park",
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

If an existing place matches deterministically (same GLN) or is
within proximity threshold with a name match, the response is
`409 Conflict` with the candidate matches.

### Create a place with GLN

```bash
curl -X POST http://localhost:8080/api/places \
  -H 'content-type: application/json' \
  -d '{
    "name": "Acme Distribution Center 42",
    "place_type": "LocalBusiness",
    "global_location_number": "0614141999996",
    "geo": { "latitude": 37.7749, "longitude": -122.4194 }
  }'
```

### Check for duplicates

```bash
curl -X POST http://localhost:8080/api/places/duplicates \
  -H 'content-type: application/json' \
  -d '{
    "name": "Central Park",
    "address": {
      "address_locality": "New York",
      "address_region": "NY"
    },
    "geo": { "latitude": 40.7829, "longitude": -73.9654 }
  }'
```

### Search (full-text)

```bash
curl "http://localhost:8080/api/places/search?\
q=Central+Park&limit=10&offset=0&fuzzy=true&mask_sensitive=true"
```

### Geo-radius search

```bash
curl "http://localhost:8080/api/places/nearby?\
lat=40.7829&lon=-73.9654&radius_km=2.0&limit=20"
```

Returns places within 2 km of the supplied coordinates, sorted by
distance ascending. Today: Haversine + bounding-box pre-filter
(app-side). PostGIS-backed spatial indexing is queued as
[`spec.md` T-1](spec.md#13-tasks).

### Match a place

```bash
curl -X POST http://localhost:8080/api/places/match \
  -H 'content-type: application/json' \
  -d '{
    "name": "Centrl Park",
    "address": { "address_locality": "New York" },
    "geo": { "latitude": 40.783, "longitude": -73.965 },
    "threshold": 0.7
  }'
```

Returns ranked candidates with `score`, `confidence`, and a
per-component `breakdown` (`name_score` / `geo_score` /
`address_score` / `place_type_score` / `identifier_score`).

### Merge

```bash
curl -X POST http://localhost:8080/api/places/merge \
  -H 'content-type: application/json' \
  -d '{
    "main_place_id": "11111111-1111-1111-1111-111111111111",
    "duplicate_place_id": "22222222-2222-2222-2222-222222222222",
    "merge_reason": "Confirmed duplicate"
  }'
```

### Batch deduplication

```bash
curl -X POST http://localhost:8080/api/places/deduplicate \
  -H 'content-type: application/json' \
  -d '{
    "threshold": 0.70,
    "auto_merge_threshold": 0.95,
    "max_candidates": 50
  }'
```

### GDPR Article 15 export

```bash
curl "http://localhost:8080/api/places/{id}/export"
```

### Masked view

```bash
curl "http://localhost:8080/api/places/{id}/masked"
```

Returns the place with telephone / fax masked and geo coordinates
rounded to 2 decimal places (~1 km precision).

## Library API examples

### Validate and normalise

```rust
use place_service::models::place::Place;
use place_service::models::address::PostalAddress;
use place_service::models::geo::GeoCoordinates;
use place_service::models::place_type::PlaceType;
use place_service::validation::{validate_place, normalize_place};

let mut place = Place::new("Central Park");
place.address = Some(PostalAddress {
    street_address: Some("14 E 60th St".into()),
    address_locality: Some("new york".into()),
    address_region: Some("ny".into()),
    address_country: Some("us".into()),
    postal_code: Some("10022".into()),
});
place.geo = Some(GeoCoordinates::new(40.7829, -73.9654));
place.place_type = Some(PlaceType::Park);

let errs = validate_place(&place);
assert!(errs.is_empty(), "validation failed: {errs:?}");

normalize_place(&mut place);
// address_locality → "New York", address_region → "NY", country → "US"
```

### Match two places

```rust
use place_service::matching::scoring::{compute_match, MatchWeights};

let a = Place::new("Central Park");
let b = Place::new("Centrl Park");  // typo

let result = compute_match(&a, &b, &MatchWeights::default());
println!("score={:.3} confidence={:?}", result.score, result.confidence);
println!("  name: {:.3}", result.breakdown.name_score);
println!("  geo:  {:.3}", result.breakdown.geo_score);
```

### Match via the canonical `place-matcher` bridge

Use the sibling `place-matcher` crate as the reference algorithm. The
service re-exports it as `matcher_lib`, and `adapter::to_matcher_place`
projects the service's domain model into the matcher's input shape
(including national-identifier routing by FHIR `system` URI, address
field renaming, and identifier-scheme dispatch).

```rust,no_run
use place_service::matching::adapter::to_matcher_place;
use place_service::matching::matcher_lib::{Confidence, MatchConfig, MatchingEngine};
use place_service::models::*;

let mut a = Place::new("Central Park");
a.global_location_number = Some("0614141999996".into());
a.geo = Some(GeoCoordinates { latitude: 40.7829, longitude: -73.9654, elevation: None });
let mut b = Place::new("Centrl Park"); // typo
b.global_location_number = Some("0614141999996".into()); // same GLN

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_places(&to_matcher_place(&a), &to_matcher_place(&b));

assert!(result.is_match, "near-duplicate should classify as match");
assert_eq!(result.confidence, Confidence::High);
println!("score   = {:.3}", result.score);
println!("conf    = {:?}", result.confidence);
// `result.breakdown` carries per-field Option<f64> for an auditable trail.
```

End-to-end pinning lives in
[`tests/duplicate_detection.rs`](tests/duplicate_detection.rs); the
adapter source is [`src/matching/adapter.rs`](src/matching/adapter.rs).


### Geo distance and radius

```rust
use place_service::models::geo::GeoCoordinates;
use place_service::matching::geo::{geo_similarity, within_radius};

let nyc = GeoCoordinates::new(40.7128, -74.0060);
let lax = GeoCoordinates::new(33.9425, -118.4081);

let distance_m = nyc.distance_to(&lax);              // ~3 944 000 m
let close      = within_radius(&nyc, &lax, 5_000.0); // false
let sim        = geo_similarity(&nyc, &lax);         // ~0.0003
```

### Privacy mask + GDPR export

```rust
use place_service::privacy::{mask_place, gdpr_export};

let mut place = Place::new("Sensitive Place");
place.telephone = Some("+1-555-867-5309".into());
place.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));

let masked = mask_place(&place);
// telephone: "+1-555-867****"
// geo: (40.78, -73.97) — ~1 km precision

let export = gdpr_export(&place);
```

## Configuration

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | _required_ |
| `DATABASE_MIN_CONNECTIONS` / `DATABASE_MAX_CONNECTIONS` | Pool sizes | `2` / `10` |
| `SERVER_HOST` | REST bind address | `0.0.0.0` |
| `SERVER_PORT` | REST port | `8080` |
| `PORT` | Web UI port (`cargo run --bin web`) | `5150` |
| `SEARCH_INDEX_PATH` | Tantivy index directory | `./search_index` |
| `MATCHING_THRESHOLD` | Default match cutoff | `0.7` |
| `OTLP_ENDPOINT` | OpenTelemetry collector | `http://localhost:4317` |
| `OTLP_SERVICE_NAME` | OTel `service.name` | `place-service` |
| `RUST_LOG` | `tracing-subscriber` filter | `info,place_service=info` |

## Project layout

```
src/
├── lib.rs              # Library root
├── main.rs             # Binary entry point
├── models/             # Place, PostalAddress, GeoCoordinates, PlaceType, PlaceIdentifier, …
├── matching/           # name, address, geo, identifier, phonetic, scoring
├── validation/         # validate_place, normalize_place
├── privacy/            # mask_place, gdpr_export
├── api/                # REST + gRPC (stub)
├── web/                # Loco app + Tera views + Axum web router
└── bin/web.rs          # cargo run --bin web

assets/views/           # Tera templates (HTMX + Alpine + Lily)
assets/static/          # lily.css, htmx.min.js, alpine.min.js
config/                 # development.yaml, test.yaml, production.yaml
migrations/             # SeaORM up.sql / down.sql pairs
tests/                  # integration_* (matching, validation, privacy, models, scoring, edge_cases)
benches/                # matching, validation, searching, database_reading/writing, privacy
AGENTS/                 # Reference documentation
```

## Key types

| Type | Module | Description |
|---|---|---|
| `Place` | `models::place` | Core place entity (schema.org/Place) |
| `PostalAddress` | `models::address` | Structured address |
| `GeoCoordinates` | `models::geo` | Lat / lon / elevation with Haversine |
| `PlaceType` | `models::place_type` | Classification enum (LocalBusiness, Park, Hospital, …) |
| `PlaceIdentifier` | `models::identifier` | External IDs (GLN, FIPS, GNIS, OSM, Custom) |
| `AmenityFeature` | `models::amenity` | Place amenity features |
| `OpeningHoursSpecification` | `models::opening_hours` | Operating hours |
| `Consent` | `models::consent` | GDPR consent record |
| `MatchResult` / `MatchBreakdown` | `matching::scoring` | Score + per-component detail |
| `MatchWeights` | `matching::scoring` | Configurable scoring weights |
| `MatchConfidence` | `matching::scoring` | Certain / Probable / Possible / Unlikely |
| `ValidationError` | `validation` | Field + message |

## Key functions

| Function | Module | Description |
|---|---|---|
| `compute_match` | `matching::scoring` | Match two places with weighted scoring |
| `name_similarity` | `matching::name` | Jaro-Winkler |
| `address_similarity` | `matching::address` | Weighted-field comparison |
| `geo_similarity` | `matching::geo` | Haversine + sigmoid decay |
| `geo_similarity_with_reference` | `matching::geo` | Custom reference distance |
| `within_radius` | `matching::geo` | Radius predicate |
| `identifier_similarity` | `matching::identifier` | Exact `(type, value)` |
| `has_gln_match` | `matching::identifier` | GLN deterministic short-circuit |
| `soundex` / `soundex_match` | `matching::phonetic` | 4-char phonetic |
| `validate_place` | `validation` | Required + format + invariants |
| `normalize_place` | `validation` | Title-case + uppercase + abbreviation expansion |
| `mask_place` | `privacy` | Phone / fax / geo masking |
| `gdpr_export` | `privacy` | GDPR Article 15 export |

## Status & roadmap

- **Status** — see [`spec.md §14`](spec.md#14-implementation-status).
- **Tasks** — see [`spec.md §13`](spec.md#13-tasks), including PostGIS
  spatial queries (T-1), OSM import (T-5), reverse-geocoding (T-6),
  GeoJSON export (T-7).
- **Roadmap** — see [`spec.md §15`](spec.md#15-roadmap).
- **Open questions** — see [`spec.md §16`](spec.md#16-open-questions),
  including tile-server choice (OQ-1).

## Compliance

| Standard | Mechanism |
|---|---|
| GDPR Art. 15 | `/api/places/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| schema.org/Place | Domain-model conformance |
| ISO/IEC 27001 | Operational controls (deployment-side) |

## License

Dual-licensed: MIT OR Apache-2.0.
