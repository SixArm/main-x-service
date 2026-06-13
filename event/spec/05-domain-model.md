## 5. Domain Model

Three model surfaces exist; this section pins how they relate.
Field-by-field references:
service [`AGENTS/models.md`](../event-service-rust-crate/AGENTS/models.md),
matcher [`README.md`](../event-matcher-rust-crate/README.md) (0.5.0
surface), front-end `src/lib/api/types.ts`.

### 5.1 Canonical `Event` (service)

The persisted record, aligned with schema.org/Event:

- **Identity** — UUID `id` + `identifiers: Vec<Identifier>` (typed,
  system-qualified; `BookingNumber`, `ConfirmationCode`,
  `TicketNumber`, `EncounterId`, `TransactionId`, `ExternalRef`,
  `Tax`, `Other`).
- **Thing properties** — `name` (required), `alternate_names`,
  `description`, `url`, `image`, `same_as`, `keywords`.
- **Time window** — `start_date` (required, UTC), `end_date`,
  `door_time`, `duration` (ISO 8601), `previous_start_date`,
  `time_zone` (IANA, display-only), `all_day`.
- **Status / mode / type** — `event_status`,
  `event_attendance_mode`, `event_type` (29 variants).
- **Location** — `Vec<Location>` where each entry is one of `Place`
  (with optional ID into the sibling place entity), `PostalAddress`,
  `Virtual` (URL), or `Text`.
- **Parties** — `organizers`, `performers`, `attendees`, `sponsors`,
  `funders`, `contributors`; each `Party` may carry an external ID
  into the sibling person / worker / organization entities.
- **Offers, hierarchy, links** — `Vec<Offer>`, `super_event` /
  `sub_events`, `EventLink` (`Replaces`, `ReplacedBy`, `Refer`,
  `Seealso`).

### 5.2 Matcher `Event`

A flat, builder-constructed comparison record: `name` +
`alternate_names`, ISO 8601 string dates, single `Location`,
`category: EventCategory` (24 schema.org subtypes + `Other(s)`),
`event_ids: Vec<EventId>` with `EventIdScheme`, single `organizer`
string, `performers: Vec<String>`, capacities, `url`. Pure data — no
persistence, no UUIDs (only `local_id`, which is never scored).

### 5.3 The service ↔ matcher DTO contract

The service embeds the matcher (declared in its `Cargo.toml`,
re-exported from `src/matching/mod.rs` as `matcher_lib`). The bridge
is
[`src/matching/adapter.rs`](../event-service-rust-crate/src/matching/adapter.rs):
`to_matcher_event(&service::Event) -> event_matcher::Event`. **This
adapter is the entity-level contract**; the entity spec wins over
either crate spec for what it must map. Material rules (full list in
service spec [§6.2](../event-service-rust-crate/spec/06-functional-requirements.md)):

| Service side | Matcher side |
|---|---|
| `DateTime<Utc>` time fields | RFC 3339 strings |
| `event_type` (29 variants) | `EventCategory` via `map_event_type`; operational subtypes flow through as `Other(name)` |
| `event_status::Completed` | Collapses to `EventScheduled` (no matcher counterpart) |
| `Vec<Location>` | First populated entry, variant-dispatched into the flat `Location` |
| `organizers: Vec<Party>` | First non-empty `Party.name` (single string) |
| `performers: Vec<Party>` | `Vec<String>` of names |
| `identifiers[]` | `EventId` via `map_identifier_scheme` — `system` URI hints win; else `IdentifierType` as `Other(name)` |
| Capacities, `is_accessible_for_free`, `super_event` | Pass through (UUID → string) |
| `in_language: Vec<String>` (ISO 639-1, validated) | First non-empty entry → `in_language` (documented as BCP 47; **data-only — never scored**, so the ISO 639-1 ⊂ BCP 47 divergence is inert; remaining entries dropped — ET-8) |

Service-only fields (`id`, `active`, `duration`, `time_zone`,
`all_day`, `attendees`, `sponsors`, `funders`, `contributors`,
`offers`, `links`, audit timestamps, …) are **dropped** — they have
no matcher counterpart. The contract is pinned by the bridge tests
([`tests/duplicate_detection.rs`](../event-service-rust-crate/tests/duplicate_detection.rs),
16 tests).

### 5.4 Front-end types

`src/lib/api/types.ts` mirrors the service's wire format 1:1. By repo
decision the front-end keeps its own copy; when a field changes in
the service model, the front-end types are fixed in the same PR — do
not let them drift (front-end [`AGENTS.md`](../event-front-end-with-svelte/AGENTS.md)).

### 5.5 Shared invariants

All three subprojects MUST agree on:

- `name` non-empty; `start_date` present.
- `end_date ≥ start_date`; `door_time ≤ start_date` (when present).
- Capacity coherence: physical + virtual ≤ total; remaining ≤ total.
- `Online` attendance ⇒ a `Virtual` location; `Mixed` ⇒ at least one
  physical + one virtual.
- `Identifier` unique within `(event_id, identifier_type, system, value)`.
- Cancelled ≠ deleted: `event_status` changes, `active` stays `true`.
- Soft delete is the only delete.
