## 9. API Surface

Complete REST reference:
[service `AGENTS/restful.md`](../thing-service-rust-crate/AGENTS/restful.md).
Front-end consumption map:
[front-end §9](../thing-front-end-with-svelte/spec/09-api-consumption.md).

### 9.1 REST endpoints (service)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/health` | Health check |
| POST | `/api/things` | Create thing (`409` on duplicate detected) |
| GET | `/api/things/{id}` | Get thing |
| PUT | `/api/things/{id}` | Update thing |
| DELETE | `/api/things/{id}` | Soft delete thing |
| GET | `/api/things/search` | Search (full-text / fuzzy / boolean, paginated) |
| POST | `/api/things/match` | Match a candidate against the index |
| POST | `/api/things/check-duplicates` | Explicit duplicate check without create |
| POST | `/api/things/merge` | Merge duplicate into main |
| POST | `/api/things/deduplicate` | Batch deduplication scan |
| GET | `/api/things/{id}/export` | GDPR data export |
| GET | `/api/things/{id}/masked` | Masked thing view |
| GET | `/api/things/{id}/audit` | Per-thing audit logs |
| GET | `/api/audit/recent` | Recent system-wide audit activity |
| GET | `/api/audit/user` | Per-user audit logs |

Standard response envelope; `422` on validation failure; Swagger UI
at `/swagger-ui`. No FHIR surface — Things are not a FHIR-resource
concern. gRPC is stubbed (service spec §13 T-3).

> Note: code and OpenAPI use `/api/things/check-duplicates`; the
> service's `AGENTS/restful.md` table says `/api/things/duplicates` —
> doc drift tracked as §13 T-2.

### 9.2 Front-end routes

| Route | Purpose | Calls |
|---|---|---|
| `/` | Dashboard — health + recent audit | `GET /api/health`, `GET /api/audit/recent` |
| `/things` | List + search (SVAR DataGrid) | `GET /api/things/search` |
| `/things/new` | Create; surfaces 409 duplicate candidates | `POST /api/things` |
| `/things/[id]` | Detail view | `GET /api/things/{id}`, `DELETE …` |
| `/things/[id]/edit` | Edit | `GET` + `PUT /api/things/{id}` |
| `/things/[id]/audit` | Per-thing audit log | `GET /api/things/{id}/audit` |
| `/things/match` | Score a hypothetical record | `POST /api/things/match` |
| `/things/merge` | Merge two things with preview | `POST /api/things/merge` |

Endpoints available but **not yet routed** in the UI:
`check-duplicates`, `deduplicate`, `masked`, `export` (front-end spec
§13 T-17–T-20; entity §13 T-7).

### 9.3 Library surface (matcher)

`MatchingEngine` (`new` / `default_config`), `deterministic_match`,
`match_things`, `match_one_to_many`, `rank_one_to_many`,
`ThingBuilder`, `MatchConfig` (+ `strict()` / `lenient()` presets),
`Normalizer`, `Scorer`. SemVer-stable, `#[non_exhaustive]` types. See
[matcher spec §8](../thing-matcher-rust-crate/spec/08-public-api-surface.md).
