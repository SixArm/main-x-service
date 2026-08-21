# Domain Model Reference

Based on [schema.org/Place](https://schema.org/Place).

## Place

`src/models/place.rs`

Core entity representing a physical place.

**Wire optionality (`#[serde(default)]`).** `id`, `name`,
`is_deleted`, `created_at`, `updated_at`, and every collection field
(`keywords`, `identifiers`, `amenity_features`, `opening_hours`) carry
`#[serde(default)]` — every one of them is optional on
`POST`/`PUT /api/places` bodies (QA-SERVER-FIELDS, 2026-08-04). The
server-managed ones (everything in that list except `name`) are
**ignored on input and stamped by the server regardless of what a
client sends**: the handler mints a fresh `id` when the wire value is
nil, and the repository stamps `created_at`/`updated_at` on
insert/update. `name` is `#[serde(default)]` too, but for a different
reason — it is the one field the server does *not* own, so an omitted
value now reaches `validate_place` (`422 validation_error`) instead of
being refused by the JSON extractor before any handler code runs. See
[`spec/09-api-surface.md`](../spec/09-api-surface.md) for the full
contract.

| Field | Type | Server-managed? | Description |
|-------|------|:---:|-------------|
| `id` | `Uuid` | server-managed | Unique identifier (auto-generated v4 if the wire value is nil) |
| `name` | `String` | — (required, `422` if blank) | Primary name |
| `alternate_name` | `Option<String>` | | Alternate name / alias |
| `description` | `Option<String>` | | Description |
| `place_type` | `Option<PlaceType>` | | Classification |
| `address` | `Option<PostalAddress>` | | Structured address |
| `geo` | `Option<GeoCoordinates>` | | Coordinates |
| `telephone` | `Option<String>` | | Phone (international format) |
| `fax_number` | `Option<String>` | | Fax number |
| `url` | `Option<String>` | | Website URL |
| `global_location_number` | `Option<String>` | | 13-digit GLN |
| `branch_code` | `Option<String>` | | Branch code |
| `contained_in_place` | `Option<Uuid>` | | Parent place ID |
| `keywords` | `Vec<String>` | optional on wire | Tags |
| `identifiers` | `Vec<PlaceIdentifier>` | optional on wire | External identifiers |
| `amenity_features` | `Vec<AmenityFeature>` | optional on wire | Features |
| `opening_hours` | `Vec<OpeningHoursSpecification>` | optional on wire | Hours |
| `is_accessible_for_free` | `Option<bool>` | | Free access |
| `public_access` | `Option<bool>` | | Open to public |
| `smoking_allowed` | `Option<bool>` | | Smoking permitted |
| `maximum_attendee_capacity` | `Option<u32>` | | Max capacity |
| `is_deleted` | `bool` | server-managed | Soft delete flag (default `false`) |
| `deleted_at` | `Option<DateTime<Utc>>` | | Deletion timestamp |
| `created_at` | `DateTime<Utc>` | server-managed | Creation timestamp (stamped on insert) |
| `updated_at` | `DateTime<Utc>` | server-managed | Last update timestamp (stamped on insert/update) |

Methods: `Place::new(name)`, `place.soft_delete()`

## PostalAddress

`src/models/address.rs`

| Field | Type |
|-------|------|
| `street_address` | `Option<String>` |
| `address_locality` | `Option<String>` |
| `address_region` | `Option<String>` |
| `address_country` | `Option<String>` |
| `postal_code` | `Option<String>` |

## GeoCoordinates

`src/models/geo.rs`

| Field | Type | Range |
|-------|------|-------|
| `latitude` | `f64` | -90.0 to 90.0 |
| `longitude` | `f64` | -180.0 to 180.0 |
| `elevation` | `Option<f64>` | Meters |

Methods: `GeoCoordinates::new(lat, lon)`, `geo.distance_to(&other)` (Haversine, returns meters)

## PlaceType

`src/models/place_type.rs`

Enum variants: `LocalBusiness`, `CivicStructure`, `AdministrativeArea`, `Landform`, `Park`, `Airport`, `Hospital`, `School`, `Library`, `Museum`, `Restaurant`, `Hotel`, `Other(String)`

## PlaceIdentifier

`src/models/identifier.rs`

| Field | Type |
|-------|------|
| `identifier_type` | `IdentifierType` |
| `value` | `String` |

IdentifierType variants: `GlobalLocationNumber`, `BranchCode`, `Fips`, `Gnis`, `OpenStreetMap`, `Custom(String)`

Methods: `PlaceIdentifier::new(type, value)`, `PlaceIdentifier::gln(value)`

## AmenityFeature

`src/models/amenity.rs`

| Field | Type |
|-------|------|
| `name` | `String` |
| `value` | `Option<String>` |

## OpeningHoursSpecification

`src/models/opening_hours.rs`

| Field | Type |
|-------|------|
| `day_of_week` | `DayOfWeek` |
| `opens` | `String` (HH:MM) |
| `closes` | `String` (HH:MM) |

## Consent

`src/models/consent.rs`

| Field | Type |
|-------|------|
| `id` | `Uuid` |
| `place_id` | `Uuid` |
| `consent_type` | `ConsentType` |
| `status` | `ConsentStatus` |
| `granted_at` | `DateTime<Utc>` |
| `expires_at` | `Option<DateTime<Utc>>` |

ConsentType: `DataProcessing`, `DataSharing`, `Marketing`, `Research`
ConsentStatus: `Active`, `Revoked`, `Expired`

Methods: `consent.is_active()` (checks status and expiration)
