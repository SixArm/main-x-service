# RESTful API Reference — Portfolio Entity

The service is loco.rs and returns **raw JSON** — no
`{success, data, error}` envelope (unlike the pre-loco person service).
Source:
[`src/controllers/`](../project-portfolio-management-service-with-loco/src/controllers/).
Base URL in development: `http://localhost:5150`.

The service exposes **one matchable collection** — `/api/plans`. Every
record is a `Plan` with an optional descriptive `kind` label; there is no
per-kind collection and no `{collection}` path segment. Matching is
**kind-agnostic** — any two plans may match.

## Endpoints

### Health (loco built-ins)

| Method | Path | Description |
|---|---|---|
| GET | `/_health` | Health check |
| GET | `/_ping` | Liveness ping |

### Plan CRUD

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/plans` | `Plan` | `{pid, name}` |
| GET | `/api/plans` | — | `[{pid, name}]` (active, most-recent first, cap 100) |
| GET | `/api/plans/search?q=` | — | `[{pid, name}]` — `ILIKE` name match (cap 50); blank `q` → `400` |
| GET | `/api/plans/{pid}` | — | stored `Plan` |
| PUT | `/api/plans/{pid}` | `Plan` | `{pid, name}` |
| DELETE | `/api/plans/{pid}` | — | `{}` (soft delete) |

`kind` is an optional descriptive label taken verbatim from the body (no
per-kind collection to stamp it from). A `parent_ref` may point at any
other plan (recursive containment); a `parent_ref` that targets the plan
itself or one of its descendants (a containment cycle) is rejected `422`.

### Matching

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/plans/match` | `{query, candidates}` | ranked `[(index, MatchResult)]` |
| POST | `/api/plans/check-duplicates` | `Plan` | `[{pid, name, score, confidence, is_match}]`, score-descending |
| POST | `/api/plans/deduplicate` | `{threshold?}` | batch scan over active plan rows → clusters of candidate duplicates |
| POST | `/api/plans/merge` | `{main_pid, duplicate_pid, reason?}` | `{main_pid, duplicate_pid, main}`; `422` equal pids (self-merge), `404` unknown |
| GET | `/api/plans/merges/recent` | — | recent `merge_records` (history + transferred snapshot) |

Any two plans may match or merge regardless of their `kind` labels —
matching is kind-agnostic.

### Sub-resource CRUD

Each sub-resource hangs off a parent plan. Read / update / delete are by
the sub-resource id. Soft-delete semantics match the parent.

| Sub-resource | Collection | Item |
|---|---|---|
| Goals | `GET` / `POST /api/plans/{pid}/goals` | `GET` / `PUT` / `DELETE …/goals/{id}` |
| Tasks | `GET` / `POST /api/plans/{pid}/tasks` | `GET` / `PUT` / `DELETE …/tasks/{id}` |
| Issues | `GET` / `POST /api/plans/{pid}/issues` | `GET` / `PUT` / `DELETE …/issues/{id}` |

Goal writes also mutate the parent's `data.goals[]` payload field (the
goals bridge), keeping the matcher's `Goals` component in sync.

### Derived views (read-only)

| Method | Path | Returns |
|---|---|---|
| GET | `/api/plans/{pid}/timeline` | Gantt-shaped rows from plan + task + goal dates |
| GET | `/api/plans/{pid}/burndown` | remaining-work series over the plan timeframe |

### Cross-service links

A plan links out to other entities (parent plan, child plans, sponsoring
organization, lead / assignee people / workers). The link aggregator
surface:

| Method | Path | Returns |
|---|---|---|
| GET | `/api/plans/{pid}/links` | typed outbound + inbound links for the plan |
| POST | `/api/plans/{pid}/links` | add a typed link `{kind, target}` |
| DELETE | `/api/plans/{pid}/links/{id}` | remove a link |

### Bulk import / export

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/plans/import` | `[Plan]` (NDJSON or array) | per-row `{pid, name}` / error |
| GET | `/api/plans/export` | — | full `[Plan]` snapshot (active rows) |

### Authentication

| Method | Path | Returns |
|---|---|---|
| GET | `/api/plans/whoami` | verified PASETO-token `Claims`; `401` without a valid token |

Short-lived **PASETO v4.public** tokens are verified offline against the
auth-service's published **Ed25519 key** via the embedded
`authentication-verifier` (`src/auth.rs`). The `AuthUser` extractor
requires a token; `MaybeAuthUser` is optional and feeds the audit
`actor`. Blanket `/api/*` auth enforcement (`PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`) +
published-key fetch are follow-ups. (Front-ends use a BFF + cookie
session, so the browser holds no token; the BFF supplies this bearer
server-side.) Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256/JWKS not used).

### Audit & events

| Method | Path | Returns |
|---|---|---|
| GET | `/api/plans/audit/recent` | recent `audit_logs` rows (all plans, cap 100) |
| GET | `/api/plans/{pid}/audit` | audit trail for one plan |
| GET | `/api/plans/events/recent` | recent `PlanEvent`s from the in-memory stream |

Each create / update / delete (plan or sub-resource) writes a
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
| 422 | Validation failure: blank `name` on create/update; containment cycle on `parent_ref`; blank sub-resource title (family convention) |
| 500 | Internal error |

## Example

```bash
curl -s localhost:5150/api/plans \
  -H 'content-type: application/json' \
  -d '{"kind":"Project",
       "name":"Q3 Platform Migration",
       "parent_ref":"0c4f1e2a-…",
       "goals":[{"title":"Ship v2 API","status":"InProgress"}]}'
```

## Front-end consumption

**Files:**
[`src/lib/api/client.ts`](../project-portfolio-management-front-end-with-svelte/src/lib/api/client.ts)
(lean fetch wrapper + `ApiError`),
[`src/lib/api/plans.ts`](../project-portfolio-management-front-end-with-svelte/src/lib/api/plans.ts)
(`PlanRepository`: CRUD + sub-resource CRUD + `checkDuplicates` +
timeline / burndown reads).

| Route | Endpoints |
|---|---|
| `/plans` | `GET /api/plans` |
| `/plans/new` | `POST /api/plans` |
| `/plans/[pid]` | `GET`, `DELETE /api/plans/{pid}`; `POST …/check-duplicates`; sub-resource lists |
| `/plans/[pid]/edit` | `GET`, `PUT /api/plans/{pid}` |
| `/plans/[pid]/board` | task list / Kanban via `…/{pid}/tasks` |
| `/plans/[pid]/issues` | `…/{pid}/issues` |
| `/plans/[pid]/goals` | `…/{pid}/goals` |
| `/plans/[pid]/timeline` | `GET …/{pid}/timeline` |
| `/plans/[pid]/burndown` | `GET …/{pid}/burndown` |

A plan detail also rolls up its child plans (recursive containment). Base
URL: `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).

## Matcher library API

```rust
use project_portfolio_management_matcher::{Plan, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_plans(&a, &b);
// result.score: f64 in [0.0, 1.0]  (kind-agnostic — kind never gates)
// result.confidence: High | Medium | Low
// result.is_match: bool
// result.breakdown: per-component Option<f64>
```

SemVer surface = `lib.rs` re-exports: `Plan`, `PlanKind`,
`PlanStatus`, `PlanIdentifier`, `IdentifierScheme`, `Goal`,
`GoalStatus`, `PlanRelationship`, `RelationKind`, `MatchingEngine`,
`MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`, `Error`,
`Result`.
