## 5. Domain Model

Field-by-field reference: [`agents/models.md`](../agents/models.md).

### 5.1 `Event`

Material aspects:

- **Identity** — UUID `id` + `identifiers: Vec<Identifier>` with the
  typed enum above; multiple per event.
- **Thing properties** — `name` (required), `alternate_names`,
  `description`, `disambiguating_description`, `url`, `image`,
  `same_as`, `keywords`.
- **Time window** — `start_date` (required, UTC), `end_date`,
  `door_time`, `duration` (ISO 8601), `previous_start_date`,
  `time_zone` (IANA, display-only), `all_day`.
- **Status / mode / type** — `event_status`, `event_attendance_mode`,
  `event_type` (29 variants).
- **Audience & accessibility** — `typical_age_range`, `in_language`
  (ISO 639-1), `is_accessible_for_free`, capacity totals.
- **Location** — `Vec<Location>` (Place / PostalAddress /
  VirtualLocation / Text).
- **Parties** — `organizers`, `performers`, `attendees`, `sponsors`,
  `funders`, `contributors`.
- **Hierarchy** — `super_event`, `sub_events`.
- **Works** — `about`, `works`.
- **Offers** — `Vec<Offer>` with price + currency + availability.
- **Audit** — `active`, `created_at`, `updated_at`.

### 5.2 Supporting types

`Location`, `Place`, `VirtualLocation`, `Address`, `Party`,
`Reference`, `Offer`, `EventLink`, `Organization`, `MergeRequest` /
`MergeResponse` / `MergeRecord`, `ReviewQueueItem`,
`BatchDeduplicationRequest` / `Response`, `Consent`.

#### 5.2.1 Geo coordinates are exact decimals

`Place::latitude` / `Place::longitude` are **`BigDecimal`**, stored in
`NUMERIC` columns — not `f64` / `DOUBLE PRECISION`.

A coordinate is a decimal quantity. A binary float cannot hold `37.87`;
it holds `37.869999999999997`, and cannot distinguish `37.87` from
`37.8700000000000001` at all. A coordinate therefore round-trips as the
digits the caller sent.

Two consequences the implementation MUST preserve:

- **The wire format is a JSON number, not a string.** `BigDecimal`'s
  default serde representation is a quoted string; these fields opt into
  `bigdecimal::impl_serde::arbitrary_precision_option` so the JSON is
  unchanged from when they were `f64` (`"latitude":37.87`, and `null`
  when absent). Existing clients — including the SvelteKit front-end,
  which types these as `number | null` — keep working untouched.
- **Matching converts at the boundary.** The matcher scores geo distance
  with Haversine, which is floating-point; the adapter converts to `f64`
  there. A coordinate that cannot be represented as `f64` is dropped from
  scoring rather than approximated into a wrong position. Exactness is a
  property of what is stored and returned, not of the distance score.

This is also what makes the type sound under `serde_json`'s
`arbitrary_precision` feature (`spec/serde-json-float-roundtrip-arbitrary-precision`
at the repository root). `Location` is an internally-tagged enum
(`#[serde(tag = "kind")]`), so serde buffers the variant's fields through
`Content`; under that feature an `f64` field read from the buffer fails
with `invalid type: map, expected f64`. An exact decimal does not.

### 5.3 Invariants

The implementation MUST enforce:

- `name` is non-empty.
- `start_date` is present.
- `end_date ≥ start_date` when both are present.
- `door_time ≤ start_date` when present.
- `maximum_physical + maximum_virtual ≤ maximum_total` when all three
  are set.
- `remaining ≤ maximum_total` when both are set.
- `Place::latitude` is within `-90..=90` and `Place::longitude` within
  `-180..=180`, inclusive, when present.
- A coordinate carries at most `MAX_COORDINATE_SCALE` (10) decimal
  places. An `f64` bounded the digit count implicitly; an exact decimal
  does not, so the cap is enforced rather than left to the database to
  truncate silently. Ten places is ~10 µm — past any real positioning
  system, and inside what an `f64` carried, so nothing a client could
  previously send is newly rejected.
- `EventAttendanceMode::Online` requires at least one
  `Location::Virtual`.
- `EventAttendanceMode::Mixed` requires at least one physical and one
  virtual location.
- An `Identifier` is unique within `(event_id, identifier_type, system, value)`.
- Cancelled events are not deleted — `event_status` changes;
  `active` remains `true` until an explicit soft-delete.
- Soft delete is the only delete.

