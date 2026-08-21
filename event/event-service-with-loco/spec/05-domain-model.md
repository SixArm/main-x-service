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

### 5.3 Invariants

The implementation MUST enforce:

- `name` is non-empty.
- `start_date` is present.
- `end_date ≥ start_date` when both are present.
- `door_time ≤ start_date` when present.
- `maximum_physical + maximum_virtual ≤ maximum_total` when all three
  are set.
- `remaining ≤ maximum_total` when both are set.
- `EventAttendanceMode::Online` requires at least one
  `Location::Virtual`.
- `EventAttendanceMode::Mixed` requires at least one physical and one
  virtual location.
- An `Identifier` is unique within `(event_id, identifier_type, system, value)`.
- Cancelled events are not deleted — `event_status` changes;
  `active` remains `true` until an explicit soft-delete.
- Soft delete is the only delete.

