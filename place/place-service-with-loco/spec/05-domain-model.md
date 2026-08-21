## 5. Domain Model

Field-by-field reference: [`agents/models.md`](../agents/models.md).

### 5.1 `Place`

Material aspects:

- **Identity** — UUID `id` + `identifiers: Vec<PlaceIdentifier>` (GLN
  13-digit, FIPS, GNIS, OSM ID, custom). `global_location_number` and
  `branch_code` are shortcuts.
- **Names** — `name` (primary), `alternate_name`, `description`.
- **Classification** — `place_type` (`LocalBusiness`,
  `CivicStructure`, `AdministrativeArea`, `Landform`, `Residence`,
  `TouristAttraction`, `EducationalOrganization`, `Park`,
  `BodyOfWater`, `Hospital`, `School`, `Library`, `Museum`,
  `Restaurant`, `Hotel`, `Airport`, `Other(String)`), `keywords` tags.
- **Address** — `PostalAddress` (`street_address`,
  `address_locality`, `address_region`, `address_country`,
  `postal_code`).
- **Geo** — `GeoCoordinates` (`latitude`, `longitude`, `elevation`).
- **Hierarchy** — `contained_in_place` (parent UUID) + transitive
  `contains_place` (children).
- **Contact** — `telephone`, `fax_number`, `url`.
- **Operational** — `opening_hours_specification`,
  `is_accessible_for_free`, `public_access`, `smoking_allowed`,
  `maximum_attendee_capacity`, `amenity_feature`.
- **External cross-refs** — `same_as` (URLs to authoritative sources,
  used in dedup).
- **Audit** — `is_deleted`, `deleted_at`, `created_at`, `updated_at`.

### 5.2 Supporting types

`PostalAddress`, `GeoCoordinates`, `PlaceType`, `PlaceIdentifier`
(GLN / FIPS / GNIS / OSM / Custom), `AmenityFeature`,
`OpeningHoursSpecification`, `Organization`, `MergeRequest` /
`MergeResponse` / `MergeRecord`, `ReviewQueueItem`,
`BatchDeduplicationRequest` / `Response`, `Consent`.

### 5.3 Invariants

The implementation MUST enforce:

- `name` is non-empty.
- `geo.latitude ∈ [-90, 90]`, `geo.longitude ∈ [-180, 180]` when
  present.
- `GLN` MUST be exactly 13 digits and pass the GLN check-digit
  algorithm.
- `url` MUST use the `http://` or `https://` scheme.
- `telephone` MUST be in international `+` format.
- `address`, when present, MUST have at least one of
  `address_locality`, `postal_code`, or `address_country`.
- A place MUST be in at most one `contained_in_place`; cycles are
  rejected.
- Soft delete is the only delete.

