# RESTful API reference

## Library API

The crate exposes a public library API for use in Rust applications.

### Models

```rust
use place_service_rust_crate::models::place::Place;
use place_service_rust_crate::models::address::PostalAddress;
use place_service_rust_crate::models::geo::GeoCoordinates;
use place_service_rust_crate::models::place_type::PlaceType;
use place_service_rust_crate::models::identifier::{PlaceIdentifier, IdentifierType};
use place_service_rust_crate::models::amenity::AmenityFeature;
use place_service_rust_crate::models::opening_hours::{OpeningHoursSpecification, DayOfWeek};
use place_service_rust_crate::models::consent::{Consent, ConsentType, ConsentStatus};
```

### Matching

```rust
use place_service_rust_crate::matching::scoring::{compute_match, MatchWeights, MatchResult, MatchConfidence};
use place_service_rust_crate::matching::name::name_similarity;
use place_service_rust_crate::matching::address::address_similarity;
use place_service_rust_crate::matching::geo::{geo_similarity, geo_similarity_with_reference, within_radius};
use place_service_rust_crate::matching::identifier::{identifier_similarity, has_gln_match};
use place_service_rust_crate::matching::phonetic::{soundex, soundex_match};
```


### Adapter to the canonical `place-matcher` crate

The service embeds the sibling `place-matcher` crate and re-exports it
from `src/matching/mod.rs` as `matcher_lib`. Pair it with
`adapter::to_matcher_place` to score two service records through the
canonical algorithm:

```rust
use place_service::matching::adapter::to_matcher_place;
use place_service::matching::matcher_lib::{MatchingEngine, MatchConfig};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_places(
    &to_matcher_place(&a),
    &to_matcher_place(&b),
);
// result.score: f64 in [0.0, 1.0]
// result.is_match: bool
// result.confidence: High | Medium | Low
// result.breakdown: per-field Option<f64>
```

Field-routing rules are documented inline in
[`src/matching/adapter.rs`](../src/matching/adapter.rs) and pinned by
[`tests/duplicate_detection.rs`](../tests/duplicate_detection.rs).

### Validation

```rust
use place_service_rust_crate::validation::{validate_place, normalize_place, ValidationError};
```

### Privacy

```rust
use place_service_rust_crate::privacy::{mask_place, gdpr_export};
```

## Usage Examples

### Create and validate a place

```rust
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

// Validate
let errors = validate_place(&place);
assert!(errors.is_empty());

// Normalize address
normalize_place(&mut place);
// address_locality is now "New York", address_region is "NY", etc.
```

### Match two places

```rust
let a = Place::new("Central Park");
let b = Place::new("Centrl Park"); // typo

let result = compute_match(&a, &b, &MatchWeights::default());
println!("Score: {:.2}", result.score);        // 0.96+
println!("Confidence: {:?}", result.confidence); // Certain
println!("Name score: {:.2}", result.breakdown.name_score);
```

### Privacy masking

```rust
let mut place = Place::new("Sensitive Place");
place.telephone = Some("+1-555-867-5309".into());
place.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));

let masked = mask_place(&place);
// telephone: "+1-555-867****"
// geo: (40.78, -73.97) - rounded to ~1km precision

let export = gdpr_export(&place); // Full JSON for data portability
```

### Geo distance and radius search

```rust
let nyc = GeoCoordinates::new(40.7128, -74.0060);
let lax = GeoCoordinates::new(33.9425, -118.4081);

let distance_m = nyc.distance_to(&lax); // ~3,944,000 m
let within_5km = within_radius(&nyc, &lax, 5000.0); // false

let similarity = geo_similarity(&nyc, &lax); // ~0.0003
```


### Prometheus metrics

| Method | Path             | Description                                                                  |
| ------ | ---------------- | ---------------------------------------------------------------------------- |
| GET    | `/metrics.prom`  | Prometheus text-exposition format (`text/plain; version=0.0.4`) for scraping |

Served at the application root (not under `/api`), so configure your
scraper with `metrics_path: /metrics.prom`. The metric inventory
(`place_*` CRUD counters, HTTP request counter, latency histograms) is
in [`src/metrics.rs`](../src/metrics.rs). The handler is
[`api::rest::handlers::metrics_prom`](../src/api/rest/handlers.rs),
registered at root via `api::rest::metrics_routes()`.

## RESTful API Endpoints

| Method | Path                      | Description           |
| ------ | ------------------------- | --------------------- |
| GET    | `/api/health`             | Health check          |
| POST   | `/api/places`             | Create place          |
| GET    | `/api/places/{id}`        | Get place             |
| PUT    | `/api/places/{id}`        | Update place          |
| DELETE | `/api/places/{id}`        | Soft delete place     |
| GET    | `/api/places/search`      | Search places         |
| POST   | `/api/places/match`       | Match places          |
| POST   | `/api/places/check-duplicates` | Check for duplicates |
| POST   | `/api/places/merge`       | Merge places          |
| POST   | `/api/places/deduplicate` | Batch deduplication   |
| GET    | `/api/places/{id}/export` | GDPR data export      |
| GET    | `/api/places/{id}/masked` | Masked place view     |
| GET    | `/api/places/{id}/audit`  | Audit logs            |
| GET    | `/api/audit/recent`       | Recent audit activity |
| GET    | `/api/audit/user`         | User audit logs       |
