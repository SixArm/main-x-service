## 9. API Surface

Complete endpoint reference with parameters, envelopes, and status
codes: [service `AGENTS/restful.md`](../worker-service-with-loco/AGENTS/restful.md).
This section is the entity-level summary.

### 9.1 Service REST API (consumed by the front-end)

| Area | Method + Path | Purpose |
|---|---|---|
| Health | `GET /api/health` | Liveness for orchestration + front-end dashboard |
| CRUD | `POST /api/workers` | Create (real-time dedup; `409` with candidates) |
| | `GET /api/workers/{id}` | Read |
| | `PUT /api/workers/{id}` | Update |
| | `DELETE /api/workers/{id}` | Soft delete |
| Search | `GET /api/workers/search` | Full-text / fuzzy / phonetic; `q`, `limit`, `offset`, `fuzzy`, `phonetic`, `mask_sensitive` |
| Matching | `POST /api/workers/match` | Score input against the index; per-component breakdown |
| Dedup | `POST /api/workers/check-duplicates` | Check without creating |
| | `POST /api/workers/merge` | Merge duplicate into main |
| | `POST /api/workers/deduplicate` | Batch scan with thresholds |
| Privacy | `GET /api/workers/{id}/export` | GDPR Article 15 export |
| | `GET /api/workers/{id}/masked` | Masked view |
| Audit | `GET /api/workers/{id}/audit` | Per-worker trail |
| | `GET /api/audit/recent` | Recent system-wide |
| | `GET /api/audit/user` | Per-user |
| Docs | `GET /swagger-ui` | OpenAPI 3.0 (utoipa) |
| Metrics | `GET /metrics.prom` | Prometheus text exposition |

Other service tiers: FHIR R5 Practitioner under `/fhir/*`
(service §6.8), gRPC stub (Tonic). The front-end uses neither.

### 9.2 Front-end routes (operator UI)

| Route | Purpose | Calls |
|---|---|---|
| `/` | Dashboard — service health + recent audit | health, audit/recent |
| `/workers` | List & search (SVAR DataGrid) | search |
| `/workers/new` | Create; surfaces `409` duplicate candidates inline | create |
| `/workers/[id]` | Detail — identity, identifiers, addresses, telecom, emergency contacts | read |
| `/workers/[id]/edit` | Edit | read, update |
| `/workers/[id]/audit` | Per-worker audit log | audit |
| `/workers/match` | Match check — score a hypothetical record | match |
| `/workers/merge` | Merge two workers (main + duplicate) | merge |

Front-end detail:
[front-end README](../worker-front-end-with-svelte/README.md),
[front-end spec §9 API Consumption](../worker-front-end-with-svelte/spec/09-api-consumption.md).

### 9.3 Contract conventions

- Envelope: `{ "success": bool, "data": …, "error": { code, message, details } }`.
- Status codes: `200/201/204` success; `409` duplicate-on-create;
  `422` validation; `404`; `500`.
- Base URL injected into the front-end via `PUBLIC_API_BASE_URL`
  (default `http://localhost:8080`).
- Matcher: not a network API. Its public surface is the Rust API in
  [matcher §11](../worker-matcher-rust-crate/spec/11-public-api-specification.md);
  it reaches the wire only through the service.
