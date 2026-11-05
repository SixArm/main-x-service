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
- **Geo** — `GeoCoordinates` (`latitude_as_decimal_degrees`,
  `longitude_as_decimal_degrees`, `elevation_as_decimal_metres`),
  exact decimals — see §5.2.1.
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

#### 5.2.1 Geo coordinates are exact decimals

`GeoCoordinates::latitude_as_decimal_degrees` /
`longitude_as_decimal_degrees` / `elevation_as_decimal_metres` are
**`BigDecimal`**, stored in `NUMERIC` columns — not `f64` /
`DOUBLE PRECISION`.

A coordinate is a decimal quantity. A binary float cannot hold `40.7829`;
it holds `40.78289999999999793…`, and cannot distinguish `40.7829` from
`40.78290000000000001` at all. A coordinate therefore round-trips as the
digits the caller sent.

Unlike the same change in event-service, this one fixes **no
deserialization break**. Event's `Location` is an internally-tagged enum,
so its `f64` coordinates failed under `serde_json`'s `arbitrary_precision`
feature; place-service has no tagged enum or flattened struct in its
request path and was unaffected. This is the same correctness argument
applied for its own sake, and so the two services that model geography
agree.

Four consequences the implementation MUST preserve:

- **The wire format is a JSON number, not a string.** `BigDecimal`'s
  default serde representation is a quoted string; these fields opt into
  `bigdecimal::impl_serde::arbitrary_precision` (and
  `arbitrary_precision_option` for `elevation_as_decimal_metres`) so the
  JSON is unchanged from when they were `f64`. This holds for the FHIR
  `Location.position`
  surface too, where FHIR's own `decimal` type is arbitrary-precision by
  spec.
- **Matching converts at the boundary.** Haversine is trigonometry, so
  `distance_to` and the matcher adapter convert to `f64` there. A
  coordinate not representable as `f64` yields `NaN`, and every
  comparison against `NaN` is false, so a proximity check fails closed
  rather than returning a plausible wrong distance. Exactness is a
  property of what is stored and returned, not of the distance score.
- **`GeoCoordinates::new` takes `f64` and stores the decimal the literal
  denotes**, via the shortest round-tripping string.
  `BigDecimal::from_f64` is deliberately *not* used: it expands the
  binary approximation to forty-six digits of representation noise,
  which is worse than the `f64` it replaces and exceeds the scale cap.
- **A non-finite coordinate is unrepresentable.** A decimal has no `NaN`,
  which closes a hole the `f64` version had: `NaN` compares false against
  both range bounds, so a `NaN` latitude passed validation unnoticed.
  `new` panics rather than inventing a value; untrusted input reaches
  this type through deserialization or the database, never through `new`.

### 5.3 Invariants

The implementation MUST enforce:

- `name` is non-empty.
- `geo.latitude_as_decimal_degrees ∈ [-90, 90]`,
  `geo.longitude_as_decimal_degrees ∈ [-180, 180]` when present,
  inclusive.
- A geo coordinate carries at most `MAX_COORDINATE_SCALE` (10) decimal
  places — `latitude_as_decimal_degrees`, `longitude_as_decimal_degrees`,
  and `elevation_as_decimal_metres` alike. An `f64`
  bounded the digit count implicitly; an exact decimal does not, so the
  cap is enforced rather than left to the database to truncate silently.
  Ten places is ~10 µm — past any real positioning system, and inside
  what an `f64` carried, so nothing a client could previously send is
  newly rejected.
- `GLN` MUST be exactly 13 digits and pass the GLN check-digit
  algorithm.
- `url` MUST use the `http://` or `https://` scheme.
- `telephone` MUST be in international `+` format.
- `address`, when present, MUST have at least one of
  `address_locality`, `postal_code`, or `address_country`.
- A place MUST be in at most one `contained_in_place`; cycles are
  rejected.
- Soft delete is the only delete.

