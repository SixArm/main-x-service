# Event Service

A registry of **time-bounded events** — conferences, performances,
appointments, encounters, shifts, sessions, screenings, sales,
deliveries, incidents. The domain model is aligned with
[schema.org/Event](https://schema.org/Event).

@agents/share/overview.md

## Features

### Event identity

- CRUD with soft-delete and full audit trail.
- External identifiers (`BookingNumber`, `ConfirmationCode`,
  `TicketNumber`, `EncounterId`, `TransactionId`, `ExternalRef`,
  `Tax`, `Other`).
- `super_event` / `sub_events` hierarchy.
- Cross-event links (`Replaces`, `ReplacedBy`, `Refer`, `Seealso`).

### Event content

- schema.org/Thing properties: `name`, `description`,
  `alternate_names`, `url`, `image`, `same_as`, `keywords`.
- Time window: `start_date` (required), `end_date`, `door_time`,
  `duration` (ISO 8601), `previous_start_date`, `time_zone`, `all_day`.
- Status / classification: `event_status`,
  `event_attendance_mode`, `event_type` (29 variants from
  `Generic` through `VisualArts` plus operational subtypes like
  `Appointment`, `Encounter`, `Shift`, `Incident`).
- Capacity: `maximum_attendee_capacity` (total / physical /
  virtual / remaining).
- Audience: `typical_age_range`, `in_language`, `is_accessible_for_free`.

### Location

`Vec<Location>` where each `Location` is one of:

- `Place` — physical venue with name, address, geo coords;
  optional ID into a sibling place service.
- `PostalAddress` — inline address.
- `Virtual` — URL + optional name (livestream, video-conference, …).
- `Text` — free-text fallback (e.g. "TBA").

### Parties

`organizers`, `performers`, `attendees`, `sponsors`, `funders`,
`contributors` — each a `Vec<Party>`. A `Party` is a typed
reference (`Person` / `Organization`) with a name, optional
external ID into a sibling person/worker/organization service, and
optional email / URL.

### Offers

`Vec<Offer>` for ticket / pricing tiers: `price`, `price_currency`
(ISO 4217), `availability`, `valid_from`, `valid_through`.

### Matching

- Probabilistic (weighted fuzzy) and deterministic (rule-based) strategies.
- Components: name (Jaro-Winkler + Levenshtein + Soundex), start /
  end date proximity (exponential decay), window overlap, location,
  organizer, performer, attendee, identifier.
- Short-circuit on exact match of a strong identifier (booking /
  ticket / confirmation / encounter / transaction).

### Search

- Tantivy index on `name`, `alternate_names`, `description`,
  `keywords`, `organizer_name`, `performer_name`,
  `identifier_value`.
- Faceted by `event_status`, `event_attendance_mode`,
  `event_type`, `in_language`.
- Date-range filter on `start_date`.
- Full-text + fuzzy + name+date blocking for matching.

### Validation

- `name` non-empty.
- `end_date >= start_date`, `door_time <= start_date`.
- ISO 8601 `duration`; 2-letter ISO 639-1 `in_language` codes.
- Attendance mode coherent with location list (Online ⇒ a Virtual
  location; Mixed ⇒ at least one physical + one Virtual).
- Capacity invariants: physical + virtual ≤ total; remaining ≤ total.
- Per-location validation: place name required, lat/lon ranges,
  virtual URL scheme, address completeness.
- Per-offer validation: 3-letter currency, parseable price,
  valid_from ≤ valid_through.

### Privacy

- Identifier values masked (last 4 visible) — they often double
  as access tokens.
- Party emails masked.
- External party IDs stripped from masked view.
- GDPR right-of-access export.
- Consent management (`Consent` model with type / status / dates).

### Event streaming + audit

- `EventEvent::{Created, Updated, Deleted, Merged, Linked, Unlinked}`
  published on every CRUD / merge / link operation.
- HIPAA-style audit log in PostgreSQL (`audit_log` table, with old
  / new JSON snapshots).

@AGENTS/index.md
@AGENTS/models.md
@AGENTS/matching.md
@AGENTS/restful.md
@AGENTS/testing.md

@agents/share/auditability.md
@agents/share/availability.md
@agents/share/match-search-merge.md
@agents/share/observability.md
@agents/share/privacy.md
@agents/share/restful.md
@agents/share/technology.md

## Quick start

### Docker

```bash
git clone https://github.com/sixarm/event-service-rust-crate.git
cd event-service-rust-crate
cp .env.example .env
docker-compose up -d
curl http://localhost:8080/api/v1/health
```

- API: http://localhost:8080/api/v1
- Swagger UI: http://localhost:8080/swagger-ui

### Local

Prereqs: Rust 1.93+ (2024 edition), PostgreSQL 18+, `sea-orm-cli`.

```bash
createdb event_service
cp .env.example .env  # set DATABASE_URL
sea-orm-cli migrate up
cargo run --release
```

## API examples

Create an event:

```bash
curl -X POST http://localhost:8080/api/v1/events \
  -H 'content-type: application/json' \
  -d '{
    "name": "Annual Conference",
    "start_date": "2026-06-01T09:00:00Z",
    "end_date":   "2026-06-01T17:00:00Z",
    "event_status": "scheduled",
    "event_attendance_mode": "offline",
    "event_type": "conference",
    "location": [{
      "kind": "place",
      "name": "Greek Theatre",
      "address": { "city": "Berkeley", "state": "CA", "postal_code": "94720", "country": "US" }
    }],
    "organizers": [{ "kind": "organization", "name": "Cal Performances" }]
  }'
```

Search:

```bash
curl "http://localhost:8080/api/v1/events/search?q=Conference&date_from=2026-06-01&date_to=2026-06-30"
```

Match:

```bash
curl -X POST http://localhost:8080/api/v1/events/match \
  -H 'content-type: application/json' \
  -d '{ "name": "Conferance", "start_date": "2026-06-01T09:00:00Z", "threshold": 0.5 }'
```

## Configuration

| Variable | Default | Notes |
|---|---|---|
| `DATABASE_URL` | — | Postgres connection string (required) |
| `DATABASE_MAX_CONNECTIONS` | 10 | Pool max |
| `SERVER_HOST` | `0.0.0.0` | |
| `SERVER_PORT` | `8080` | |
| `SEARCH_INDEX_PATH` | `./data/search_index` | Tantivy index dir |
| `MATCHING_THRESHOLD` | `0.85` | Probabilistic match cutoff |
| `RUST_LOG` | `info` | tracing-subscriber filter |

## Testing

```bash
cargo test --lib                              # unit tests
DATABASE_URL=… cargo test --test api_integration_test  # integration
cargo bench                                   # criterion benches
```

See [`AGENTS/testing.md`](AGENTS/testing.md) for more.

## Compliance

- **HIPAA** (when records include protected health information): audit log, soft delete, access
  control, encryption at rest.
- **GDPR**: right of access (`/events/{id}/export`), right to
  erasure (soft delete), consent management.

## License

Dual-licensed under MIT OR Apache-2.0.

**Status**: Schema.org/Event rebuild complete.
**Version**: 0.2.0
