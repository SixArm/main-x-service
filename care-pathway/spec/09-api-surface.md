## 9. API Surface

Endpoint detail: [`AGENTS/restful.md`](../AGENTS/restful.md); source:
[`src/controllers/care_pathways.rs`](../care-pathway-service-rust-crate/src/controllers/care_pathways.rs).

### 9.1 Service REST API

| Method | Path | Purpose | Returns |
|---|---|---|---|
| POST | `/api/care-pathways` | Create (body: `CarePathway`; blank `name` rejected) | `{pid, name}` |
| GET | `/api/care-pathways` | List active, most-recent first, capped 100 | `[{pid, name}]` |
| GET | `/api/care-pathways/{pid}` | Fetch the stored `CarePathway` | `CarePathway` |
| PUT | `/api/care-pathways/{pid}` | Replace the payload | `{pid, name}` |
| DELETE | `/api/care-pathways/{pid}` | Soft-delete | empty JSON |
| POST | `/api/care-pathways/match` | Rank `{query, candidates}` (no persistence) | `[(index, MatchResult)]` |
| POST | `/api/care-pathways/check-duplicates` | Match a query against stored pathways | `[{pid, name, score, confidence, is_match}]` sorted by score |
| GET | `/api-docs/openapi.json` | OpenAPI 3 document for the API | `OpenAPI` JSON |
| GET | `/swagger-ui` | Swagger UI rendering the spec (CDN assets) | HTML |

Plus loco's built-in `/_health` and `/_ping`.

Conventions: **raw loco JSON** (no `{success, data, error}` envelope
— this is the loco-era convention, unlike the pre-loco person
service). `404` for unknown / soft-deleted `pid`; `422` for a
validation failure (blank `name` on create/update — family
convention, OQ-1 resolved via T-2); `400` for a malformed body. No
authentication yet (roadmap §15).

### 9.2 Front-end routes

| Route | Purpose | Endpoints consumed |
|---|---|---|
| `/` | List care pathways | `GET /api/care-pathways` |
| `/new` | Create form | `POST /api/care-pathways` |
| `/[pid]` | Detail + delete + check-duplicates | `GET`, `DELETE /api/care-pathways/{pid}`; `POST …/check-duplicates` |
| `/[pid]/edit` | Edit form | `GET`, `PUT /api/care-pathways/{pid}` |

### 9.3 Library API (matcher)

The matcher's SemVer surface is its `lib.rs` re-exports:
`CarePathway`, `PathwayIdentifier`, `IdentifierScheme`,
`ConditionCode`, `CodeSystem`, `CareSetting`, `MatchingEngine`,
`MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`,
`Error`, `Result`. Entry point:

```rust
let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_care_pathways(&a, &b);
// also: engine.rank(&query, &candidates), engine.find_matches(…)
```
