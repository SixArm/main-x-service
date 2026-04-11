# RESTful API reference

## Library API

The crate exposes a public library API for use in Rust applications.

### Models

```rust
use main_thing_index_rust_crate::models::thing::Thing;
use main_thing_index_rust_crate::models::address::PostalAddress;
use main_thing_index_rust_crate::models::geo::GeoCoordinates;
use main_thing_index_rust_crate::models::thing_type::PlaceType;
use main_thing_index_rust_crate::models::identifier::{PlaceIdentifier, IdentifierType};
use main_thing_index_rust_crate::models::amenity::AmenityFeature;
use main_thing_index_rust_crate::models::opening_hours::{OpeningHoursSpecification, DayOfWeek};
use main_thing_index_rust_crate::models::consent::{Consent, ConsentType, ConsentStatus};
```

### Matching

```rust
use main_thing_index_rust_crate::matching::scoring::{compute_match, MatchWeights, MatchResult, MatchConfidence};
use main_thing_index_rust_crate::matching::name::name_similarity;
use main_thing_index_rust_crate::matching::address::address_similarity;
use main_thing_index_rust_crate::matching::geo::{geo_similarity, geo_similarity_with_reference, within_radius};
use main_thing_index_rust_crate::matching::identifier::{identifier_similarity, has_gln_match};
use main_thing_index_rust_crate::matching::phonetic::{soundex, soundex_match};
```

### Validation

```rust
use main_place_index_rust_crate::validation::{validate_thing, normalize_thing, ValidationError};
```

### Privacy

```rust
use main_thing_index_rust_crate::privacy::{mask_thing, gdpr_export};
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
| POST   | `/api/places/duplicates`  | Check for duplicates  |
| POST   | `/api/places/merge`       | Merge places          |
| POST   | `/api/places/deduplicate` | Batch deduplication   |
| GET    | `/api/places/{id}/export` | GDPR data export      |
| GET    | `/api/places/{id}/masked` | Masked place view     |
| GET    | `/api/places/{id}/audit`  | Audit logs            |
| GET    | `/api/audit/recent`       | Recent audit activity |
| GET    | `/api/audit/user`         | User audit logs       |
