# Domain model reference — Thing Service

Based on [schema.org/Thing](https://schema.org/Thing). The Thing model is
the most general entity in the Main X Index family: a stable identity
for any arbitrary discrete object — a book, a paper, a digital asset, a
device, a product, etc. — anything that doesn't fit one of the more
opinionated sibling crates (`person`, `worker`, `event`).

## Thing

`src/models/thing.rs`

Core entity, modelled one-to-one on the canonical properties of
schema.org/Thing.

### Schema.org/Thing properties

| Field | Type | Schema.org property | Notes |
|-------|------|---------------------|-------|
| `name` | `String` | [`name`](https://schema.org/name) | Primary name (required) |
| `alternate_names` | `Vec<String>` | [`alternateName`](https://schema.org/alternateName) | Aliases, prior names, translations |
| `description` | `Option<String>` | [`description`](https://schema.org/description) | Free-text description |
| `disambiguating_description` | `Option<String>` | [`disambiguatingDescription`](https://schema.org/disambiguatingDescription) | Short contextual distinguishing detail |
| `additional_type` | `Option<String>` | [`additionalType`](https://schema.org/additionalType) | URL to a more specific subtype (e.g. `https://schema.org/Book`) |
| `url` | `Option<String>` | [`url`](https://schema.org/url) | Canonical URL of the item |
| `identifiers` | `Vec<ThingIdentifier>` | [`identifier`](https://schema.org/identifier) | Typed identifiers (ISBN, DOI, GTIN, …) |
| `images` | `Vec<String>` | [`image`](https://schema.org/image) | Image URLs |
| `main_entity_of_page` | `Option<String>` | [`mainEntityOfPage`](https://schema.org/mainEntityOfPage) | URL of the page for which this is the main entity |
| `owner` | `Option<String>` | [`owner`](https://schema.org/owner) | Person or organisation owning the item |
| `same_as` | `Vec<String>` | [`sameAs`](https://schema.org/sameAs) | Authoritative external URLs (Wikipedia, Wikidata, …) |
| `subject_of` | `Option<String>` | [`subjectOf`](https://schema.org/subjectOf) | URL of a CreativeWork/Event about this item |
| `potential_action` | `Option<String>` | [`potentialAction`](https://schema.org/potentialAction) | Idealised action (URL or descriptor) |

### Registry-internal fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | `Uuid` | Stable system identifier (auto-generated v4) |
| `is_deleted` | `bool` | Soft delete flag |
| `deleted_at` | `Option<DateTime<Utc>>` | Soft-delete timestamp |
| `created_at` | `DateTime<Utc>` | Creation timestamp |
| `updated_at` | `DateTime<Utc>` | Last update timestamp |

Methods: `Thing::new(name)`, `thing.soft_delete()`.

**Server-owned fields are optional on the wire (QA-SERVER-FIELDS,
2026-08-04).** `id`, `name`, `alternate_names`, `identifiers`,
`images`, `same_as`, `is_deleted`, `created_at`, and `updated_at` all
carry `#[serde(default)]`, so a hand-written `POST /api/things` body
may omit any of them instead of being refused by the JSON extractor
with `422 missing field …` before the handler ever runs. The create
handler mints a fresh `id` when the wire value is nil and the
repository stamps `created_at`/`updated_at` to "now" on insert
(preserving `created_at`, refreshing `updated_at` on update); an
omitted `name` now reaches `validate_thing` (`422 validation_error`)
rather than the extractor. `description`, `disambiguating_description`,
`additional_type`, `url`, `main_entity_of_page`, `owner`, `subject_of`,
`potential_action`, and `deleted_at` are plain `Option<T>` with no
explicit `#[serde(default)]` — Serde's derive already treats a missing
`Option<T>` field as `None`. See spec
[§9](../spec/09-api-surface.md#9-api-surface) for the full contract
and [`CHANGELOG.md`](../CHANGELOG.md) for the defect this fixed.

## ThingIdentifier

`src/models/identifier.rs`

Schema.org [`PropertyValue`](https://schema.org/PropertyValue) shape.

| Field | Type | Schema.org property |
|-------|------|---------------------|
| `property_id` | `IdentifierType` | [`propertyID`](https://schema.org/propertyID) |
| `value` | `String` | [`value`](https://schema.org/value) |
| `name` | `Option<String>` | [`name`](https://schema.org/name) |
| `url` | `Option<String>` | [`url`](https://schema.org/url) |

### IdentifierType variants

| Variant | Use |
|---------|-----|
| `Doi` | Digital Object Identifier (10.*/*) |
| `Isbn` | International Standard Book Number (10- or 13-digit) |
| `Issn` | International Standard Serial Number (8 chars) |
| `Gtin` | Global Trade Item Number (8/12/13/14 digit; UPC/EAN/JAN) |
| `Sku` | Stock-Keeping Unit (vendor-scoped) |
| `Mpn` | Manufacturer Part Number |
| `SerialNumber` | Manufacturer serial number |
| `Uri` | Generic URI |
| `Uuid` | UUID |
| `Custom(String)` | Any other scheme (free-text label) |

### Constructors

```rust
ThingIdentifier::isbn("9780141439518")
ThingIdentifier::doi("10.1038/nature12373")
ThingIdentifier::issn("0317-8471")
ThingIdentifier::gtin("0012345600012")
ThingIdentifier::sku("WIDGET-42")
ThingIdentifier::mpn("MPN-ABC-001")
ThingIdentifier::serial_number("SN-001")
ThingIdentifier::uri("urn:example:resource:42")
ThingIdentifier::uuid("550e8400-e29b-41d4-a716-446655440000")
ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1394865W")
```

### Deterministic identifiers

`id.is_deterministic()` returns `true` when the identifier is globally
unique by construction — DOI, ISBN, ISSN, GTIN, MPN, SerialNumber, UUID.
A deterministic-identifier match short-circuits matching to score 1.0.
`Sku`, `Uri`, and `Custom` are *not* considered deterministic because
they are not globally unique.

## Consent

`src/models/consent.rs`

Generic consent record for things subject to data-protection regimes
(e.g. a personal device or a personally-identifying record).

| Field | Type |
|-------|------|
| `id` | `Uuid` |
| `thing_id` | `Uuid` |
| `consent_type` | `ConsentType` |
| `status` | `ConsentStatus` |
| `granted_at` | `DateTime<Utc>` |
| `expires_at` | `Option<DateTime<Utc>>` |

`ConsentType`: `DataProcessing`, `DataSharing`, `Marketing`, `Research`
`ConsentStatus`: `Active`, `Revoked`, `Expired`

Methods: `consent.is_active()` (checks status and expiration).
