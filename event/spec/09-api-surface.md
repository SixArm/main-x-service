## 9. API Surface

Complete endpoint reference:
[`event-service-with-loco/AGENTS/restful.md`](../event-service-with-loco/AGENTS/restful.md).
Front-end consumption map:
[front-end spec §9](../event-front-end-with-svelte/spec/09-api-consumption.md).

### 9.1 REST API (service, under `/api/v1`)

| Method | Path | Purpose |
|---|---|---|
| GET | `/health` | Health check |
| POST | `/events` | Create; `409` + candidates on duplicate; `422` on validation |
| GET | `/events/{id}` | Read one |
| PUT | `/events/{id}` | Replace |
| DELETE | `/events/{id}` | Soft-delete |
| GET | `/events/search` | Full-text / fuzzy search; date-range, status, type filters; pagination; optional masking |
| POST | `/events/match` | Ranked candidate matches with score breakdowns |
| POST | `/events/check-duplicates` | Explicit duplicate check |
| POST | `/events/merge` | Merge survivor + duplicate |
| POST | `/events/deduplicate` | Batch dedup scan → review-queue items |
| GET | `/events/{id}/masked` | Masked view |
| GET | `/events/{id}/export` | GDPR right-of-access export |
| GET | `/events/{id}/audit` | Audit log for one Event |
| GET | `/audit/recent` | Recent system-wide audit activity |
| GET | `/audit/user` | Audit logs for one user |

Also: Swagger UI at `/swagger-ui`; Prometheus at `/metrics.prom`;
`/fhir/Event/*` returns `501` (stub); gRPC stubbed. Standard
envelope `{ "success", "data", "error" }`.

### 9.2 Front-end routes

| Route | Purpose | Endpoints used |
|---|---|---|
| `/` | Dashboard — health + recent audit | `GET /health`, `GET /audit/recent` |
| `/events` | List & search with SVAR DataGrid | `GET /events/search` |
| `/events/new` | Create; surfaces 409 duplicate candidates inline | `POST /events` |
| `/events/[id]` | Detail view; soft-delete button | `GET /events/{id}`, `DELETE /events/{id}` |
| `/events/[id]/edit` | Edit (whole-record re-PUT) | `PUT /events/{id}` |
| `/events/[id]/audit` | Per-Event audit log | `GET /events/{id}/audit` |
| `/events/match` | Match check — score a hypothetical record | `POST /events/match` |
| `/events/merge` | Merge two Events (main + duplicate) with preview | `POST /events/merge` |

Not yet routed in the UI (available on the service):
`check-duplicates`, `deduplicate`, `masked`, `export` — front-end
spec §13 T-17 / T-18 / T-19 / T-20.

### 9.3 Library API (matcher)

`MatchingEngine::{match_events, deterministic_match}` over
builder-constructed `Event` records, with `MatchConfig` presets
(default 0.80 / strict 0.95 / lenient 0.65). SemVer contract:
[matcher spec §9](../event-matcher-rust-crate/spec/09-public-api-contract-semver.md).
