## 9. API Surface

Endpoint detail: [`AGENTS/restful.md`](../AGENTS/restful.md); source
(planned): `src/controllers/`. All endpoints are **targets** — the
entity is spec-only (§14).

### 9.1 Service REST API — plans (thin record)

| Method | Path | Purpose | Returns |
|---|---|---|---|
| POST | `/api/plans` | Create (body: `Plan`; blank `name` rejected; `409` with candidates on likely duplicate, `force` to bypass) | `{pid, name}` |
| GET | `/api/plans` | List active, most-recent first, capped 100 | `[{pid, name}]` |
| GET | `/api/plans/search?q=` | Case-insensitive name search (Postgres `ILIKE`, cap 50); blank `q` → `400` | `[{pid, name}]` |
| GET | `/api/plans/{pid}` | Fetch the stored thin `Plan` | `Plan` |
| PUT | `/api/plans/{pid}` | Replace the thin payload | `{pid, name}` |
| DELETE | `/api/plans/{pid}` | Soft-delete (cascades to sub-resources on read paths) | empty JSON |
| POST | `/api/plans/match` | Rank `{query, candidates}` (no persistence) | `[(index, MatchResult)]` |
| POST | `/api/plans/check-duplicates` | Match a query against stored plans | `[{pid, name, score, confidence, is_match}]` sorted by score |
| POST | `/api/plans/merge` | Merge a duplicate into a survivor (`{main_pid, duplicate_pid, reason?}`; re-homes sub-resources); equal pids → `422`, unknown → `404` | `{main_pid, duplicate_pid, main: Plan}` |
| GET | `/api/plans/merges/recent` | Recent merge-history records, newest first | `[MergeRecord]` |
| GET | `/api/plans/whoami` | Echo verified bearer-token claims; `401` without a valid token | `Claims` |
| GET | `/api/plans/audit/recent` | Recent audit-log entries (all plans + sub-resources), newest first, cap 100 | `[AuditLog]` |
| GET | `/api/plans/{pid}/audit` | Audit trail for one plan (incl. its sub-resources), newest first | `[AuditLog]` |
| GET | `/api/plans/events/recent` | Recent events from the in-memory stream | `[PlanEvent]` |
| GET | `/api-docs/openapi.json` | OpenAPI 3 document | `OpenAPI` JSON |
| GET | `/swagger-ui` | Swagger UI (CDN assets) | HTML |

Plus loco's built-in `/_health` and `/_ping`.

Every create / update / delete / merge (on the plan **and** every
sub-resource) writes a best-effort `audit_logs` row and publishes a
`PlanEvent` to the in-memory stream; merge additionally writes a
`merge_records` row with a snapshot of the transferred payload. A
durable broker is roadmap (§15). Bearer-token verification (a PASETO v4
public token, verified offline against the auth-service's published
Ed25519 key;
[`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
supersedes the RS256-JWT model) is via the `AuthUser` / `MaybeAuthUser`
extractors; `whoami` is protected; writes stamp the audit `actor` from
the token when present. Blanket `/api/*` enforcement + paseto-keys-over-HTTP
fetch are follow-ups (§13).

Conventions: **raw loco JSON** (no `{success, data, error}` envelope).
`404` for unknown / soft-deleted `pid`; `422` for a validation failure
(blank `name`, malformed `EntityRef` / deterministic identifier /
relationship / `in_language` — FR-1a); `400` for a malformed body;
`409` for a real-time duplicate on create (FR-10a).

### 9.2 Service REST API — operational sub-resources

Each sub-resource is nested under its plan. None enters the matcher
payload (§5.6); all writes emit events + audit rows.

| Method | Path | Purpose |
|---|---|---|
| GET/POST | `/api/plans/{pid}/goals` · PUT/DELETE `…/goals/{id}` | Goals — writes also mutate `data.goals[]` (§5.3) |
| GET/POST | `/api/plans/{pid}/tasks` · PUT/DELETE `…/tasks/{id}` | Tasks (board / list; status, assignee, estimate, due, nesting) |
| GET/POST | `/api/plans/{pid}/issues` · PUT/DELETE `…/issues/{id}` | Issues (kind / severity / status / assignee) |
| GET/POST | `/api/plans/{pid}/posts` · PUT/DELETE `…/posts/{id}` | Posts (Markdown updates) |
| GET/POST | `/api/plans/{pid}/comments` · PUT/DELETE `…/comments/{id}` | Comments on a post / task / issue |
| GET/POST | `/api/plans/{pid}/members` · PUT/DELETE `…/members/{id}` | Membership + role |
| GET | `/api/plans/{pid}/timeline` | Derived Gantt projection (read-only) |
| GET | `/api/plans/{pid}/burndown` | Derived remaining-vs-estimate (read-only) |

### 9.3 Front-end routes

| Route | Purpose | Endpoints consumed |
|---|---|---|
| `/` | List plans | `GET /api/plans` |
| `/new` | Create form | `POST /api/plans` |
| `/[pid]` | Detail + delete + check-duplicates + sub-resource tabs | `GET`, `DELETE /api/plans/{pid}`; `POST …/check-duplicates` |
| `/[pid]/edit` | Edit form | `GET`, `PUT /api/plans/{pid}` |
| `/[pid]/goals` · `/tasks` · `/issues` · `/posts` · `/members` | Sub-resource workspaces | the §9.2 sub-resource endpoints |
| `/[pid]/timeline` · `/[pid]/burndown` | Derived views | `GET …/timeline`, `GET …/burndown` |

### 9.4 Library API (matcher)

The matcher's SemVer surface is its `lib.rs` re-exports: `Plan`,
`Goal`, `PlanIdentifier`, `IdentifierScheme`, `PlanType`, `PlanStatus`,
`GoalStatus`, `PlanRelationship`, `RelationKind`, `MatchingEngine`,
`MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`, `Error`,
`Result`. Entry point:

```rust
let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_plans(&a, &b);
// also: engine.rank(&query, &candidates), engine.find_matches(…)
```

The sub-resource types (`Task`, `Issue`, `Post`, `Comment`, `Member`)
are **not** in the matcher crate — they have no match role and are
owned by the service crate (§5.6, §5.7).

### 9.5 Cross-service links (write-side)

Per [`agents/share/cross-service-linking.md` §4](../../agents/share/cross-service-linking.md),
the plan service mounts the write-side link surface so a plan / goal /
task / issue can link to **any** index entity:

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/v1/plans/{pid}/links` | Create / upsert an outbound edge (`{kind, to_ref, role?, valid_from?, valid_to?}`) |
| GET | `/api/v1/plans/{pid}/links` | List this plan's outbound edges |
| DELETE | `/api/v1/plans/{pid}/links/{id}` | Soft-delete (emits `unlinked`) |

Writes are **optimistic** (no call to the target service) and emit
`linked` / `unlinked` events on the bus. Links are **never** a match
signal (§7 there). The aggregator and graph-query API are out of scope
(§2.3).

### 9.6 Bulk import / export

The async, job-based bulk contract is fixed family-wide in
[bulk import/export](../../agents/share/bulk-import-export.md)
(execution model on `bg_pg`, the five endpoints, JSONL/CSV/Parquet
codecs, upsert-by-stable-key + dedupe-to-review, the per-row error
report, and export masking + audit). This section declares only the
**plan-specific** bits; the shared doc is the source of truth for
everything else.

The five endpoints (shared doc §4) mount under the plan resource:

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/v1/plans/import` | `202 {job_id}` — body: `format`, `dedupe_mode`, `dry_run`; file upload |
| `GET` | `/api/v1/plans/import/{id}` | Job status + counts + `errors_url` + `review_url` |
| `POST` | `/api/v1/plans/export` | `202 {job_id}` — body: `format`, `filter`, `fields`, `include_soft_deleted`, `masking_profile` |
| `GET` | `/api/v1/plans/export/{id}` | Job status + `download_url` |
| `GET` | `/api/v1/plans/bulk-jobs` | List (filter by `kind`/`status`); `GET .../{id}` for one |

**Stable key(s) for upsert** (shared doc §6, §10). A row upserts in
place when it carries either:

- a **deterministic scheme-scoped identifier** — the same
  `identifiers` the matcher short-circuits on (R-0, §6.6), keyed by
  `(scheme, value)`: `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`,
  `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId`
  (these are exactly the external-PM-tool ids a migration carries); or
- the **owner-scoped plan code** keyed by `(owner_org_id, plan_code)` —
  never globally unique, so it upserts **only within the same
  `owner_org_id`** (the cross-owner-uniqueness invariant, §5.8); the
  matcher likewise never short-circuits across owners; or
- the record **`pid`** (the plan UUID) when present in the row.

A row with neither runs the normal duplicate detection
(`check-duplicates` path), routing likely duplicates to the review
queue with `provenance = import` (matching the cross-service-linking
provenance vocabulary). Bulk import covers the **thin plan record**;
sub-resource bulk import is a roadmap extension of the same job (§15).

**CSV column set + flattening** (shared doc §5). CSV is the operator /
spreadsheet format and is lossy for deep nesting — steer
fidelity-sensitive loads to **JSONL** (the lossless reference). Flat
columns:

- **scalar** (one column each): `pid`, `name`, `plan_type`,
  `plan_code`, `owner_org_id`, `owner_org_name`, `lead_ref`, `status`,
  `start_date`, `target_date`, `in_language`, `active`;
- **arrays / arrays-of-objects** → a single **JSON-encoded cell** each:
  `alternate_names`, `goals` (`{title, description?, target_date?,
  status?}`), `keywords`, `tags`, `identifiers` (`{scheme, value}`),
  `same_as`, and `relationships` (`{relation, plan_id}`).

There is no single-nested-object scalar field, so no dotted columns;
every repeated / nested field is a JSON-encoded cell. JSONL
round-trips the whole thin `Plan` payload (including `goals[]`)
losslessly.

**Export sensitivity** (shared doc §8). Plans are **operational /
business data**; the **personal-data** angle is the `EntityRef`s to
people — `lead_ref`, members, task assignees, post / comment authors
(§12). Export defaults to **masked** per the shared contract; full /
unmasked output (which reveals the people references) requires a
`masking_profile` selecting elevated authorisation and must never
reveal more than the caller could read one record at a time.
`include_soft_deleted` defaults `false` and is gated. **Every export is
audited** (actor, filter, format, row count, masking profile,
timestamp — written even for a zero-row export).
