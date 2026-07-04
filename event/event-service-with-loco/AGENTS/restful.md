# RESTful API Reference

All REST endpoints are mounted under `/api/v1`.

## Library API — bridge to the canonical `event-matcher` crate

The service embeds the sibling `event-matcher` crate and re-exports it
from `src/matching/mod.rs` as `matcher_lib`. Pair it with
`adapter::to_matcher_event` to score two service records through the
canonical algorithm:

```rust
use event_service::matching::adapter::to_matcher_event;
use event_service::matching::matcher_lib::{MatchingEngine, MatchConfig};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_events(
    &to_matcher_event(&a),
    &to_matcher_event(&b),
);
// result.score: f64 in [0.0, 1.0]
// result.is_match: bool
// result.confidence: High | Medium | Low
// result.breakdown: per-field Option<f64>
```

Field-routing rules are documented inline in
[`src/matching/adapter.rs`](../src/matching/adapter.rs) (DateTime →
RFC 3339; `Vec<Location>` → first variant-dispatched matcher
`Location`; `Vec<Party>` → first organizer name + performers
`Vec<String>`; identifier `system` URI → `EventIdScheme`) and pinned
by [`tests/duplicate_detection.rs`](../tests/duplicate_detection.rs).

## Prometheus metrics

| Method | Path | Notes |
|---|---|---|
| GET | `/metrics.prom` | Prometheus text-exposition format (`text/plain; version=0.0.4`) for scraping |

Configure your scraper with `metrics_path: /metrics.prom`. The metric
inventory (entity-CRUD counters, HTTP request counter, latency
histograms) is in [`src/metrics.rs`](../src/metrics.rs). The handler
is [`api::rest::handlers::metrics_prom`](../src/api/rest/handlers.rs).

## Health

| Method | Path | Notes |
|---|---|---|
| GET | `/health` | Health check (returns `HealthResponse`) |

## Auth

| Method | Path | Notes |
|---|---|---|
| GET | `/whoami` | Echo the verified bearer-token claims (`401` without a valid token) |

Bearer tokens are PASETO `v4.public` (Ed25519) minted by the central
authentication-service and verified **offline** against its published
key set (`/.well-known/paseto-keys`) via the `authentication-verifier`
crate — no shared secret, no introspection call. Configure with
`EVENT_PASETO_KEYS` (key-set JSON), `EVENT_TOKEN_ISSUER`, and
`EVENT_TOKEN_AUDIENCE`. Handlers opt in by taking an `AuthUser`
argument (`src/api/rest/auth.rs`).

## Event CRUD

| Method | Path | Notes |
|---|---|---|
| POST | `/events` | Create event. Returns `409` with `DuplicateCheckResponse` when potential duplicates are detected. Returns `422` on validation error. |
| GET | `/events/{id}` | Get one event |
| PUT | `/events/{id}` | Replace an event |
| DELETE | `/events/{id}` | Soft-delete |

## Search

| Method | Path | Notes |
|---|---|---|
| GET | `/events/search` | Full-text / fuzzy search |

`SearchQuery` parameters:

| Field | Type | Notes |
|---|---|---|
| `q` | string | Free-text query against name / description / keywords / parties / identifiers |
| `limit` | usize | Default 10, capped at 100 |
| `offset` | usize | Pagination |
| `fuzzy` | bool | Use fuzzy title search |
| `mask_sensitive` | bool | Mask party emails and identifier values |
| `date_from` | `yyyy-mm-dd` | Filter to `start_date >= date_from` |
| `date_to` | `yyyy-mm-dd` | Filter to `start_date <= date_to` |
| `event_status` | `EventStatus` | Filter |
| `event_type` | `EventType` | Filter |

## Matching & deduplication

| Method | Path | Notes |
|---|---|---|
| POST | `/events/match` | Find candidate matches for a request event |
| POST | `/events/check-duplicates` | Real-time duplicate check (used internally by `POST /events`) |
| POST | `/events/merge` | Merge two events (surviving record + duplicate) |
| POST | `/events/deduplicate` | Batch dedup scan; returns `ReviewQueueItem`s |

Blocking strategy for `POST /events/match` and `POST /events/check-duplicates`:
the search index is queried for events with similar names whose
`start_date` matches the request's date (yyyy-mm-dd).

## Privacy

| Method | Path | Notes |
|---|---|---|
| GET | `/events/{id}/export` | GDPR right-of-access export |
| GET | `/events/{id}/masked` | Masked view (identifier values + party emails) |

## Audit

| Method | Path | Notes |
|---|---|---|
| GET | `/events/{id}/audit` | Audit logs for one event |
| GET | `/audit/recent` | Recent audit activity |
| GET | `/audit/user` | Audit logs for one user |

## FHIR

`/fhir/Event` endpoints currently return `501 Not Implemented` with
an `OperationOutcome` body. See
[`src/api/fhir/mod.rs`](../src/api/fhir/mod.rs) for the rationale —
the schema.org/Event → FHIR R5 mapping isn't fixed yet
(Appointment vs Encounter vs other event-pattern resources).

## Response envelope

```json
{ "success": true, "data": {…}, "error": null }
```

Error envelope:

```json
{ "success": false, "data": null,
  "error": { "code": "…", "message": "…", "details": null } }
```

## HTTP status codes

| Code | Meaning |
|---|---|
| 200 | OK |
| 201 | Created |
| 204 | Deleted |
| 400 | Bad request |
| 404 | Not found |
| 409 | Duplicate detected (on create) |
| 422 | Validation error |
| 500 | Internal error |
| 501 | FHIR endpoint stub |

## Source files

- `src/api/mod.rs` — `ApiResponse`, `ApiError`
- `src/api/rest/mod.rs` — router, OpenAPI doc, `serve`
- `src/api/rest/handlers.rs` — all REST handlers
- `src/api/rest/state.rs` — `AppState`
- `src/api/fhir/mod.rs` + `handlers.rs` — FHIR stubs
- `src/api/grpc/mod.rs` — gRPC stub
