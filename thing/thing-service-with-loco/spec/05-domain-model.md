## 5. Domain Model

Field-by-field reference: [`agents/models.md`](../agents/models.md).

### 5.1 `Thing`

Material aspects:

- **Schema.org/Thing properties** — `name` (required),
  `alternate_names`, `description`, `disambiguating_description`,
  `additional_type` (URL of a schema.org sub-type),
  `url`, `images`, `main_entity_of_page`, `owner`, `same_as`,
  `subject_of`, `potential_action`.
- **Identifiers** — `Vec<ThingIdentifier>` (the `PropertyValue` shape).
- **Registry-internal** — UUID `id`, `created_at`, `updated_at`,
  `is_deleted`, `deleted_at`.

### 5.2 `ThingIdentifier`

`{ property_id: IdentifierType, value: String, name?: String, url?: String }`

`IdentifierType` variants:

- **Deterministic** (globally unique): `Doi`, `Isbn`, `Issn`, `Gtin`,
  `Mpn`, `SerialNumber`, `Uuid`.
- **Non-deterministic**: `Sku`, `Uri`, `Custom(String)`.

### 5.3 Supporting types

`MergeRequest` / `MergeResponse` / `MergeRecord`, `ReviewQueueItem`,
`BatchDeduplicationRequest` / `Response`, `Consent` (for Things
subject to data-protection regimes — e.g. a personally-owned record).

### 5.4 Invariants

The implementation MUST enforce:

- `name` is non-empty.
- An `Identifier` is keyed by `(property_id, value)`; duplicates within
  a single record are silently deduplicated.
- All URL-valued properties (`url`, `additional_type`,
  `main_entity_of_page`, `subject_of`, each `image`, each `same_as`)
  MUST use the `http://` or `https://` scheme.
- `Isbn` is 10 or 13 digits (dashes / spaces tolerated; trailing `X`
  allowed for ISBN-10).
- `Issn` is 8 chars (trailing `X` allowed).
- `Doi` MUST start with `10.` and contain `/`.
- `Gtin` is 8 / 12 / 13 / 14 digits.
- `Uuid` MUST parse per RFC 4122.
- `Uri` MUST contain a scheme separator (`:`).
- Soft delete is the only delete.

