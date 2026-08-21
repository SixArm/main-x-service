# Domain models — Place entity

One canonical model, three representations. This page gives the shape
and the pointer; field-by-field tables live in the per-crate docs.

## The three representations

| Representation | Where | Shape | Reference |
|---|---|---|---|
| Service `Place` (canonical) | `place-service-with-loco/src/models/place.rs` | schema.org/Place: `name`, `alternate_name`, `place_type`, `PostalAddress`, `GeoCoordinates`, GLN + `identifiers`, hierarchy, opening hours, amenities, soft-delete flags | [service agents/models.md](../place-service-with-loco/agents/models.md) |
| Matcher `Place` (flat builder) | `place-matcher-rust-crate/src/models.rs` | 15 optional fields: `name`, `alternate_names`, bare `latitude`/`longitude`, `category` (34-variant), `place_ids` (scheme + value), `address`, `phone`, `email`, country code, capacity; `#[non_exhaustive]`, built via `Place::builder()` | [matcher spec §3](../place-matcher-rust-crate/spec/03-data-model.md) |
| Front-end TypeScript types | `place-front-end-with-svelte/src/lib/api/types.ts` | Mirrors the service wire format + the response envelope | [front-end AGENTS.md](../place-front-end-with-svelte/AGENTS.md) |

## Key supporting types (service)

`PostalAddress` (street / locality / region / country / postal code),
`GeoCoordinates` (WGS 84 lat/lon + elevation, Haversine
`distance_to`), `PlaceType` enum, `PlaceIdentifier`
(GLN / FIPS / GNIS / OSM / custom), `AmenityFeature`,
`OpeningHoursSpecification`, `Consent`. Tables in
[service agents/models.md](../place-service-with-loco/agents/models.md).

## The adapter (service → matcher projection)

[`src/matching/adapter.rs`](../place-service-with-loco/src/matching/adapter.rs)
exposes `to_matcher_place(&service::Place) -> place_matcher::Place`.
It is **lossy by design** — registry-only fields (id, timestamps,
keywords, amenities, opening hours, fax, url, access flags) drop.
Normative routing rules: entity
[spec §5.3](../spec/05-domain-model.md). Pinned by the bridge tests
([`tests/duplicate_detection.rs`](../place-service-with-loco/tests/duplicate_detection.rs)).

Highlights an agent trips over:

- service `address_region` → matcher `county`; `postal_code` →
  `postcode`; `address_country` → `country` (+ ISO alpha-2 slot when
  2 chars).
- `global_location_number` / `branch_code` become `place_ids` with
  `Other("GLN")` / `Other("BranchCode")` schemes.
- `telephone` → `phone`; the matcher's `email` has no service source
  today.
- matcher `local_id` is **never scored** — do not route registry IDs
  into it expecting a signal.

## Shared invariants

Every representation upholds: non-empty `name`; lat ∈ [-90, 90] /
lon ∈ [-180, 180]; 13-digit check-digit-valid GLN; scheme-local
place-ids (never cross-matched); soft delete only; scores in
[0.00, 1.00] with a per-component breakdown. Full list: entity
[spec §5.5](../spec/05-domain-model.md).

## When a field changes

Service model is upstream. A service field change ripples in the same
change cycle to: the front-end types (entity FR-20), the adapter +
bridge test if the field is match-relevant (FR-19), and the relevant
spec sections (entity [agents/spec-driven-development.md](spec-driven-development.md)).
