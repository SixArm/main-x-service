# RESTful API reference

## Library API

The crate exposes a public library API for use in Rust applications.

### Models

```rust
use place_service::models::place::Place;
use place_service::models::address::PostalAddress;
use place_service::models::geo::GeoCoordinates;
use place_service::models::place_type::PlaceType;
use place_service::models::identifier::{PlaceIdentifier, IdentifierType};
use place_service::models::amenity::AmenityFeature;
use place_service::models::opening_hours::{OpeningHoursSpecification, DayOfWeek};
use place_service::models::consent::{Consent, ConsentType, ConsentStatus};
```

### Matching

```rust
use place_service::matching::scoring::{compute_match, MatchWeights, MatchResult, MatchConfidence};
use place_service::matching::name::name_similarity;
use place_service::matching::address::address_similarity;
use place_service::matching::geo::{geo_similarity, geo_similarity_with_reference, within_radius};
use place_service::matching::identifier::{identifier_similarity, has_gln_match};
use place_service::matching::phonetic::{soundex, soundex_match};
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
use place_service::validation::{validate_place, normalize_place, ValidationError};
```

### Privacy

```rust
use place_service::privacy::{mask_place, gdpr_export};
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

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../../agents/share/api-versioning.md).


| Method | Path                      | Description           |
| ------ | ------------------------- | --------------------- |
| GET    | `/api/health`             | Health check          |
| GET    | `/api/whoami`             | Echo the verified bearer-token claims (`401` without a valid token) |
| POST   | `/api/places`             | Create place          |
| GET    | `/api/places/{id}`        | Get place             |
| PUT    | `/api/places/{id}`        | Update place          |
| DELETE | `/api/places/{id}`        | Soft delete place     |
| GET    | `/api/places/search`      | Search places         |
| POST   | `/api/places/match`       | Match places          |
| POST   | `/api/places/check-duplicates` | Check for duplicates |
| POST   | `/api/places/merge`       | Merge places          |
| POST   | `/api/places/deduplicate` | Batch deduplication   |
| GET    | `/api/places/review-queue` | Stored review queue (filter `status`, `limit`) |
| POST   | `/api/places/review-queue/{id}/decision` | Decide a pending review item (`confirmed` / `rejected`) |
| GET    | `/api/places/{id}/export` | GDPR data export      |
| GET    | `/api/places/{id}/masked` | Masked place view     |
| GET    | `/api/places/{id}/audit`  | Audit logs            |
| GET    | `/api/audit/recent`       | Recent audit activity |

### FHIR R5 Endpoints

HL7 FHIR R5 surface mapping places to the FHIR `Location` resource,
mounted by [`src/controllers/fhir.rs`](../src/controllers/fhir.rs)
(`routes()`, prefix `/fhir`):

| Method | Path                  | Description              |
| ------ | --------------------- | ------------------------ |
| GET    | `/fhir/metadata`      | CapabilityStatement      |
| POST   | `/fhir/Location`      | Create FHIR Location     |
| GET    | `/fhir/Location`      | Search FHIR Locations    |
| GET    | `/fhir/Location/{id}` | Get FHIR Location        |
| PUT    | `/fhir/Location/{id}` | Update FHIR Location     |
| DELETE | `/fhir/Location/{id}` | Delete FHIR Location (soft) |

### Auth

Bearer tokens are PASETO `v4.public` (Ed25519) minted by the central
authentication-service and verified **offline** against its published
key set (`/.well-known/paseto-keys`) via the `authentication-verifier`
crate — no shared secret, no introspection call. Configure with
`PLACE_PASETO_KEYS_URL` (boot-time HTTP fetch of the key set) or
`PLACE_PASETO_KEYS` (key-set JSON via env), plus `PLACE_TOKEN_ISSUER`
and `PLACE_TOKEN_AUDIENCE` (defaults `authentication-service` /
`main-x-service`). Handlers opt in by taking an `AuthUser`
argument (`src/api/rest/auth.rs`).

Key-set precedence (`state::boot_verifier`, run in `after_routes`
before the routers/middleware capture the verifier): when
`PLACE_PASETO_KEYS_URL` is set (non-blank) the key set is fetched over
HTTP **once at boot** and, on success, **wins** over
`PLACE_PASETO_KEYS`; a fetch failure logs a warning and falls back to
the env path. Unset/blank ⇒ `PLACE_PASETO_KEYS`, else an empty
reject-all key set. The service **always boots**; there is no refresh
loop (key-rotation re-fetch is a roadmap item, spec §15).

#### Blanket enforcement (default-off)

Setting `PLACE_REQUIRE_AUTH` truthy (`1`/`true`/`yes`/`on`,
case-insensitive; anything else — unset, blank, `0`, junk — means
off) turns on a blanket middleware that requires a valid PASETO
bearer token on **every** route except the public allow-list:

| Public path | Why |
| --- | --- |
| `/api/health` | Liveness probe (this crate's surface) |
| `/_health`, `/_ping` | loco default health routes |
| `/api-docs/openapi.json` | Raw OpenAPI document |
| `/swagger-ui*` (prefix) | Swagger UI + static assets |
| `/metrics.prom` | Prometheus scrape (root-mounted, outside `/api`) |

The list is the `PUBLIC_PATHS` / `PUBLIC_PATH_PREFIXES` constants in
`src/api/rest/auth.rs`; the pure `auth::enforce` decision is layered
via `axum::middleware::from_fn_with_state` on both router surfaces
(`create_router` and the loco router in `src/app.rs`). The flag is
read once at router construction — restart the service to change it.
Unauthenticated requests to any non-public path return `401`. See
`agents/share/jwt-enforcement.md` for the family-wide contract.

#### Authorization (ABAC)

Inside the same guard (so only when `PLACE_REQUIRE_AUTH` is on), a
verified token is authorized by **attribute-based access control**
per `agents/share/authorization-attributes.md`: the request's action
is derived from the HTTP method plus the crate's destructive named
POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`, `/deduplicate`,
`/import`), and the shared engine in `authentication-verifier` 0.3
evaluates the policy over the token's `attrs` claim. Configure with
`PLACE_ABAC_POLICY` (inline JSON) or `PLACE_ABAC_POLICY_FILE` (path);
unset or unparsable ⇒ warn-log + the built-in default policy (any
authenticated subject reads; `access=write` writes; `access=admin`
adds DELETE/merge/deduplicate; `svc=true` does everything). Read once
at router construction — restart to change. `401` = missing/bad
credential; `403` = valid credential, policy denied (the body names
the deciding rule).
