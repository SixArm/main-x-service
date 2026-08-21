## 9. API Surface

Endpoint detail: [`agents/restful.md`](../agents/restful.md); source:
[`src/controllers/cases.rs`](../case-service-with-loco/src/controllers/cases.rs).

### 9.1 Service REST API

| Method | Path | Purpose | Returns |
|---|---|---|---|
| POST | `/api/cases` | Create (body: `Case`; blank `title` rejected) | `{pid, title}` |
| GET | `/api/cases` | List active, most-recent first, capped 100 | `[{pid, title}]` |
| GET | `/api/cases/search?q=` | Case-insensitive title search (Postgres `ILIKE`, cap 50); blank `q` → `400` | `[{pid, title}]` |
| GET | `/api/cases/{pid}` | Fetch the stored `Case` | `Case` |
| PUT | `/api/cases/{pid}` | Replace the payload | `{pid, title}` |
| DELETE | `/api/cases/{pid}` | Soft-delete | empty JSON |
| POST | `/api/cases/match` | Rank `{query, candidates}` (no persistence) | `[(index, MatchResult)]` |
| POST | `/api/cases/check-duplicates` | Match a query against stored cases | `[{pid, title, score, confidence, is_match}]` sorted by score |
| POST | `/api/cases/merge` | Merge a duplicate into a survivor (`{main_pid, duplicate_pid, reason?}`); equal pids → `422`, unknown → `404` | `{main_pid, duplicate_pid, main: Case}` |
| GET | `/api/cases/merges/recent` | Recent merge-history records, newest first | `[MergeRecord]` |
| GET | `/api/cases/whoami` | Echo verified bearer-token claims; `401` without a valid token | `Claims` |
| GET | `/api/cases/audit/recent` | Recent audit-log entries (all cases), newest first, cap 100 | `[AuditLog]` |
| GET | `/api/cases/{pid}/audit` | Audit trail for one case, newest first | `[AuditLog]` |
| GET | `/api/cases/events/recent` | Recent CRUD/merge events from the in-memory stream | `[CaseEvent]` |
| GET | `/api-docs/openapi.json` | OpenAPI 3 document for the API | `OpenAPI` JSON |
| GET | `/swagger-ui` | Swagger UI rendering the spec (CDN assets) | HTML |

Plus loco's built-in `/_health` and `/_ping`.

Every create / update / delete / merge writes a best-effort `audit_logs`
row (action + JSON snapshot, durable in Postgres) and publishes a
`CaseEvent` (`created`/`updated`/`deleted`/`merged`) to the in-memory
event stream. Merge additionally writes a `merge_records` history row
with a snapshot of the transferred (duplicate) payload. A durable
broker is roadmap (§15).

Bearer-token verification (PASETO v4 public, Ed25519, against the
auth-service's published key, offline — tokens ride in `Authorization:
Bearer v4.public.…`) is available via the `AuthUser` extractor; `whoami`
is protected by it, and create/update/delete/merge stamp the audit
`actor` from the token when one is present (`MaybeAuthUser`). Blanket
`/api/*` enforcement and paseto-keys-over-HTTP fetch are follow-ups
(§13 T-7). Auth model source of truth:
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
(supersedes the RS256-JWT + JWKS model).

Conventions: **raw loco JSON** (no `{success, data, error}` envelope —
this is the loco-era convention, unlike the pre-loco person service).
`404` for unknown / soft-deleted `pid`; `422` for a validation failure
(blank `title`, malformed `opened_date`, blank identifier value, blank
subject/keyword — family convention); `400` for a malformed body or a
blank search `q`.

### 9.2 Front-end routes

| Route | Purpose | Endpoints consumed |
|---|---|---|
| `/` | List cases | `GET /api/cases` |
| `/new` | Create form | `POST /api/cases` |
| `/[pid]` | Detail + delete + check-duplicates | `GET`, `DELETE /api/cases/{pid}`; `POST …/check-duplicates` |
| `/[pid]/edit` | Edit form | `GET`, `PUT /api/cases/{pid}` |

A front-end search box and audit / event views are roadmap (§13 T-11);
the service `/search`, `/audit/*`, and `/events/recent` endpoints exist
ahead of the UI.

### 9.3 Library API (matcher)

The matcher's SemVer surface is its `lib.rs` re-exports: `Case`,
`CaseIdentifier`, `IdentifierScheme`, `CaseType`, `CaseStatus`,
`Priority`, `MatchingEngine`, `MatchConfig`, `MatchResult`,
`MatchBreakdown`, `Confidence`, `Error`, `Result`. Entry point:

```rust
let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_cases(&a, &b);
// also: engine.rank(&query, &candidates), engine.find_matches(…)
```
