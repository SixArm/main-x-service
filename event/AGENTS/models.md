# Domain models — Event Entity

Orientation only — the field-by-field tables live in the
subprojects. Three model surfaces exist:

## 1. Canonical persisted `Event` (service)

schema.org/Event-aligned: Thing properties (`name` required,
`alternate_names`, `description`, `url`, `image`, `same_as`,
`keywords`), the **time window** (`start_date` required UTC,
`end_date`, `door_time`, ISO 8601 `duration`,
`previous_start_date`, IANA `time_zone`, `all_day`), status / mode /
type enums (29 `EventType` variants), capacities, the **`Location`
union** (`Place` / `PostalAddress` / `Virtual` / `Text`, as
`Vec<Location>`), six **`Party`** lists (organizers, performers,
attendees, sponsors, funders, contributors), `Vec<Offer>`,
`super_event` / `sub_events` hierarchy, typed `Identifier`s, and
`EventLink`s.

→ Full reference:
[`event-service-rust-crate/AGENTS/models.md`](../event-service-rust-crate/AGENTS/models.md)
(includes invariants and the SeaORM table mapping).

## 2. Matcher `Event` (comparison DTO)

Flat, builder-constructed, `#[non_exhaustive]`: optional `name` +
alternates, ISO 8601 **string** dates, a **single** flat `Location`,
`EventCategory` (24 schema.org subtypes + `Other`),
`event_ids: Vec<EventId>` with `EventIdScheme` (Eventbrite, Meetup,
Ticketmaster, Wikidata, iCalendar UID, …), single `organizer`
string, `performers: Vec<String>`. `local_id` is never scored.

→ Live surface: [`event-matcher-rust-crate/README.md`](../event-matcher-rust-crate/README.md)
(the matcher's spec §3 still describes the 0.4.x place model — ET-1).

## 3. Front-end TypeScript types

`src/lib/api/types.ts` mirrors the service wire format 1:1; fix it in
the same PR as any service model change. Per-project copy, drift
between front-ends accepted.

→ Ground rules: [`event-front-end-with-svelte/AGENTS.md`](../event-front-end-with-svelte/AGENTS.md).

## The contract between 1 and 2

`to_matcher_event` in
[`src/matching/adapter.rs`](../event-service-rust-crate/src/matching/adapter.rs)
projects the persisted record into the comparison DTO: DateTimes →
RFC 3339 strings, first populated `Location` variant-dispatched,
first organizer name, performer names only, identifier `system` URI
hints → `EventIdScheme`. Service-only fields (attendees, offers,
links, audit columns, …) are dropped. The projection is normative in
the entity spec [§5.3](../spec/05-domain-model.md) and pinned by the
bridge tests
([`tests/duplicate_detection.rs`](../event-service-rust-crate/tests/duplicate_detection.rs)).

## Shared invariants

Listed in entity spec [§5.5](../spec/05-domain-model.md) — time-window
ordering, capacity coherence, attendance-mode ↔ location coherence,
identifier uniqueness, soft-delete-only. All three subprojects must
agree on these.
