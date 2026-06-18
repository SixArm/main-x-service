## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete place records.
- Multiple identifiers per place.
- Hierarchy management via `contained_in_place` / `contains_place`.
- Multiple amenity features and opening-hours specifications.
- Automatic event publish on every CRUD.

### 6.2 Matching

Algorithm reference: [`AGENTS/matching.md`](../AGENTS/matching.md).

Default component weights (sum to 1.0):

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.35 | Jaro-Winkler + Soundex phonetic bonus |
| Geo coordinates | 0.25 | Haversine distance with sigmoid decay |
| Address | 0.20 | Weighted postal / locality / street / region / country |
| Place type | 0.10 | Exact match |
| Identifier | 0.10 | Type + value exact |

Deterministic short-circuit: exact GLN match → 1.0.

Match quality (configurable thresholds):

| Quality | Score |
|---|---|
| Certain | ≥ 0.95 |
| Probable | ≥ 0.80 |
| Possible | ≥ 0.60 |
| Unlikely | < 0.60 |

#### Interoperability with `place-matcher`

The service embeds the sibling `place-matcher` crate (registry
dependency `place-matcher = "0.6.1"` in `Cargo.toml`) and re-exports it from
`src/matching/mod.rs` as `matcher_lib`. The matcher crate is the
**canonical reference algorithm** — it carries the full
`PlaceCategory` vocabulary (34 variants), `PlaceIdScheme` for
external IDs (Google, OSM nodes/ways/relations, GeoNames, Wikidata,
Foursquare, Here, Mapbox, …), Haversine + Gaussian-decay geo scoring,
weight renormalisation for missing fields, and three tuned config
presets (`strict` / `default` / `lenient`) that the in-service
matcher does not duplicate.

Bridge: [`src/matching/adapter.rs`](../src/matching/adapter.rs) exposes
`to_matcher_place(&service::Place) -> place_matcher::Place`. The
projection lifts the service's schema.org-shaped record (`PostalAddress`,
`GeoCoordinates`, `PlaceType`, `Vec<PlaceIdentifier>`) into the
matcher's flat builder shape:

- `name` → `name`; `alternate_name` (Option) → first entry of `alternate_names`
- `place_type` → `category` (12-variant service enum → 34-variant matcher vocabulary, with `Other(s)` flowing through)
- `address.street_address` → `line1`; `address_locality` → `city`; `address_region` → `county`; `postal_code` → `postcode`; `address_country` → `country` (and `country_code_as_iso_3166_1_alpha_2` if 2-character)
- `geo.latitude` / `.longitude` / `.elevation` → bare `f64` slots + `elevation_as_metre`
- `telephone` → `phone`
- `global_location_number` → `add_place_id(Other("GLN"), value)`
- `branch_code` → `add_place_id(Other("BranchCode"), value)`
- `identifiers[]` routed to `PlaceIdScheme` via `map_identifier_scheme` (`OpenStreetMap` → `OsmNode`; `Fips` / `Gnis` / `Custom(s)` → `Other(name)`)
- `maximum_attendee_capacity` → `maximum_capacity_count`

Registry-only fields (`id`, `is_deleted`, `created_at`, `keywords`,
`amenity_features`, `opening_hours`, `description`, `fax_number`,
`url`, `public_access`, `smoking_allowed`, …) are dropped — they
have no matcher counterpart. See
[`AGENTS/matching.md`](../AGENTS/matching.md) for the in-service
algorithm and the matcher crate's
[`spec.md §5–§7`](../../place-matcher-rust-crate/spec/index.md) for the
canonical algorithm.

### 6.3 Search

Tantivy across `name`, `alternate_name`, `identifiers`, address
components, `place_type`. Full-text + fuzzy + boolean. Search query
parameters delivered today: `q`, `limit`, `fuzzy`, `mask_sensitive`.

**Geo-radius search** (`GET /api/places/nearby?lat=&lon=&radius_km=`)
and `offset` pagination are **not yet delivered** — tracked as §13
T-9. The matching primitive
[`within_radius`](../src/matching/geo.rs) (Haversine) exists and is
unit-tested, but no HTTP route or `SearchQuery` geo parameters wire it
up yet. PostGIS-backed spatial queries are a further roadmap item
(§13 T-1).

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/places` when an existing
  place is within proximity threshold + name match.
- Explicit `POST /api/places/check-duplicates`.
- Batch `POST /api/places/deduplicate`.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge transfers identifiers, alternate names, amenity features,
  opening-hours specifications, `same_as` URLs, hierarchy links;
  appends the duplicate's name as `alternate_name` on the survivor;
  adds a `Replaces` link; soft-deletes the duplicate; records a JSON
  snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

Required `name`; coordinate bounds; GLN check digit; URL protocol;
telephone format; opening-hours times (24-hour `HH:MM`); address
completeness; place hierarchy acyclicity.
Address normalised (title-case locality, uppercase region / country,
expand St. / Ave. / Rd. abbreviations). Coordinate normalisation
(decimal degrees, WGS 84). Failed validation → `422`.

### 6.6 Privacy

Per-field masking for sensitive contact fields: phone, fax, exact
coordinates rounded to 2 decimal places (~1 km precision). GDPR
Article 15 export at `GET /api/places/{id}/export`. Consent model
where the place represents a private residence.

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp.

