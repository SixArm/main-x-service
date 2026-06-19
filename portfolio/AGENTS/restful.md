# RESTful API Reference — Portfolio Entity

The service is loco.rs and returns **raw JSON** — no
`{success, data, error}` envelope (unlike the pre-loco person service).
Source:
[`src/controllers/`](../portfolio-service-with-loco/src/controllers/).
Base URL in development: `http://localhost:5150`.

The service exposes **four matchable collections** with the **identical**
controller shape — `/api/v1/portfolios`, `/api/v1/projects`,
`/api/v1/products`, `/api/v1/programs`. Below, `{collection}` stands for
any of the four; every collection carries the same routes. Matching is
**within a collection only** (the matcher's R-GATE enforces it).

## Endpoints

### Health (loco built-ins)

| Method | Path | Description |
|---|---|---|
| GET | `/_health` | Health check |
| GET | `/_ping` | Liveness ping |

### Work-item CRUD

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/v1/{collection}` | `WorkItem` | `{pid, name}` |
| GET | `/api/v1/{collection}` | — | `[{pid, name}]` (active, most-recent first, cap 100) |
| GET | `/api/v1/{collection}/search?q=` | — | `[{pid, name}]` — `ILIKE` name match (cap 50); blank `q` → `400` |
| GET | `/api/v1/{collection}/{pid}` | — | stored `WorkItem` |
| PUT | `/api/v1/{collection}/{pid}` | `WorkItem` | `{pid, name}` |
| DELETE | `/api/v1/{collection}/{pid}` | — | `{}` (soft delete) |

The service stamps `kind` from the collection on create (a `portfolios`
POST is a Portfolio); a mismatched `kind` in the body is rejected. Child
collections (`projects` / `products` / `programs`) accept / require a
`portfolio_ref`; `portfolios` ignore it.

### Matching

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/v1/{collection}/match` | `{query, candidates}` | ranked `[(index, MatchResult)]` |
| POST | `/api/v1/{collection}/check-duplicates` | `WorkItem` | `[{pid, name, score, confidence, is_match}]`, score-descending |
| POST | `/api/v1/{collection}/deduplicate` | `{threshold?}` | batch scan over the collection's active rows → clusters of candidate duplicates |
| POST | `/api/v1/{collection}/merge` | `{main_pid, duplicate_pid, reason?}` | `{main_pid, duplicate_pid, main}`; `422` equal pids, `404` unknown |
| GET | `/api/v1/{collection}/merges/recent` | — | recent `merge_records` (history + transferred snapshot) |

A cross-kind `query` against the wrong collection scores `0.0` for every
candidate (R-GATE) — matching never crosses collections.

### Sub-resource CRUD

Each sub-resource hangs off a parent work item in **any** collection.
List is the collection GET; create is the collection POST; read / update
/ delete are by the sub-resource id. Soft-delete semantics match the
parent.

| Sub-resource | Collection | Item |
|---|---|---|
| Goals | `GET` / `POST /api/v1/{collection}/{pid}/goals` | `GET` / `PUT` / `DELETE …/goals/{id}` |
| Tasks | `GET` / `POST /api/v1/{collection}/{pid}/tasks` | `GET` / `PUT` / `DELETE …/tasks/{id}` |
| Issues | `GET` / `POST /api/v1/{collection}/{pid}/issues` | `GET` / `PUT` / `DELETE …/issues/{id}` |

Goal writes also mutate the parent's `data.goals[]` payload field (the
goals bridge), keeping the matcher's `Goals` component in sync.

### Derived views (read-only)

| Method | Path | Returns |
|---|---|---|
| GET | `/api/v1/{collection}/{pid}/timeline` | Gantt-shaped rows from work-item + task + goal dates |
| GET | `/api/v1/{collection}/{pid}/burndown` | remaining-work series over the work-item timeframe |

### Cross-service links

A work item links out to other entities (parent portfolio, child work
items, sponsoring organization, lead / assignee people / workers). The
link aggregator surface:

| Method | Path | Returns |
|---|---|---|
| GET | `/api/v1/{collection}/{pid}/links` | typed outbound + inbound links for the work item |
| POST | `/api/v1/{collection}/{pid}/links` | add a typed link `{kind, target}` |
| DELETE | `/api/v1/{collection}/{pid}/links/{id}` | remove a link |

### Bulk import / export

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/v1/{collection}/import` | `[WorkItem]` (NDJSON or array) | per-row `{pid, name}` / error |
| GET | `/api/v1/{collection}/export` | — | full `[WorkItem]` snapshot (active rows) |

### Authentication

| Method | Path | Returns |
|---|---|---|
| GET | `/api/v1/{collection}/whoami` | verified PASETO-token `Claims`; `401` without a valid token |

Short-lived **PASETO v4.public** tokens are verified offline against the
auth-service's published **Ed25519 key** via the embedded
`authentication-verifier` (`src/auth.rs`). The `AuthUser` extractor
requires a token; `MaybeAuthUser` is optional and feeds the audit
`actor`. Blanket `/api/*` auth enforcement (`PORTFOLIO_REQUIRE_AUTH`) +
published-key fetch are follow-ups. (Front-ends use a BFF + cookie
session, so the browser holds no token; the BFF supplies this bearer
server-side.) Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256/JWKS not used).

### Audit & events

| Method | Path | Returns |
|---|---|---|
| GET | `/api/v1/{collection}/audit/recent` | recent `audit_logs` rows (all work items, cap 100) |
| GET | `/api/v1/{collection}/{pid}/audit` | audit trail for one work item |
| GET | `/api/v1/{collection}/events/recent` | recent `WorkItemEvent`s from the in-memory stream |

Each create / update / delete (work item or sub-resource) writes a
best-effort `audit_logs` row (durable) and publishes a
`created`/`updated`/`deleted` event to the in-memory stream. Durable
broker is roadmap.

### API documentation & metrics

| Method | Path | Returns |
|---|---|---|
| GET | `/api-docs/openapi.json` | hand-written OpenAPI 3 document |
| GET | `/swagger-ui` | Swagger UI page (CDN assets) rendering the spec |
| GET | `/metrics.prom` | Prometheus exposition |

## HTTP status codes

| Code | Meaning |
|---|---|
| 200 | Success |
| 400 | Malformed body (loco JSON rejection); blank `q` on search |
| 404 | Unknown or soft-deleted `pid` / sub-resource id |
| 422 | Validation failure: blank `name` on create/update; `kind` mismatch with the collection; blank sub-resource title (family convention) |
| 500 | Internal error |

## Example

```bash
curl -s localhost:5150/api/v1/projects \
  -H 'content-type: application/json' \
  -d '{"kind":"Project",
       "name":"Q3 Platform Migration",
       "portfolio_ref":"0c4f1e2a-…",
       "goals":[{"title":"Ship v2 API","status":"InProgress"}]}'
```

## Front-end consumption

**Files:**
[`src/lib/api/client.ts`](../portfolio-front-end-with-svelte/src/lib/api/client.ts)
(lean fetch wrapper + `ApiError`),
[`src/lib/api/work-items.ts`](../portfolio-front-end-with-svelte/src/lib/api/work-items.ts)
(`WorkItemRepository`, parameterised by collection: CRUD + sub-resource
CRUD + `checkDuplicates` + timeline / burndown reads).

| Route | Endpoints |
|---|---|
| `/{collection}` | `GET /api/v1/{collection}` |
| `/{collection}/new` | `POST /api/v1/{collection}` |
| `/{collection}/[pid]` | `GET`, `DELETE /api/v1/{collection}/{pid}`; `POST …/check-duplicates`; sub-resource lists |
| `/{collection}/[pid]/edit` | `GET`, `PUT /api/v1/{collection}/{pid}` |
| `/{collection}/[pid]/board` | task list / Kanban via `…/{pid}/tasks` |
| `/{collection}/[pid]/issues` | `…/{pid}/issues` |
| `/{collection}/[pid]/goals` | `…/{pid}/goals` |
| `/{collection}/[pid]/timeline` | `GET …/{pid}/timeline` |
| `/{collection}/[pid]/burndown` | `GET …/{pid}/burndown` |

where `{collection}` ∈ `portfolios` / `projects` / `products` /
`programs`. A portfolio detail also rolls up its child work items. Base
URL: `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).

## Matcher library API

```rust
use portfolio_matcher::{WorkItem, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_work_items(&a, &b);
// result.score: f64 in [0.0, 1.0]  (0.0 when a.kind != b.kind — R-GATE)
// result.confidence: High | Medium | Low
// result.is_match: bool
// result.breakdown: per-component Option<f64>
```

SemVer surface = `lib.rs` re-exports: `WorkItem`, `WorkItemKind`,
`WorkItemStatus`, `WorkItemIdentifier`, `IdentifierScheme`, `Goal`,
`GoalStatus`, `WorkItemRelationship`, `RelationKind`, `MatchingEngine`,
`MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`, `Error`,
`Result`.
