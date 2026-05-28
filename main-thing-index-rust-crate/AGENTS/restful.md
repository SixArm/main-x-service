# RESTful API reference — Main Thing Index

## Library API

The crate exposes a public library API for use in Rust applications.

### Models

```rust
use main_thing_index::models::thing::Thing;
use main_thing_index::models::address::PostalAddress;
use main_thing_index::models::geo::GeoCoordinates;
use main_thing_index::models::thing_type::ThingType;
use main_thing_index::models::identifier::{ThingIdentifier, IdentifierType};
use main_thing_index::models::amenity::AmenityFeature;
use main_thing_index::models::opening_hours::{OpeningHoursSpecification, DayOfWeek};
use main_thing_index::models::consent::{Consent, ConsentType, ConsentStatus};
```

### Matching

```rust
use main_thing_index::matching::scoring::{compute_match, MatchWeights, MatchResult, MatchConfidence};
use main_thing_index::matching::name::name_similarity;
use main_thing_index::matching::address::address_similarity;
use main_thing_index::matching::geo::{geo_similarity, geo_similarity_with_reference, within_radius};
use main_thing_index::matching::identifier::{identifier_similarity, has_gln_match};
use main_thing_index::matching::phonetic::{soundex, soundex_match};
```

### Validation

```rust
use main_thing_index::validation::{validate_thing, normalize_thing, ValidationError};
```

### Privacy

```rust
use main_thing_index::privacy::{mask_thing, gdpr_export};
```

## Usage examples

### Create and validate a thing

```rust
let mut thing = Thing::new("Central Park");
thing.address = Some(PostalAddress {
    street_address: Some("14 E 60th St".into()),
    address_locality: Some("new york".into()),
    address_region: Some("ny".into()),
    address_country: Some("us".into()),
    postal_code: Some("10022".into()),
});
thing.geo = Some(GeoCoordinates::new(40.7829, -73.9654));
thing.thing_type = Some(ThingType::Park);

// Validate
let errors = validate_thing(&thing);
assert!(errors.is_empty());

// Normalize address
normalize_thing(&mut thing);
// address_locality is now "New York", address_region is "NY", etc.
```

### Match two things

```rust
let a = Thing::new("Central Park");
let b = Thing::new("Centrl Park"); // typo

let result = compute_match(&a, &b, &MatchWeights::default());
println!("Score: {:.2}", result.score);            // 0.96+
println!("Confidence: {:?}", result.confidence);   // Certain
println!("Name score: {:.2}", result.breakdown.name_score);
```

### Privacy masking

```rust
let mut thing = Thing::new("Sensitive Thing");
thing.telephone = Some("+1-555-867-5309".into());
thing.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));

let masked = mask_thing(&thing);
// telephone: "+1-555-867****"
// geo: (40.78, -73.97) — rounded to ~1km precision

let export = gdpr_export(&thing); // Full JSON for data portability
```

### Geo distance and radius search

```rust
let nyc = GeoCoordinates::new(40.7128, -74.0060);
let lax = GeoCoordinates::new(33.9425, -118.4081);

let distance_m = nyc.distance_to(&lax);          // ~3,944,000 m
let within_5km = within_radius(&nyc, &lax, 5000.0); // false
let similarity = geo_similarity(&nyc, &lax);     // ~0.0003
```

## RESTful API endpoints

| Method | Path                      | Description           |
| ------ | ------------------------- | --------------------- |
| GET    | `/api/health`             | Health check          |
| POST   | `/api/things`             | Create thing          |
| GET    | `/api/things/{id}`        | Get thing             |
| PUT    | `/api/things/{id}`        | Update thing          |
| DELETE | `/api/things/{id}`        | Soft delete thing     |
| GET    | `/api/things/search`      | Search things         |
| POST   | `/api/things/match`       | Match things          |
| POST   | `/api/things/duplicates`  | Check for duplicates  |
| POST   | `/api/things/merge`       | Merge things          |
| POST   | `/api/things/deduplicate` | Batch deduplication   |
| GET    | `/api/things/{id}/export` | GDPR data export      |
| GET    | `/api/things/{id}/masked` | Masked thing view     |
| GET    | `/api/things/{id}/audit`  | Audit logs            |
| GET    | `/api/audit/recent`       | Recent audit activity |
| GET    | `/api/audit/user`         | User audit logs       |

## Web UI endpoints

| Method | Path                                  | Description              |
| ------ | ------------------------------------- | ------------------------ |
| GET    | `/`                                   | Home page                |
| GET    | `/things`                             | Thing index              |
| GET    | `/things/search/partial?q=…`          | HTMX search fragment     |
| GET    | `/static/css/lily.css`                | NHS UK theme that styles Lily HTML Headless components |
| GET    | `/static/js/htmx.min.js`              | HTMX 2.0.4               |
| GET    | `/static/js/alpine.min.js`            | Alpine 3.14.8            |
