## 9. API Surface

The entity's externally consumable surface is the **service's HTTP
API** plus the **front-end's operator routes**. Complete endpoint
reference (parameters, payloads, status codes):
[`person-service-rust-crate/AGENTS/restful.md`](../person-service-rust-crate/AGENTS/restful.md).

### 9.1 Service REST API (summary)

| Area | Endpoints |
|---|---|
| Health | `GET /api/health` |
| CRUD | `POST /api/persons` (real-time dup-detect, `409`), `GET/PUT/DELETE /api/persons/{id}` |
| Search | `GET /api/persons/search` (`q`, `limit`, `offset`, `fuzzy`, `phonetic`, `mask_sensitive`) |
| Matching | `POST /api/persons/match`, `POST /api/persons/check-duplicates` |
| Merge / dedup | `POST /api/persons/merge`, `POST /api/persons/deduplicate` |
| Privacy | `GET /api/persons/{id}/export` (GDPR), `GET /api/persons/{id}/masked` |
| Audit | `GET /api/persons/{id}/audit`, `GET /api/audit/recent`, `GET /api/audit/user` |
| FHIR R5 | `GET/POST/PUT/DELETE /fhir/Person[/{id}]`, `GET /fhir/Person` (search) |
| Docs | Swagger UI at `/swagger-ui` (OpenAPI 3.0) |
| Metrics | `GET /metrics.prom` (Prometheus text exposition) |
| gRPC | Tonic stub — not yet implemented (service §13 T-6) |

All REST endpoints return the envelope
`{ "success": bool, "data": …, "error": … }`; `409` carries duplicate
candidates, `422` carries validation errors.

### 9.2 Front-end operator routes

| Route | Purpose | Consumes |
|---|---|---|
| `/` | Dashboard — service health + recent audit activity | `/api/health`, `/api/audit/recent` |
| `/persons` | List & search grid (full-text, fuzzy, phonetic) | `/api/persons/search` |
| `/persons/new` | Create; surfaces `409` duplicate candidates inline | `POST /api/persons` |
| `/persons/[id]` | Detail — identity, identifiers, addresses, telecom, emergency contacts | `GET /api/persons/{id}` |
| `/persons/[id]/edit` | Edit | `PUT /api/persons/{id}` |
| `/persons/[id]/audit` | Per-person audit log | `GET /api/persons/{id}/audit` |
| `/persons/match` | Match check — score a hypothetical record | `POST /api/persons/match` |
| `/persons/merge` | Merge two persons (main + duplicate) | `POST /api/persons/merge` |

Front-end consumption contract:
[front-end spec §9](../person-front-end-with-svelte/spec/09-api-consumption.md).

### 9.3 Matcher library API

Not an HTTP surface — a Rust API (`MatchingEngine`, `MatchConfig`
presets `strict` / `default` / `lenient`, `MatchResult` with `score`,
`is_match`, `confidence`, `breakdown`). Reference:
[matcher spec §11](../person-matcher-rust-crate/spec/11-public-api-specification.md).
The service reaches it through the adapter (§5.3), never around it.
