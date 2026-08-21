## 5. Domain Model

The place entity has **one canonical domain model and three
representations**. The service's Rust model is canonical; the matcher
and front-end representations are projections of it.

### 5.1 Canonical `Place` (service)

Defined in the service crate (`src/models/place.rs`); field-by-field
reference in
[`place-service-with-loco/agents/models.md`](../place-service-with-loco/agents/models.md).
Material aspects (service [spec §5](../place-service-with-loco/spec/05-domain-model.md)):

- **Identity** — UUID `id` + `identifiers: Vec<PlaceIdentifier>`
  (GLN 13-digit, FIPS, GNIS, OSM ID, custom); `global_location_number`
  and `branch_code` shortcuts.
- **Names** — `name` (required), `alternate_name`, `description`.
- **Classification** — `place_type` enum (`LocalBusiness`,
  `CivicStructure`, `AdministrativeArea`, `Landform`, `Hospital`,
  `School`, …, `Other(String)`) + `keywords`. See [kinds.md](kinds.md)
  for the human-facing catalogue of place kinds across scales (shelf →
  province) and how they map to this classification + the hierarchy.
- **Setting (indoor / outdoor)** — `setting: Option<IndoorOutdoor>`:
  `Indoor` (an enclosed place — e.g. a house, office, room), `Outdoor`
  (an open place — e.g. a park, road, lake), or `Mixed` (both — e.g. a
  stadium with covered stands). Independent of `place_type` / kind.
- **Address** — `PostalAddress` (street / locality / region / country /
  postal code).
- **Geo** — `GeoCoordinates` (`latitude`, `longitude`, `elevation`),
  WGS 84 decimal degrees.
- **Relationships** — typed place-to-place links:
  `relationships: Vec<PlaceRelationship>`, each `{ relation, place_id }`
  referencing another `Place`. `relation` is a `PlaceRelationKind` enum,
  initially **`Contains`** / **`ContainedIn`** (inverses — A `Contains` B
  ⇔ B `ContainedIn` A; e.g. a building **contains** a room). These
  generalise the existing hierarchy fields `contained_in_place` (parent
  UUID) + transitive `contains_place`, and the enum is extensible to other
  kinds (e.g. `AdjacentTo`, `NearBy`, `OverlapsWith`).
- **Contact & operational** — `telephone`, `fax_number`, `url`,
  opening hours, amenities, accessibility flags, capacity.
- **Tags** — `tags: Vec<String>`: a list of short, free-text operator
  labels that **any `Place` can carry**, attached to a record for
  grouping, filtering, triage, or workflow (e.g. `"vip"`, `"review"`,
  `"archived-2026"`, `"fast-track"`). Each tag is a short, trimmed,
  non-empty string; the list is unordered, de-duplicated
  case-insensitively, and defaults to empty. Distinct from `keywords`
  above: **keywords** are descriptive / discovery terms about *what the
  place is* (classification, search), whereas **tags** are
  *user-applied operational labels* for grouping and workflow. Tags are
  a registry attribute **and** a supporting match signal: the matcher
  scores them by set Jaccard over the case-insensitively normalised tag
  sets, weighted `tags_weight` (matcher
  [spec §6.11](../place-matcher-rust-crate/spec/06-per-field-scoring-algorithms.md);
  routed by the adapter §5.3). As a supporting signal, identical tags
  alone do not identify a place.
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
[`src/matching/adapter.rs`](../place-service-with-loco/src/matching/adapter.rs):
`to_matcher_place(&service::Place) -> place_matcher::Place`.

Routing rules (normative; pinned by
[`tests/duplicate_detection.rs`](../place-service-with-loco/tests/duplicate_detection.rs)):

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
- `setting` (indoor / outdoor) → matcher `setting` (exact-match signal,
  matcher data-model + algorithm).
- `tags` → matcher `tags` (supporting signal: scored by set Jaccard over
  the case-insensitively normalised tag sets, weighted `tags_weight`;
  matcher [spec §6.11](../place-matcher-rust-crate/spec/06-per-field-scoring-algorithms.md) +
  [§7](../place-matcher-rust-crate/spec/07-configuration.md)). **Not** a
  lossy-dropped field.

The projection is **lossy by design**: registry-only fields (`id`,
`is_deleted`, timestamps, `keywords`, `amenity_features`,
`opening_hours`, `description`, `fax_number`, `url`, access flags,
`relationships`, …) are dropped — they have no matcher counterpart
(`relationships` is registry-only; it is not a match signal today). Full rationale:
service [spec §6.2](../place-service-with-loco/spec/06-functional-requirements.md).

### 5.4 Front-end TypeScript types

The front-end mirrors the service's wire format in
`src/lib/api/types.ts` and unwraps the shared envelope in
`src/lib/api/client.ts`. The service model is upstream: if a field
changes in the service, the front-end types MUST be fixed in the same
change cycle (front-end
[`AGENTS.md`](../place-front-end-with-svelte/AGENTS.md)).

### 5.5 Shared invariants

All subprojects MUST uphold (service [spec §5.3](../place-service-with-loco/spec/05-domain-model.md)):

- `name` is non-empty.
- `latitude ∈ [-90, 90]`, `longitude ∈ [-180, 180]` when present.
- GLN is exactly 13 digits and passes the GLN check-digit algorithm.
- An address, when present, carries at least one of locality, postal
  code, or country.
- A place has at most one `contained_in_place`; hierarchy cycles are
  rejected. Each `PlaceRelationship` references an existing `Place` and
  is not self-referential; `Contains` / `ContainedIn` stay **acyclic**
  (no place contains itself, directly or transitively) and
  inverse-consistent (A `Contains` B ⇔ B `ContainedIn` A).
- `PlaceId` values are **scheme-local** — never cross-matched across
  schemes (matcher spec §3.5; the adapter routes, it does not coerce).
- `tags` are short, trimmed, non-empty strings; the list is unordered,
  de-duplicated case-insensitively, and defaults to empty. The canonical
  service model (§5.1) is upstream: the matcher DTO and front-end types
  follow in the same change cycle. Tags **are** a supporting match
  signal: the adapter routes them to the matcher's `tags` (§5.3), scored
  by set Jaccard and weighted `tags_weight` (matcher §6.11) — they are
  **not** dropped from the matcher projection.
- Soft delete is the only delete, end to end: the service never
  row-deletes, and the front-end never offers hard delete.
- Match scores are in `[0.00, 1.00]` and always travel with a
  per-component breakdown.
