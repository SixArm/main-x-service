# RESTful API Reference — Case Entity

The service is loco.rs and returns **raw JSON** — no
`{success, data, error}` envelope (unlike the pre-loco person service).
Source:
[`src/controllers/cases.rs`](../case-service-rust-crate/src/controllers/cases.rs).
Base URL in development: `http://localhost:5150`.

## Endpoints

### Health (loco built-ins)

| Method | Path | Description |
|---|---|---|
| GET | `/_health` | Health check |
| GET | `/_ping` | Liveness ping |

### Case CRUD

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/cases` | `Case` | `{pid, title}` |
| GET | `/api/cases` | — | `[{pid, title}]` (active, most-recent first, cap 100) |
| GET | `/api/cases/search?q=` | — | `[{pid, title}]` — `ILIKE` title match (cap 50); blank `q` → `400` |
| GET | `/api/cases/{pid}` | — | stored `Case` |
| PUT | `/api/cases/{pid}` | `Case` | `{pid, title}` |
| DELETE | `/api/cases/{pid}` | — | `{}` (soft delete) |

### Matching

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/cases/match` | `{query, candidates}` | ranked `[(index, MatchResult)]` |
| POST | `/api/cases/check-duplicates` | `Case` | `[{pid, title, score, confidence, is_match}]`, score-descending |
| POST | `/api/cases/merge` | `{main_pid, duplicate_pid, reason?}` | `{main_pid, duplicate_pid, main}`; `422` equal pids, `404` unknown |
| GET | `/api/cases/merges/recent` | — | recent `merge_records` (history + transferred snapshot) |

### Authentication

| Method | Path | Returns |
|---|---|---|
| GET | `/api/cases/whoami` | verified bearer-token `Claims`; `401` without a valid token |

RS256 tokens are verified offline against the auth-service JWKS via the
embedded `authentication-verifier` (`src/auth.rs`), built from `CASE_JWKS`
/ `CASE_JWT_ISSUER` / `CASE_JWT_AUDIENCE`. The `AuthUser` extractor
requires a token; `MaybeAuthUser` is optional and feeds the audit / merge
`actor`. Blanket `/api/*` enforcement + JWKS-over-HTTP fetch are
follow-ups.

### Audit & events

| Method | Path | Returns |
|---|---|---|
| GET | `/api/cases/audit/recent` | recent `audit_logs` rows (all cases, cap 100) |
| GET | `/api/cases/{pid}/audit` | audit trail for one case |
| GET | `/api/cases/events/recent` | recent `CaseEvent`s from the in-memory stream |

Each create / update / delete / merge writes a best-effort `audit_logs`
row (durable) and publishes a `created`/`updated`/`deleted`/`merged`
event to the in-memory stream. Durable broker is roadmap.

### API documentation

| Method | Path | Returns |
|---|---|---|
| GET | `/api-docs/openapi.json` | hand-written OpenAPI 3 document |
| GET | `/swagger-ui` | Swagger UI page (CDN assets) rendering the spec |

## HTTP status codes

| Code | Meaning |
|---|---|
| 200 | Success |
| 400 | Malformed body (loco JSON rejection) or blank search `q` |
| 401 | Missing / invalid bearer token on a protected route (`whoami`) |
| 404 | Unknown or soft-deleted `pid` |
| 422 | Validation failure: blank `title`, malformed `opened_date`, blank identifier value, blank subject/keyword (family convention) |
| 500 | Internal error |

## Example

```bash
curl -s localhost:5150/api/cases \
  -H 'content-type: application/json' \
  -d '{"title":"Housing benefit claim — 14 Elm Street",
       "case_type":"Benefit","status":"Open",
       "agency_id":"dwp","case_number":"BEN-2026-00417",
       "subjects":["person:9f3c…"]}'
```

## Front-end consumption

**Files:**
[`src/lib/api/client.ts`](../case-front-end-with-svelte/src/lib/api/client.ts)
(lean fetch wrapper + `ApiError`),
[`src/lib/api/cases.ts`](../case-front-end-with-svelte/src/lib/api/cases.ts)
(`CaseRepository`: CRUD + `checkDuplicates`).

| Route | Endpoints |
|---|---|
| `/` | `GET /api/cases` |
| `/new` | `POST /api/cases` |
| `/[pid]` | `GET`, `DELETE /api/cases/{pid}`; `POST …/check-duplicates` |
| `/[pid]/edit` | `GET`, `PUT /api/cases/{pid}` |

A search box and audit / event views are roadmap (entity spec §13 T-11).
Base URL: `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).

## Matcher library API

```rust
use case_matcher::{Case, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_cases(&a, &b);
// result.score: f64 in [0.0, 1.0]
// result.confidence: High | Medium | Low
// result.is_match: bool
// result.breakdown: per-component Option<f64>
```

SemVer surface = `lib.rs` re-exports: `Case`, `CaseIdentifier`,
`IdentifierScheme`, `CaseType`, `CaseStatus`, `Priority`,
`MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`,
`Confidence`, `Error`, `Result`.
