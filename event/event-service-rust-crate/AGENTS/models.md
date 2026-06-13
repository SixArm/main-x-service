# Domain Model Reference

The central model is [`Event`], aligned with
[schema.org/Event](https://schema.org/Event). An event is a
time-bounded occurrence (conference, appointment, shift, encounter,
sale, screening, …) with a location, parties, optional offers, and
links to other events.

## Event

**File:** `src/models/event.rs`

### schema.org/Thing properties

| Field | Type | schema.org | Purpose |
|---|---|---|---|
| `id` | `Uuid` | — | Internal identifier |
| `identifiers` | `Vec<Identifier>` | (mapped from `identifier`) | External system IDs |
| `active` | `bool` | — | Soft-delete / hidden flag |
| `name` | `String` | `name` | Human-readable title |
| `alternate_names` | `Vec<String>` | `alternateName` | Aliases |
| `description` | `Option<String>` | `description` | Long-form description |
| `disambiguating_description` | `Option<String>` | `disambiguatingDescription` | Short distinguishing description |
| `url` | `Option<String>` | `url` | Canonical URL |
| `image` | `Vec<String>` | `image` | Image URLs |
| `same_as` | `Vec<String>` | `sameAs` | Cross-system identity URLs |
| `keywords` | `Vec<String>` | `keywords` | Tags |

### Event time window

| Field | Type | schema.org | Purpose |
|---|---|---|---|
| `start_date` | `DateTime<Utc>` | `startDate` | When the event starts (required) |
| `end_date` | `Option<DateTime<Utc>>` | `endDate` | When it ends; open-ended if absent |
| `door_time` | `Option<DateTime<Utc>>` | `doorTime` | Admission opens |
| `duration` | `Option<String>` | `duration` | ISO 8601 duration (e.g. `PT1H30M`) |
| `previous_start_date` | `Option<DateTime<Utc>>` | `previousStartDate` | Original start before reschedule |
| `time_zone` | `Option<String>` | — | IANA tz for display (storage is UTC) |
| `all_day` | `bool` | — | All-day flag |

### Status / mode / type

| Field | Type | schema.org | Variants |
|---|---|---|---|
| `event_status` | `EventStatus` | `eventStatus` | `Scheduled`, `Cancelled`, `MovedOnline`, `Postponed`, `Rescheduled`, `Completed` |
| `event_attendance_mode` | `EventAttendanceMode` | `eventAttendanceMode` | `Offline`, `Online`, `Mixed` |
| `event_type` | `EventType` | (local subtype) | `Generic`, `Appointment`, `Business`, `Childrens`, `Comedy`, `Conference`, `Course`, `Dance`, `Delivery`, `Education`, `Encounter`, `Exhibition`, `Festival`, `Food`, `Hackathon`, `Incident`, `Literary`, `Music`, `PerformingArts`, `Publication`, `Sale`, `Screening`, `Series`, `Session`, `Shift`, `Social`, `Sports`, `Theater`, `VisualArts` |

### Audience & accessibility

| Field | Type | schema.org |
|---|---|---|
| `typical_age_range` | `Option<String>` | `typicalAgeRange` |
| `in_language` | `Vec<String>` (ISO 639-1) | `inLanguage` |
| `is_accessible_for_free` | `Option<bool>` | `isAccessibleForFree` |
| `maximum_attendee_capacity` | `Option<u32>` | `maximumAttendeeCapacity` |
| `maximum_physical_attendee_capacity` | `Option<u32>` | `maximumPhysicalAttendeeCapacity` |
| `maximum_virtual_attendee_capacity` | `Option<u32>` | `maximumVirtualAttendeeCapacity` |
| `remaining_attendee_capacity` | `Option<u32>` | `remainingAttendeeCapacity` |

### Location, parties, works, hierarchy, offers

| Field | Type | schema.org |
|---|---|---|
| `location` | `Vec<Location>` | `location` |
| `organizers` | `Vec<Party>` | `organizer` |
| `performers` | `Vec<Party>` | `performer` |
| `attendees` | `Vec<Party>` | `attendee` |
| `sponsors` | `Vec<Party>` | `sponsor` |
| `funders` | `Vec<Party>` | `funder` |
| `contributors` | `Vec<Party>` | `contributor` |
| `about` | `Vec<Reference>` | `about` |
| `works` | `Vec<Reference>` | `workFeatured` / `workPerformed` |
| `super_event` | `Option<Uuid>` | `superEvent` |
| `sub_events` | `Vec<Uuid>` | `subEvent` |
| `offers` | `Vec<Offer>` | `offers` |
| `links` | `Vec<EventLink>` | (merge / refer / see-also) |

### Constructors

- `Event::new(name, start_date) -> Self`
- `Event::identifier_value(kind) -> Option<&str>`

## Supporting types

### Location

`enum Location { Place(Place), PostalAddress(Address), Virtual(VirtualLocation), Text { value: String } }`

Mirrors `schema.org/Event.location` which is a union of `Place`,
`PostalAddress`, `VirtualLocation`, and `Text`.

### Place

| Field | Type |
|---|---|
| `id` | `Option<Uuid>` (external place-service ref) |
| `name` | `String` |
| `address` | `Option<Address>` |
| `latitude` | `Option<f64>` |
| `longitude` | `Option<f64>` |
| `url` | `Option<String>` |

### VirtualLocation

`name: Option<String>`, `url: String`.

### Address

`use_type`, `line1`, `line2`, `city`, `state`, `postal_code`, `country`.

### Party

| Field | Type |
|---|---|
| `kind` | `PartyKind::{Person, Organization}` |
| `id` | `Option<Uuid>` (external person/org-service ref) |
| `name` | `String` |
| `email` | `Option<String>` |
| `url` | `Option<String>` |

### Reference

Typed pointer to another resource (used for `about`, `works`):
`id`, `name`, `url`, `kind`.

### Offer

`name`, `price` (decimal as string), `price_currency` (ISO 4217),
`url`, `availability`
(`InStock`/`SoldOut`/`PreOrder`/`OutOfStock`/`Discontinued`),
`valid_from`, `valid_through`.

### Identifier

`use_type`, `identifier_type`, `system`, `value`, `assigner`.

`IdentifierType` variants:

| Variant | Purpose |
|---|---|
| `BookingNumber` | Reservation number (hotels, restaurants, …) |
| `ConfirmationCode` | Alphanumeric confirmation (tickets, registrations) |
| `TicketNumber` | Specific ticket within a sale |
| `EncounterId` | Clinical Encounter resource ID |
| `TransactionId` | Sale / payment / order reference |
| `ExternalRef` | Opaque external system reference |
| `Tax` | Tax / invoice reference for billable events |
| `Other` | Catch-all |

### EventLink

`other_event_id: Uuid`, `link_type: LinkType` (`ReplacedBy`,
`Replaces`, `Refer`, `Seealso`).

### Organization, MergeRequest/Response/Record, ReviewQueueItem, Consent

Unchanged in shape from before; see the corresponding source files
under `src/models/`. They are used in the merge, batch-dedup,
review-queue, and consent workflows.

## Database mapping

SeaORM entity modules (`src/db/models.rs`) map to these tables:

- `events` — scalar fields + JSONB arrays (`alternate_names`, `image`, `same_as`, `keywords`, `in_language`)
- `event_identifiers`, `event_locations`, `event_parties`, `event_offers`, `event_links`, `event_sub_events`
- `organizations` + `organization_addresses` / `organization_contacts` / `organization_identifiers`
- `audit_log`

Migrations live in `migrations/` as numbered SQL `up.sql` / `down.sql` pairs.

## Invariants

- `start_date` is required.
- `end_date >= start_date` when present (checked in DB and at the validation layer).
- `door_time <= start_date` when present.
- `maximum_physical_attendee_capacity + maximum_virtual_attendee_capacity <= maximum_attendee_capacity` when all three are set.
- `remaining_attendee_capacity <= maximum_attendee_capacity` when both are set.
- For `EventAttendanceMode::Online`, at least one `Location::Virtual` is expected.
- For `EventAttendanceMode::Mixed`, at least one physical and one virtual location are expected.
- An `Identifier` is unique within `(event_id, identifier_type, system, value)`.

## Source files

- `src/models/event.rs` — `Event`, `EventLink`, `LinkType`
- `src/models/mod.rs` — `Address`, `ContactPoint`, `Location`, `Place`, `VirtualLocation`, `Party`, `PartyKind`, `Reference`, `Offer`, `OfferAvailability`, `EventStatus`, `EventAttendanceMode`, `EventType`
- `src/models/identifier.rs` — `Identifier`, `IdentifierType`, `IdentifierUse`
- `src/models/organization.rs` — `Organization`
- `src/models/merge.rs` — `MergeRequest`, `MergeResponse`, `MergeRecord`, `MergeStatus`
- `src/models/review_queue.rs` — `ReviewQueueItem`, `ReviewStatus`, `BatchDeduplicationRequest`, `BatchDeduplicationResponse`
- `src/models/consent.rs` — `Consent`, `ConsentType`, `ConsentStatus`
