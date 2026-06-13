## 5. Domain Model

The place entity has **one canonical domain model and three
representations**. The service's Rust model is canonical; the matcher
and front-end representations are projections of it.

### 5.1 Canonical `Place` (service)

Defined in the service crate (`src/models/place.rs`); field-by-field
reference in
[`place-service-rust-crate/AGENTS/models.md`](../place-service-rust-crate/AGENTS/models.md).
Material aspects (service [spec §5](../place-service-rust-crate/spec/05-domain-model.md)):

- **Identity** — UUID `id` + `identifiers: Vec<PlaceIdentifier>`
  (GLN 13-digit, FIPS, GNIS, OSM ID, custom); `global_location_number`
  and `branch_code` shortcuts.
- **Names** — `name` (required), `alternate_name`, `description`.
- **Classification** — `place_type` enum (`LocalBusiness`,
  `CivicStructure`, `AdministrativeArea`, `Landform`, `Hospital`,
  `School`, …, `Other(String)`) + `keywords`.
- **Address** — `PostalAddress` (street / locality / region / country /
  postal code).
- **Geo** — `GeoCoordinates` (`latitude`, `longitude`, `elevation`),
  WGS 84 decimal degrees.
- **Hierarchy** — `contained_in_place` (parent UUID) + transitive
  `contains_place`.
- **Contact & operational** — `telephone`, `fax_number`, `url`,
  opening hours, amenities, accessibility flags, capacity.
- **Registry plumbing** — `same_as` URLs, `is_deleted` / `deleted_at`,
  `created_at`, `updated_at`.

### 5.2 Matcher `Place` (flat builder shape)

Defined in the matcher crate
([spec §3](../place-matcher-rust-crate/spec/03-data-model.md)): a flat,
`#[non_exhaustive]`, 15-field record — `name`, `alternate_names`,
bare `latitude` / `longitude`, `category` (34-variant
`PlaceCategory`), `place_ids: Vec<PlaceId>` (scheme + value),
`address`, `phone`, `email`, `country_code_as_iso_3166_1_alpha_2`,
elevation / altitude / area, `maximum_capacity_count`, and an
explicitly **unscored** `local_id`. Constructed via `Place::builder()`.

### 5.3 Service ↔ matcher DTO contract (the adapter)

The service embeds the matcher (declared in `Cargo.toml`, re-exported
from `src/matching/mod.rs` as `matcher_lib`) and bridges via
[`src/matching/adapter.rs`](../place-service-rust-crate/src/matching/adapter.rs):
`to_matcher_place(&service::Place) -> place_matcher::Place`.

Routing rules (normative; pinned by
[`tests/duplicate_detection.rs`](../place-service-rust-crate/tests/duplicate_detection.rs)):

- `name` → `name`; `alternate_name` → first entry of `alternate_names`.
- `place_type` (12-variant service enum) → `category` (34-variant
  matcher vocabulary), `Other(s)` flowing through.
- `address.street_address` → `line1`; `address_locality` → `city`;
  `address_region` → `county`; `postal_code` → `postcode`;
  `address_country` → `country` (and
  `country_code_as_iso_3166_1_alpha_2` when 2 characters).
- `geo.latitude` / `.longitude` / `.elevation` → bare `f64` slots +
  `elevation_as_metre`.
- `telephone` → `phone`.
- `global_location_number` → `add_place_id(Other("GLN"), value)`;
  `branch_code` → `add_place_id(Other("BranchCode"), value)`.
- `identifiers[]` routed to `PlaceIdScheme` via `map_identifier_scheme`
  (`OpenStreetMap` → `OsmNode`; `Fips` / `Gnis` / `Custom(s)` →
  `Other(name)`).
- `maximum_attendee_capacity` → `maximum_capacity_count`.

The projection is **lossy by design**: registry-only fields (`id`,
`is_deleted`, timestamps, `keywords`, `amenity_features`,
`opening_hours`, `description`, `fax_number`, `url`, access flags, …)
are dropped — they have no matcher counterpart. Full rationale:
service [spec §6.2](../place-service-rust-crate/spec/06-functional-requirements.md).

### 5.4 Front-end TypeScript types

The front-end mirrors the service's wire format in
`src/lib/api/types.ts` and unwraps the shared envelope in
`src/lib/api/client.ts`. The service model is upstream: if a field
changes in the service, the front-end types MUST be fixed in the same
change cycle (front-end
[`AGENTS.md`](../place-front-end-with-svelte/AGENTS.md)).

### 5.5 Shared invariants

All subprojects MUST uphold (service [spec §5.3](../place-service-rust-crate/spec/05-domain-model.md)):

- `name` is non-empty.
- `latitude ∈ [-90, 90]`, `longitude ∈ [-180, 180]` when present.
- GLN is exactly 13 digits and passes the GLN check-digit algorithm.
- An address, when present, carries at least one of locality, postal
  code, or country.
- A place has at most one `contained_in_place`; hierarchy cycles are
  rejected.
- `PlaceId` values are **scheme-local** — never cross-matched across
  schemes (matcher spec §3.5; the adapter routes, it does not coerce).
- Soft delete is the only delete, end to end: the service never
  row-deletes, and the front-end never offers hard delete.
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown.
