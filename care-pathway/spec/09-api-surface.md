## 9. API Surface

Endpoint detail: [`AGENTS/restful.md`](../AGENTS/restful.md); source:
[`src/controllers/care_pathways.rs`](../care-pathway-service-rust-crate/src/controllers/care_pathways.rs).

### 9.1 Service REST API

| Method | Path | Purpose | Returns |
|---|---|---|---|
| POST | `/api/care-pathways` | Create (body: `CarePathway`; blank `name` rejected) | `{pid, name}` |
| GET | `/api/care-pathways` | List active, most-recent first, capped 100 | `[{pid, name}]` |
| GET | `/api/care-pathways/search?q=` | Case-insensitive name search (Postgres `ILIKE`, cap 50); blank `q` → `400` | `[{pid, name}]` |
| GET | `/api/care-pathways/{pid}` | Fetch the stored `CarePathway` | `CarePathway` |
| PUT | `/api/care-pathways/{pid}` | Replace the payload | `{pid, name}` |
| DELETE | `/api/care-pathways/{pid}` | Soft-delete | empty JSON |
| POST | `/api/care-pathways/match` | Rank `{query, candidates}` (no persistence) | `[(index, MatchResult)]` |
| POST | `/api/care-pathways/check-duplicates` | Match a query against stored pathways | `[{pid, name, score, confidence, is_match}]` sorted by score |
| POST | `/api/care-pathways/merge` | Merge a duplicate into a survivor (`{main_pid, duplicate_pid, reason?}`); equal pids → `422`, unknown → `404` | `{main_pid, duplicate_pid, main: CarePathway}` |
| GET | `/api/care-pathways/merges/recent` | Recent merge-history records, newest first | `[MergeRecord]` |
| GET | `/api/care-pathways/whoami` | Echo verified bearer-token claims; `401` without a valid token | `Claims` |
| GET | `/api/care-pathways/audit/recent` | Recent audit-log entries (all pathways), newest first, cap 100 | `[AuditLog]` |
| GET | `/api/care-pathways/{pid}/audit` | Audit trail for one pathway, newest first | `[AuditLog]` |
| GET | `/api/care-pathways/events/recent` | Recent CRUD events from the in-memory stream | `[PathwayEvent]` |
| GET | `/api-docs/openapi.json` | OpenAPI 3 document for the API | `OpenAPI` JSON |
| GET | `/swagger-ui` | Swagger UI rendering the spec (CDN assets) | HTML |

Plus loco's built-in `/_health` and `/_ping`.

Every create / update / delete / merge writes a best-effort `audit_logs`
row (action + JSON snapshot, durable in Postgres) and publishes a
`PathwayEvent` (`created`/`updated`/`deleted`/`merged`) to the in-memory
event stream. Merge additionally writes a `merge_records` history row
with a snapshot of the transferred (duplicate) payload. A durable broker
is roadmap (§15).

Bearer-token verification (RS256 against the auth-service JWKS, offline)
is available via the `AuthUser` extractor; `whoami` is protected by it,
and create/update/delete stamp the audit `actor` from the token when
one is present (`MaybeAuthUser`). Blanket `/api/*` enforcement and
JWKS-over-HTTP fetch are follow-ups (§13 T-7).

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
