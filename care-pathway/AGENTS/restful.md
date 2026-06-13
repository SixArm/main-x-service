# RESTful API Reference — Care Pathway Entity

The service is loco.rs and returns **raw JSON** — no
`{success, data, error}` envelope (unlike the pre-loco person
service). Source:
[`src/controllers/care_pathways.rs`](../care-pathway-service-rust-crate/src/controllers/care_pathways.rs).
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
| GET | `/api/care-pathways/{pid}` | — | stored `CarePathway` |
| PUT | `/api/care-pathways/{pid}` | `CarePathway` | `{pid, name}` |
| DELETE | `/api/care-pathways/{pid}` | — | `{}` (soft delete) |

### Matching

| Method | Path | Body | Returns |
|---|---|---|---|
| POST | `/api/care-pathways/match` | `{query, candidates}` | ranked `[(index, MatchResult)]` |
| POST | `/api/care-pathways/check-duplicates` | `CarePathway` | `[{pid, name, score, confidence, is_match}]`, score-descending |

## HTTP status codes

| Code | Meaning |
|---|---|
| 200 | Success |
| 400 | Malformed body (loco JSON rejection) |
| 404 | Unknown or soft-deleted `pid` |
| 422 | Validation failure: blank `name` on create or update (family convention; OQ-1 resolved via T-2) |
| 500 | Internal error |

No authentication yet — JWT verification against the central
auth-service JWKS is roadmap (entity spec §15).

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
