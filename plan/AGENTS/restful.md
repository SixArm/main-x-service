# RESTful API Reference — Plan Entity

The service is loco.rs and returns **raw JSON** — no
`{success, data, error}` envelope (unlike the pre-loco person
service). Source:
[`src/controllers/plans.rs`](../plan-service-with-loco/src/controllers/plans.rs).
Base URL in development: `http://localhost:5150`. All plan routes are
versioned under `/api/v1/plans/…`.

## Endpoints

### Health (loco built-ins)

| Method | Path | Description |
|---|---|---|
| GET | `/_health` | Health check |
| GET | `/_ping` | Liveness ping |

### Plan CRUD

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/v1/plans` | `Plan` | `{pid, name}` |
| GET | `/api/v1/plans` | — | `[{pid, name}]` (active, most-recent first, cap 100) |
| GET | `/api/v1/plans/search?q=` | — | `[{pid, name}]` — `ILIKE` name match (cap 50); blank `q` → `400` |
| GET | `/api/v1/plans/{pid}` | — | stored `Plan` |
| PUT | `/api/v1/plans/{pid}` | `Plan` | `{pid, name}` |
| DELETE | `/api/v1/plans/{pid}` | — | `{}` (soft delete) |

### Matching

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/v1/plans/match` | `{query, candidates}` | ranked `[(index, MatchResult)]` |
| POST | `/api/v1/plans/check-duplicates` | `Plan` | `[{pid, name, score, confidence, is_match}]`, score-descending |
| POST | `/api/v1/plans/deduplicate` | `{threshold?}` | batch scan over active rows → clusters of candidate duplicates |
| POST | `/api/v1/plans/merge` | `{main_pid, duplicate_pid, reason?}` | `{main_pid, duplicate_pid, main}`; `422` equal pids, `404` unknown |
| GET | `/api/v1/plans/merges/recent` | — | recent `merge_records` (history + transferred snapshot) |

### Sub-resource CRUD

Each sub-resource hangs off a parent plan. List is the collection
GET; create is the collection POST; read / update / delete are by
the sub-resource id. Soft-delete semantics match the parent.

| Sub-resource | Collection | Item |
|---|---|---|
| Goals | `GET` / `POST /api/v1/plans/{pid}/goals` | `GET` / `PUT` / `DELETE …/goals/{id}` |
| Tasks | `GET` / `POST /api/v1/plans/{pid}/tasks` | `GET` / `PUT` / `DELETE …/tasks/{id}` |
| Issues | `GET` / `POST /api/v1/plans/{pid}/issues` | `GET` / `PUT` / `DELETE …/issues/{id}` |
| Posts | `GET` / `POST /api/v1/plans/{pid}/posts` | `GET` / `PUT` / `DELETE …/posts/{id}` |
| Comments | `GET` / `POST /api/v1/plans/{pid}/comments` | `GET` / `PUT` / `DELETE …/comments/{id}` |
| Members | `GET` / `POST /api/v1/plans/{pid}/members` | `GET` / `PUT` / `DELETE …/members/{id}` |

Comments carry a `target` (task / issue / post) and may be filtered
with `?target=…` on the collection GET.

### Derived views (read-only)

| Method | Path | Returns |
|---|---|---|
| GET | `/api/v1/plans/{pid}/timeline` | Gantt-shaped rows from plan + task + goal dates |
| GET | `/api/v1/plans/{pid}/burndown` | remaining-work series over the plan timeframe |

### Cross-service links

A plan links out to other entities (parent / child plans, sponsoring
organization, member people / workers). The link aggregator surface:

| Method | Path | Returns |
|---|---|---|
| GET | `/api/v1/plans/{pid}/links` | typed outbound + inbound links for the plan |
| POST | `/api/v1/plans/{pid}/links` | add a typed link `{kind, target}` |
| DELETE | `/api/v1/plans/{pid}/links/{id}` | remove a link |

### Bulk import / export

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/v1/plans/import` | `[Plan]` (NDJSON or array) | per-row `{pid, name}` / error |
| GET | `/api/v1/plans/export` | — | full `[Plan]` snapshot (active rows) |

### Authentication

| Method | Path | Returns |
|---|---|---|
| GET | `/api/v1/plans/whoami` | verified PASETO-token `Claims`; `401` without a valid token |

Short-lived **PASETO v4.public** tokens are verified offline against the
auth-service's published **Ed25519 key** via the embedded
`authentication-verifier` (`src/auth.rs`). The `AuthUser` extractor
requires a token; `MaybeAuthUser` is optional and feeds the audit
`actor`. Blanket `/api/*` auth enforcement + published-key fetch are
follow-ups. (Front-ends use a BFF + cookie session, so the browser holds
no token; the BFF supplies this bearer server-side.) Source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(RS256/JWKS not used).

### Audit & events

| Method | Path | Returns |
|---|---|---|
| GET | `/api/v1/plans/audit/recent` | recent `audit_logs` rows (all plans, cap 100) |
| GET | `/api/v1/plans/{pid}/audit` | audit trail for one plan |
| GET | `/api/v1/plans/events/recent` | recent `PlanEvent`s from the in-memory stream |

Each create / update / delete (plan or sub-resource) writes a
best-effort `audit_logs` row (durable) and publishes a
`created`/`updated`/`deleted` event to the in-memory stream. Durable
broker is roadmap.

### API documentation

| Method | Path | Returns |
|---|---|---|
| GET | `/api-docs/openapi.json` | hand-written OpenAPI 3 document |
| GET | `/swagger-ui` | Swagger UI page (CDN assets) rendering the spec |

## HTTP status codes

| Code | Meaning |
|---|---|
| 200 | Success |
| 400 | Malformed body (loco JSON rejection); blank `q` on search |
| 404 | Unknown or soft-deleted `pid` / sub-resource id |
| 422 | Validation failure: blank `name` on plan create/update; blank sub-resource title (family convention) |
| 500 | Internal error |

## Example

```bash
curl -s localhost:5150/api/v1/plans \
  -H 'content-type: application/json' \
  -d '{"name":"Q3 Platform Initiative",
       "plan_type":"Initiative",
       "goals":[{"title":"Ship v2 API","status":"InProgress"}]}'
```

## Front-end consumption

**Files:**
[`src/lib/api/client.ts`](../plan-front-end-with-svelte/src/lib/api/client.ts)
(lean fetch wrapper + `ApiError`),
[`src/lib/api/plans.ts`](../plan-front-end-with-svelte/src/lib/api/plans.ts)
(`PlanRepository`: CRUD + sub-resource CRUD + `checkDuplicates` +
timeline / burndown reads).

| Route | Endpoints |
|---|---|
| `/` | `GET /api/v1/plans` |
| `/new` | `POST /api/v1/plans` |
| `/[pid]` | `GET`, `DELETE /api/v1/plans/{pid}`; `POST …/check-duplicates`; sub-resource lists |
| `/[pid]/edit` | `GET`, `PUT /api/v1/plans/{pid}` |
| `/[pid]/timeline` | `GET …/{pid}/timeline` |
| `/[pid]/burndown` | `GET …/{pid}/burndown` |

Base URL: `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).

## Matcher library API

```rust
use plan_matcher::{Plan, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_plans(&a, &b);
// result.score: f64 in [0.0, 1.0]
// result.confidence: High | Medium | Low
// result.is_match: bool
// result.breakdown: per-component Option<f64>
```

SemVer surface = `lib.rs` re-exports: `Plan`, `PlanIdentifier`,
`IdentifierScheme`, `PlanType`, `PlanStatus`, `Goal`, `GoalStatus`,
`Relationship`, `RelationKind`, `MatchingEngine`, `MatchConfig`,
`MatchResult`, `MatchBreakdown`, `Confidence`, `Error`, `Result`.
