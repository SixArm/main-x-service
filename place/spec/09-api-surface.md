## 9. API Surface

### 9.1 Service REST API (summary)

Complete reference:
[`place-service-rust-crate/AGENTS/restful.md`](../place-service-rust-crate/AGENTS/restful.md)
and service [spec §9](../place-service-rust-crate/spec/09-api-surface.md).
15 endpoints; standard response envelope; `409` on duplicate-detected
create; `422` on validation failure.

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/health` | Health check |
| POST | `/api/places` | Create (real-time duplicate detection) |
| GET / PUT / DELETE | `/api/places/{id}` | Read / update / soft delete |
| GET | `/api/places/search` | Full-text / fuzzy search (+ `mask_sensitive`) |
| GET | `/api/places/nearby` | Geo-radius search (`lat`, `lon`, `radius_km`) |
| POST | `/api/places/match` | Score a hypothetical record |
| POST | `/api/places/check-duplicates` | Explicit duplicate check |
| POST | `/api/places/merge` | Merge main + duplicate |
| POST | `/api/places/deduplicate` | Batch deduplication scan |
| GET | `/api/places/{id}/export` | GDPR Art. 15 export |
| GET | `/api/places/{id}/masked` | Masked view |
| GET | `/api/places/{id}/audit` | Per-place audit log |
| GET | `/api/audit/recent`, `/api/audit/user` | System / user audit |

Docs: Swagger UI at `/swagger-ui`. Metrics: Prometheus at
`/metrics.prom`. gRPC (Tonic): stubbed.

> Drift note (updated 2026-06-13): the code serves
> `POST /api/places/check-duplicates`; the crate's `AGENTS/restful.md`
> and spec §6.4 now both agree. Remaining: the front-end client calls
> `/api/places/duplicates` and a service route test is missing —
> tracked as [§13](13-tasks.md) E-1.

### 9.2 Front-end routes (summary)

Full set: front-end
[spec §5](../place-front-end-with-svelte/spec/05-information-architecture.md)
and [README](../place-front-end-with-svelte/README.md).

| Route | Purpose | Service endpoints used |
|---|---|---|
| `/` | Dashboard — health + recent audit | `/api/health`, `/api/audit/recent` |
| `/places` | List & search (SVAR DataGrid) | `/api/places/search` |
| `/places/new` | Create; surfaces 409 candidates inline | `POST /api/places` |
| `/places/[id]` | Detail view | `GET /api/places/{id}` |
| `/places/[id]/edit` | Edit | `PUT /api/places/{id}` |
| `/places/[id]/audit` | Per-place audit log | `GET /api/places/{id}/audit` |
| `/places/match` | Match check against the index | `POST /api/places/match` |
| `/places/merge` | Merge two places | `POST /api/places/merge` |

### 9.3 Matcher library API (summary)

Public contract: matcher
[spec §9](../place-matcher-rust-crate/spec/09-public-api-contract-semver.md).
Key entry points: `Place::builder()`, `MatchingEngine::new(MatchConfig)`,
`match_places(&a, &b) -> MatchResult { score, is_match, confidence,
breakdown }`, plus batch scoring/ranking. Defaults: threshold `0.80`
(strict `0.95`, lenient `0.65`); coordinates Gaussian decay scale
`50 m`.
