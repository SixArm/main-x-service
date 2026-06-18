## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete event records.
- Multiple identifiers per event (typed, system-qualified).
- Status transitions tracked via the audit log.
- Automatic event publish on every CRUD.

### 6.2 Matching

Algorithm reference: [`AGENTS/matching.md`](../AGENTS/matching.md).

Default component weights (sum to 1.0):

| Component | Weight | Algorithm |
|---|---:|---|
| Name (title + alternates) | 0.20 | Jaro-Winkler + Levenshtein + Soundex floor |
| Start date | 0.20 | Exponential decay (1 h half-life) |
| End date | 0.10 | Exponential decay or window overlap |
| Location | 0.15 | Place-id exact / address fuzzy / virtual URL / text |
| Organizer | 0.10 | Party id exact / name fuzzy / email exact |
| Performer | 0.10 | Same |
| Attendee | 0.05 | Same |
| Identifier | 0.10 | Type + system + value match |

Deterministic short-circuit: exact value match on a **strong**
identifier (`BookingNumber`, `ConfirmationCode`, `TicketNumber`,
`EncounterId`, `TransactionId`) → 1.0.

Match quality:

| Quality | Score |
|---|---|
| Definite | ≥ 0.95 |
| Probable | ≥ 0.85 |
| Possible | ≥ 0.50 |
| Unlikely | < 0.50 |

#### Interoperability with `event-matcher`

The service embeds the sibling `event-matcher` crate (declared in
`Cargo.toml`) and re-exports it from `src/matching/mod.rs` as
`matcher_lib`. The matcher crate is the **canonical reference
algorithm** — it carries an `EventCategory` enum (24 schema.org/Event
subtypes plus `Other(s)`), `EventIdScheme` for ticketing-system IDs
(Eventbrite, Meetup, Ticketmaster, Songkick, Bandsintown, Facebook,
Luma, Wikidata, Google Calendar, iCalendar UID), per-field weight
renormalisation, and Haversine + Gaussian-decay geo scoring that the
in-service matcher does not duplicate.

Bridge: [`src/matching/adapter.rs`](../src/matching/adapter.rs) exposes
`to_matcher_event(&service::Event) -> event_matcher::Event`. The
projection lifts the service's schema.org/Event-shaped record into
the matcher's flat builder shape, including the typing conversions
the two crates disagree on:

- Time fields: `DateTime<Utc>` → RFC 3339 strings for `start_date`,
  `end_date`, `door_time`, `previous_start_date`
- `event_status` → matcher `EventStatus` (`Completed` collapses to
  `EventScheduled` — no matcher counterpart)
- `event_attendance_mode` → matcher `EventAttendanceMode`
  (`Offline` → `OfflineEventAttendanceMode`, …)
- `event_type` → matcher `EventCategory` via `map_event_type`; the
  service's operational subtypes (`Appointment`, `Encounter`,
  `Shift`, `Incident`, `Generic`, `Session`, `Course`) flow through
  as `Other(name)` so the scheme name still participates in
  `(scheme, value)` equality
- `Location`: matcher takes a single `Location` struct; the service
  carries `Vec<Location>` (`Place`, `PostalAddress`, `Virtual`,
  `Text`). The first populated entry is dispatched variant-aware:
  `Place` → venue name + address + lat/lon; `Virtual` →
  `virtual_url` + venue name; `PostalAddress` → address only;
  `Text` → venue name only
- `organizers: Vec<Party>` → first non-empty `Party.name` →
  matcher `organizer` (single string)
- `performers: Vec<Party>` → `Vec<String>` of names
- `identifiers[]` mapped via `map_identifier_scheme`: `system` URI
  hints win (matches `eventbrite`, `meetup`, `ticketmaster`,
  `songkick`, `bandsintown`, `facebook`/`fb.com`, `lu.ma`/`luma`,
  `google`/`calendar.google`, `wikidata`, `ical`/`vcal`); otherwise
  the `IdentifierType` enum (`BookingNumber`, `ConfirmationCode`,
  `TicketNumber`, `EncounterId`, `TransactionId`, `ExternalRef`,
  `Tax`, `Other`) flows through as `Other(name)`
- Capacity caps (`maximum_attendee_capacity` /
  `maximum_physical_attendee_capacity` /
  `maximum_virtual_attendee_capacity`),
  `is_accessible_for_free`, and `super_event` (UUID → string) pass
  through unchanged
- **Language tags — known divergence (ET-8).** The service validates
  `in_language` entries as 2-letter ISO 639-1 codes (§6.5); the
  matcher documents its single `in_language` field as a full IETF
  BCP 47 tag. The adapter projects only the **first** `in_language`
  entry (trimmed; skipped when empty); any further entries are
  dropped. The projected value is **data-only** on the matcher side —
  no scoring component consults it — so the divergence is currently
  inert, and every ISO 639-1 code is a syntactically valid BCP 47
  tag. Revisit (and pin with a bridge test) if the matcher ever
  scores `in_language` or the service widens validation to full
  BCP 47

Service-only fields (`id`, `active`, `duration`, `time_zone`,
`all_day`, `image`, `same_as`, `disambiguating_description`,
`attendees`, `sponsors`, `funders`, `contributors`, `about`,
`works`, `sub_events`, `offers`, `links`, audit timestamps) are
dropped — they have no matcher counterpart. See
[`AGENTS/matching.md`](../AGENTS/matching.md) for the in-service
algorithm and the matcher crate's
[`spec.md §5–§7`](../../event-matcher-rust-crate/spec/index.md) for the
canonical algorithm.

### 6.3 Search

Tantivy across `name`, `alternate_names`, `description`, `keywords`,
`organizer_name`, `performer_name`, `identifier_value`, plus faceted
fields for `event_status`, `event_attendance_mode`, `event_type`,
`in_language`, location city / country / URL, and `start_date`
(yyyy-mm-dd for range queries). Full-text + fuzzy + boolean.
Pagination (`offset` + `limit`). Optional masking for events with
sensitive identifiers or party emails.

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/v1/events` when the blocking
  step (name + start-date) yields a probable match above the
  threshold.
- Explicit `POST /api/v1/events/check-duplicates`.
- Batch `POST /api/v1/events/deduplicate`.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`).
- Merge picks the surviving record; transfers identifiers, alternate
  names, keywords, locations, parties, and `same_as` URLs; appends the
  duplicate's primary name as an `alternate_name` on the survivor;
  adds a `Replaces` link; soft-deletes the duplicate; records a JSON
  snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

`name` required; `start_date` required; `end_date ≥ start_date` when
both present; `door_time ≤ start_date`; `duration` parsed as ISO 8601;
`in_language` entries 2-letter ISO 639-1; `Online` attendance mode
expects a `Virtual` location; `Mixed` expects one physical + one
virtual; capacities non-negative and consistent; per-location checks
(place name, lat/lon range, URL scheme, address completeness);
per-offer checks (3-letter ISO 4217 currency, parseable price,
`valid_from ≤ valid_through`). Failed validation → `422`.

### 6.6 Privacy

Per-field masking of identifier values (often double as access tokens)
and party emails; external party IDs stripped from the masked view.
GDPR Article 15 export at `GET /api/v1/events/{id}/export`. Consent
records (`Consent` model) let callers grant / revoke processing /
sharing / marketing / research consent per event. See
[`agents/share/privacy.md`](../../../agents/share/privacy.md).

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp.

### 6.8 FHIR R5

**Stubbed.** `/fhir/Event/*` returns `501 Not Implemented` with an
`OperationOutcome` body until the schema.org/Event → FHIR R5 mapping
is fixed. See OQ-1.

