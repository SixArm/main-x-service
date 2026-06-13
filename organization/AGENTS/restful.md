# RESTful API Reference — Organization Entity

Entity-level endpoint map. Source of truth for behaviour:
[`src/controllers/organizations.rs`](../organization-service-rust-crate/src/controllers/organizations.rs);
machine-readable: `GET /api-docs/openapi.json`
([`src/openapi.rs`](../organization-service-rust-crate/src/openapi.rs));
contract summary: entity [spec §9](../spec/09-api-surface.md).

Default base URL: `http://localhost:5150` (loco default port).

## Conventions

- **Raw loco JSON — no `{success, data, error}` envelope** (this
  differs from the older Axum-native entities like person).
- Wire field names are **snake_case** (`legal_name`,
  `founding_date`, …).
- The organization request/response body **is** the
  `organization_matcher::Organization` shape.
- No authentication yet (JWT via the auth-service JWKS is queued —
  entity spec §13 T-9).

## Endpoints

### CRUD

| Method | Path | Body → Response | Errors |
|---|---|---|---|
| POST | `/api/organizations` | `Organization` → `{pid, name}` | `422` blank name (validation; family convention) |
| GET | `/api/organizations` | → `[{pid, name}]` (active, newest first, cap 100) | |
| GET | `/api/organizations/{pid}` | → `Organization` (the stored payload) | `404` unknown / deleted |
| PUT | `/api/organizations/{pid}` | `Organization` → `{pid, name}` (full replace) | `404`; `422` blank name |
| DELETE | `/api/organizations/{pid}` | → `{}` (soft delete) | `404` |

### Search

| Method | Path | Notes |
|---|---|---|
| GET | `/api/organizations/search?q=acme` | Case-insensitive `ILIKE %q%` on the denormalised name; active rows; cap 50; blank `q` → `400` |

### Matching

| Method | Path | Body → Response |
|---|---|---|
| POST | `/api/organizations/match` | `{query, candidates}` → ranked `[(index, MatchResult)]`; pure scoring, no persistence |
| POST | `/api/organizations/check-duplicates` | `Organization` → `[{pid, name, score, confidence, is_match}]` score-desc (scans ≤ 1 000 stored rows) |
| POST | `/api/organizations/merge` | `{main_pid, duplicate_pid, reason?}` → `{main_pid, duplicate_pid, main}`; `422` equal pids, `404` unknown |
| GET | `/api/organizations/merges/recent` | merge-history rows (transferred snapshot) |

### Audit & events

| Method | Path | Notes |
|---|---|---|
| GET | `/api/organizations/audit/recent` | Newest 100 audit rows system-wide |
| GET | `/api/organizations/{pid}/audit` | Per-record trail; `400` invalid pid |
| GET | `/api/organizations/events/recent` | Newest 100 `OrgEvent`s from the in-memory stream |

### Docs & health

| Method | Path | Notes |
|---|---|---|
| GET | `/api-docs/openapi.json` | Hand-written OpenAPI 3 (the matcher crate has no `utoipa`; the doc is authored in `src/openapi.rs` — keep it in sync by hand) |
| GET | `/swagger-ui` | Interactive docs |
| GET | `/_health`, `/_ping` | loco defaults |

## Worked example

```bash
curl -s localhost:5150/api/organizations -H 'content-type: application/json' \
  -d '{"name":"Acme, Inc.","legal_name":"Acme Incorporated","jurisdiction":"US",
       "url":"https://acme.com",
       "identifiers":[{"scheme":"lei","value":"5493001KJTIIGC8Y1R12"}]}'
# -> {"pid":"…","name":"Acme, Inc."}

curl -s localhost:5150/api/organizations/check-duplicates \
  -H 'content-type: application/json' \
  -d '{"name":"ACME Corporation","jurisdiction":"US"}'
# -> [{"pid":"…","name":"Acme, Inc.","score":0.97,"confidence":"High","is_match":true}]
```

## Front-end consumption

Client: [`src/lib/api/client.ts`](../organization-front-end-with-svelte/src/lib/api/client.ts)
(lean fetch wrapper) +
[`organizations.ts`](../organization-front-end-with-svelte/src/lib/api/organizations.ts)
(repository). Base URL via `PUBLIC_API_BASE_URL`.

| UI action | Endpoint |
|---|---|
| List (`/`) | `GET /api/organizations` |
| Create (`/new`) | `POST /api/organizations` |
| Detail (`/[pid]`) | `GET /api/organizations/{pid}` |
| Edit (`/[pid]/edit`) | `PUT /api/organizations/{pid}` |
| Delete | `DELETE /api/organizations/{pid}` |
| Check duplicates | `POST /api/organizations/check-duplicates` |

Not yet consumed by the UI: `/search`, audit endpoints, event stream
(front-end spec §13; entity spec §13 T-11).
