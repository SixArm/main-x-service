# RESTful API Reference — Care Pathway Entity

The service is loco.rs and returns **raw JSON** — no
`{success, data, error}` envelope (unlike the pre-loco person
service). Source:
[`src/controllers/care_pathways.rs`](../care-pathway-service-with-loco/src/controllers/care_pathways.rs).
Base URL in development: `http://localhost:5150`.

## Endpoints

### Health (loco built-ins)

| Method | Path | Description |
|---|---|---|
| GET | `/_health` | Health check |
| GET | `/_ping` | Liveness ping |

### Care pathway CRUD

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/care-pathways` | `CarePathway` | `{pid, name}` |
| GET | `/api/care-pathways` | — | `[{pid, name}]` (active, most-recent first, cap 100) |
| GET | `/api/care-pathways/search?q=` | — | `[{pid, name}]` — `ILIKE` name match (cap 50); blank `q` → `400` |
| GET | `/api/care-pathways/{pid}` | — | stored `CarePathway` |
| PUT | `/api/care-pathways/{pid}` | `CarePathway` | `{pid, name}` |
| DELETE | `/api/care-pathways/{pid}` | — | `{}` (soft delete) |

### Matching

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/care-pathways/match` | `{query, candidates}` | ranked `[(index, MatchResult)]` |
| POST | `/api/care-pathways/check-duplicates` | `CarePathway` | `[{pid, name, score, confidence, is_match}]`, score-descending |
| POST | `/api/care-pathways/merge` | `{main_pid, duplicate_pid, reason?}` | `{main_pid, duplicate_pid, main}`; `422` equal pids, `404` unknown |
| GET | `/api/care-pathways/merges/recent` | — | recent `merge_records` (history + transferred snapshot) |

### Authentication

| Method | Path | Returns |
|---|---|---|
| GET | `/api/care-pathways/whoami` | verified bearer-token `Claims`; `401` without a valid token |

Short-lived **PASETO v4.public** tokens are verified offline against the
auth-service's published Ed25519 key via the embedded
`authentication-verifier` (`src/auth.rs`) — RS256 JWT + JWKS
decommissioned. The `AuthUser` extractor requires a token; `MaybeAuthUser`
is optional and feeds the audit `actor`. Blanket `/api/*` enforcement +
published-key fetch are follow-ups. Auth pivot in progress — source of
truth
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md);
code follow-up tracked in entity spec §13.

### Audit & events

| Method | Path | Returns |
|---|---|---|
| GET | `/api/care-pathways/audit/recent` | recent `audit_logs` rows (all pathways, cap 100) |
| GET | `/api/care-pathways/{pid}/audit` | audit trail for one pathway |
| GET | `/api/care-pathways/events/recent` | recent `PathwayEvent`s from the in-memory stream |

Each create / update / delete writes a best-effort `audit_logs` row
(durable) and publishes a `created`/`updated`/`deleted` event to the
in-memory stream. Durable broker is roadmap.

### API documentation

| Method | Path | Returns |
|---|---|---|
| GET | `/api-docs/openapi.json` | hand-written OpenAPI 3 document |
| GET | `/swagger-ui` | Swagger UI page (CDN assets) rendering the spec |

## HTTP status codes

| Code | Meaning |
|---|---|
| 200 | Success |
| 400 | Malformed body (loco JSON rejection) |
| 404 | Unknown or soft-deleted `pid` |
| 422 | Validation failure: blank `name` on create or update (family convention; OQ-1 resolved via T-2) |
| 500 | Internal error |

Auth via the central auth-service: offline **PASETO v4.public**
verification against the published Ed25519 key (RS256 JWT + JWKS
decommissioned). See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); code follow-up tracked in entity spec §13.

## Example

```bash
curl -s localhost:5150/api/care-pathways \
  -H 'content-type: application/json' \
  -d '{"name":"Acute Stroke Care Pathway",
       "condition_codes":[{"system":"Icd10","code":"I63"}]}'
```

## Front-end consumption

**Files:**
[`src/lib/api/client.ts`](../care-pathway-front-end-with-svelte/src/lib/api/client.ts)
(lean fetch wrapper + `ApiError`),
[`src/lib/api/care-pathways.ts`](../care-pathway-front-end-with-svelte/src/lib/api/care-pathways.ts)
(`CarePathwayRepository`: CRUD + `checkDuplicates`).

| Route | Endpoints |
|---|---|
| `/` | `GET /api/care-pathways` |
| `/new` | `POST /api/care-pathways` |
| `/[pid]` | `GET`, `DELETE /api/care-pathways/{pid}`; `POST …/check-duplicates` |
| `/[pid]/edit` | `GET`, `PUT /api/care-pathways/{pid}` |

Base URL: `PUBLIC_API_BASE_URL` (default `http://localhost:5150`).

## Matcher library API

```rust
use care_pathway_matcher::{CarePathway, MatchConfig, MatchingEngine};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_care_pathways(&a, &b);
// result.score: f64 in [0.0, 1.0]
// result.confidence: High | Medium | Low
// result.is_match: bool
// result.breakdown: per-component Option<f64>
```

SemVer surface = `lib.rs` re-exports: `CarePathway`,
`PathwayIdentifier`, `IdentifierScheme`, `ConditionCode`,
`CodeSystem`, `CareSetting`, `MatchingEngine`, `MatchConfig`,
`MatchResult`, `MatchBreakdown`, `Confidence`, `Error`, `Result`.
