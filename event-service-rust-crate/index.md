# Event Service — Index

Centralised registry of time-bounded events: appointments,
encounters, shifts, sessions, deliveries, incidents, conferences,
performances, sales. Domain model aligned with
[schema.org/Event](https://schema.org/Event). Probabilistic +
deterministic matching, real-time and batch deduplication, GDPR
Article 15 export, audit trail, and a server-rendered web UI.

This page is a **navigation aid with worked examples**. For canonical
behaviour, read [`spec.md`](spec.md).

## Documentation map

| File | Role |
|------|------|
| [`spec.md`](spec.md) | **Single source of truth.** What the system does, how it is built, NFRs, tasks (§13), open questions (§16). |
| [`README.md`](README.md) / [`CLAUDE.md`](CLAUDE.md) | User-facing intro — must stay consistent with the spec. |
| [`AGENTS.md`](AGENTS.md) | Agent-facing entry point — `AGENTS/*` directory + shared docs. |
| [`AGENTS/spec-driven-development.md`](AGENTS/spec-driven-development.md) | The SDD discipline this crate practises. |
| [`AGENTS/models.md`](AGENTS/models.md) | Field-by-field domain model reference. |
| [`AGENTS/matching.md`](AGENTS/matching.md) | Match weights, components, deterministic rules, Soundex. |
| [`AGENTS/restful.md`](AGENTS/restful.md) | Endpoint catalogue + library API. |
| [`AGENTS/testing.md`](AGENTS/testing.md) | Unit / integration / benchmark layout. |
| [`agents/share/*`](../agents/share/) | Project-wide cross-crate references (architecture, web stack, compliance, …). |

## Quick start

```bash
# REST + gRPC API
cargo run --release

# Web UI (Loco / Tera / HTMX / Alpine / Lily)
cargo run --bin web                    # → http://0.0.0.0:5150
PORT=5180 cargo run --bin web

# Tests
cargo test --lib                       # unit (~60+)
DATABASE_URL=… cargo test --tests      # integration (needs PostgreSQL)
cargo bench                            # Criterion (matching / search / validation)
```

## URL surface (REST)

All REST endpoints are mounted under `/api/v1`.

| Method | Path | Notes |
|---|---|---|
| GET | `/health` | Liveness |
| POST | `/events` | Create — `409` on detected duplicate, `422` on validation error |
| GET | `/events/{id}` | Read |
| PUT | `/events/{id}` | Replace |
| DELETE | `/events/{id}` | Soft delete |
| GET | `/events/search` | Full-text / fuzzy + date-range + facet filters |
| POST | `/events/match` | Score against candidates |
| POST | `/events/check-duplicates` | Real-time dup check |
| POST | `/events/merge` | Merge survivor + duplicate |
| POST | `/events/deduplicate` | Batch dedup scan |
| GET | `/events/{id}/masked` | Privacy view |
| GET | `/events/{id}/export` | GDPR Art. 15 export |
| GET | `/events/{id}/audit` | Per-record audit |
| GET | `/audit/recent` | System-wide recent audit |
| GET | `/audit/user` | Per-user audit |

FHIR R5 (`/fhir/Event/*`) returns `501 Not Implemented` today — see
[`spec.md §6.8`](spec.md#68-fhir-r5) for status.

## Worked examples

### Create an event

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
      "address": {
        "city": "Berkeley",
        "state": "CA",
        "postal_code": "94720",
        "country": "US"
      }
    }],
    "organizers": [{ "kind": "organization", "name": "Cal Performances" }]
  }'
```

If a duplicate is detected the response is `409 Conflict` with the
candidate matches and per-component breakdown.

### Check for duplicates

```bash
curl -X POST http://localhost:8080/api/v1/events/check-duplicates \
  -H 'content-type: application/json' \
  -d '{
    "name": "Annual Conference",
    "start_date": "2026-06-01T09:00:00Z"
  }'
```

### Search

```bash
curl "http://localhost:8080/api/v1/events/search?\
q=Conference&date_from=2026-06-01&date_to=2026-06-30&\
event_status=scheduled&limit=20&offset=0"
```

| Parameter | Meaning |
|---|---|
| `q` | Free-text against name / description / keywords / parties / identifiers |
| `limit` / `offset` | Pagination (limit ≤ 100) |
| `fuzzy` | Enable Tantivy fuzzy matching |
| `mask_sensitive` | Mask party emails and identifier values |
| `date_from` / `date_to` | `start_date` window (yyyy-mm-dd) |
| `event_status`, `event_type` | Facet filters |

### Match against existing records

```bash
curl -X POST http://localhost:8080/api/v1/events/match \
  -H 'content-type: application/json' \
  -d '{
    "name": "Conferance",
    "start_date": "2026-06-01T09:00:00Z",
    "threshold": 0.5
  }'
```

Returns ranked candidates with `score`, `match_quality`, and a
per-component `breakdown` (name / start / end / location / organizer /
performer / attendee / identifier).

### Merge

```bash
curl -X POST http://localhost:8080/api/v1/events/merge \
  -H 'content-type: application/json' \
  -d '{
    "main_event_id": "11111111-1111-1111-1111-111111111111",
    "duplicate_event_id": "22222222-2222-2222-2222-222222222222",
    "merge_reason": "Same conference, two sources"
  }'
```

### Batch deduplication

```bash
curl -X POST http://localhost:8080/api/v1/events/deduplicate \
  -H 'content-type: application/json' \
  -d '{
    "threshold": 0.70,
    "auto_merge_threshold": 0.95,
    "max_candidates": 50
  }'
```

Returns `events_scanned`, `duplicates_found`, `auto_merged`,
`queued_for_review`, and a list of `ReviewQueueItem`s.

### GDPR Article 15 export

```bash
curl "http://localhost:8080/api/v1/events/{id}/export"
```

## Library API examples

### Construct + validate

```rust
use event_service::models::{Event, EventType, Location, Place};
use event_service::validation::validate_event;
use chrono::{TimeZone, Utc};

let start = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap();
let mut event = Event::new("Annual Conference", start);
event.event_type = EventType::Conference;
event.end_date = Some(start + chrono::Duration::hours(2));
event.location.push(Location::Place(Place {
    name: "Greek Theatre".into(),
    ..Default::default()
}));

let errs = validate_event(&event);
assert!(errs.is_empty(), "validation failed: {errs:?}");
```

### Match two events

```rust
use event_service::matching::scoring::{compute_match, MatchWeights};

let a = Event::new("Conference", start);
let b = Event::new("Conferance", start); // typo

let result = compute_match(&a, &b, &MatchWeights::default());
println!("score={:.3} quality={:?}", result.score, result.confidence);
println!("  name:     {:.3}", result.breakdown.name_score);
println!("  start:    {:.3}", result.breakdown.start_score);
println!("  location: {:.3}", result.breakdown.location_score);
```

### Privacy mask + GDPR export

```rust
use event_service::privacy::{mask_event, gdpr_export};

let masked = mask_event(&event);          // identifier values masked, party emails redacted
let export = gdpr_export(&event);         // full JSON for data portability
```

## Configuration

| Variable | Description | Default |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | _required_ |
| `DATABASE_MIN_CONNECTIONS` / `DATABASE_MAX_CONNECTIONS` | Pool sizes | `2` / `10` |
| `SERVER_HOST` | REST bind address | `0.0.0.0` |
| `SERVER_PORT` | REST port | `8080` |
| `PORT` | Web UI port (`cargo run --bin web`) | `5150` |
| `SEARCH_INDEX_PATH` | Tantivy index directory | `./data/search_index` |
| `MATCHING_THRESHOLD` | Default match cutoff | `0.85` |
| `OTLP_ENDPOINT` | OpenTelemetry collector | `http://localhost:4317` |
| `OTLP_SERVICE_NAME` | OTel `service.name` | `event-service` |
| `RUST_LOG` | `tracing-subscriber` filter | `info,event_service=info` |

## Project layout

```
src/
├── lib.rs              # Library root
├── api/                # REST, FHIR (501 stub), gRPC API layers
├── models/             # Domain models (Event, Location, Party, Offer, …)
├── matching/           # Algorithms (name, time, location, party, identifier, phonetic)
├── search/             # Tantivy index + query
├── db/                 # SeaORM models + repositories + audit
├── streaming/          # Event publishing (InMemory + Fluvio stub)
├── validation/         # Validation + normalisation
├── privacy/            # Masking + GDPR export + consent
├── config/             # Env loading + Config struct
├── observability/      # OpenTelemetry setup
├── web/                # Loco app + Tera views + Axum web router
├── bin/web.rs          # cargo run --bin web
└── error.rs

assets/views/           # Tera templates (HTMX + Alpine + Lily)
assets/static/          # lily.css, htmx.min.js, alpine.min.js
config/                 # development.yaml, test.yaml, production.yaml
migrations/             # SeaORM up.sql / down.sql pairs
tests/                  # Integration tests
benches/                # Criterion benchmarks
AGENTS/                 # Reference documentation
```

## Key types

| Type | Module | Description |
|---|---|---|
| `Event` | `models::event` | Core event record (schema.org/Event aligned) |
| `EventStatus` | `models::mod` | Scheduled / Cancelled / MovedOnline / Postponed / Rescheduled / Completed |
| `EventAttendanceMode` | `models::mod` | Offline / Online / Mixed |
| `EventType` | `models::mod` | 29 variants — Generic, Appointment, Conference, Encounter, Shift, … |
| `Location` | `models::mod` | Union of Place / PostalAddress / VirtualLocation / Text |
| `Party` | `models::mod` | Typed (Person / Organization) reference with name + email + URL |
| `Offer` | `models::mod` | Price + currency + availability + validity window |
| `Identifier` | `models::identifier` | Typed external IDs (Booking / Confirmation / Ticket / Encounter / Transaction / …) |
| `EventLink` | `models::event` | Replaces / ReplacedBy / Refer / Seealso |
| `MergeRequest` / `MergeResponse` / `MergeRecord` | `models::merge` | Merge contract + persisted record |
| `ReviewQueueItem` | `models::review_queue` | Pending / Confirmed / Rejected / AutoMerged |
| `MatchResult` / `MatchBreakdown` | `matching::mod` | Score + per-component detail |
| `Consent` | `models::consent` | GDPR consent record |

## Key functions

| Function | Module | Description |
|---|---|---|
| `compute_match` | `matching::scoring` | Probabilistic + deterministic scoring |
| `match_titles` | `matching::algorithms::name_matching` | Jaro-Winkler + Levenshtein + Soundex floor |
| `match_start_dates` | `matching::algorithms::time_matching` | Exponential decay (1 h half-life) |
| `match_window_overlap` | `matching::algorithms::time_matching` | Jaccard overlap of intervals |
| `match_location` | `matching::algorithms::location_matching` | Dispatch by Location variant |
| `match_party` | `matching::algorithms::party_matching` | Best of name / external-id / email |
| `soundex` | `matching::phonetic` | 4-character phonetic code |
| `validate_event` | `validation` | Required fields, time-window guards, capacity invariants |
| `normalize_thing` family | `validation` | URL scheme lowercasing, dedup |
| `mask_event` | `privacy` | Mask identifier values and party emails |
| `gdpr_export` | `privacy` | GDPR Article 15 export |

## Status & roadmap

- **Status** — see [`spec.md §14`](spec.md#14-implementation-status).
- **Tasks** — see [`spec.md §13`](spec.md#13-tasks) for the queue of
  in-flight work with acceptance criteria.
- **Roadmap** — see [`spec.md §15`](spec.md#15-roadmap).
- **Open questions** — see [`spec.md §16`](spec.md#16-open-questions),
  including the FHIR mapping decision (OQ-1).

## Compliance

| Standard | Mechanism |
|---|---|
| HIPAA (clinical use) | Audit log, soft delete, encryption-at-rest, access controls |
| GDPR Art. 15 | `/api/v1/events/{id}/export` |
| GDPR Art. 17 | Soft delete + consent revocation |
| HL7 FHIR R5 | Stub today; pending mapping (§6.8, OQ-1) |
| ISO/IEC 27001 | Operational controls (deployment-side) |

## License

Dual-licensed: MIT OR Apache-2.0.
