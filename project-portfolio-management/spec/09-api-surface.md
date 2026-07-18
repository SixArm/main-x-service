## 9. API Surface

Endpoint detail: [`AGENTS/restful.md`](../AGENTS/restful.md); source
(planned): `src/controllers/`. All endpoints are **targets** — the
entity is spec-only (§14).

### 9.1 Service REST API — work items (thin record)

The four collections — `/api/portfolios`, `/api/projects`,
`/api/products`, `/api/programs` — expose the **identical** surface;
below, `{collection}` stands for any one of them, and all matching /
dedup / merge is **within that collection** (the kind gate, §5.5).
Child collections (`projects`, `products`, `programs`) additionally
accept a `?portfolio=<pid>` roll-up filter on list.

| Method | Path | Purpose | Returns |
|---|---|---|---|
| POST | `/api/{collection}` | Create (body: `WorkItem` whose `kind` matches the collection; blank `name` rejected; `409` with candidates on likely duplicate, `force` to bypass) | `{pid, name}` |
| GET | `/api/{collection}` | List active, most-recent first, capped 100 (child kinds: `?portfolio=<pid>` filter) | `[{pid, name}]` |
| GET | `/api/{collection}/search?q=` | Case-insensitive name search within the collection (Postgres `ILIKE`, cap 50); blank `q` → `400` | `[{pid, name}]` |
| GET | `/api/{collection}/{pid}` | Fetch the stored thin `WorkItem` | `WorkItem` |
| PUT | `/api/{collection}/{pid}` | Replace the thin payload | `{pid, name}` |
| DELETE | `/api/{collection}/{pid}` | Soft-delete (cascades to sub-resources on read paths) | empty JSON |
| POST | `/api/{collection}/match` | Rank `{query, candidates}` (no persistence; same-kind) | `[(index, MatchResult)]` |
| POST | `/api/{collection}/check-duplicates` | Match a query against stored work items of this kind | `[{pid, name, score, confidence, is_match}]` sorted by score |
| POST | `/api/{collection}/merge` | Merge a duplicate into a survivor of the same kind (`{main_pid, duplicate_pid, reason?}`; re-homes sub-resources); equal pids → `422`, unknown → `404` | `{main_pid, duplicate_pid, main: WorkItem}` |
| GET | `/api/{collection}/merges/recent` | Recent merge-history records, newest first | `[MergeRecord]` |
| GET | `/api/{collection}/whoami` | Echo verified bearer-token claims; `401` without a valid token | `Claims` |
| GET | `/api/{collection}/audit/recent` | Recent audit-log entries (this collection + its sub-resources), newest first, cap 100 | `[AuditLog]` |
| GET | `/api/{collection}/{pid}/audit` | Audit trail for one work item (incl. its sub-resources), newest first | `[AuditLog]` |
| GET | `/api/{collection}/events/recent` | Recent events from the in-memory stream | `[WorkItemEvent]` |
| GET | `/api-docs/openapi.json` | OpenAPI 3 document | `OpenAPI` JSON |
| GET | `/swagger-ui` | Swagger UI (CDN assets) | HTML |

Plus loco's built-in `/_health` and `/_ping`.

Every create / update / delete / merge (on a work item **and** every
sub-resource) writes a best-effort `audit_logs` row and publishes a
`WorkItemEvent` to the in-memory stream; merge additionally writes a
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
(blank `name`, `kind`/collection mismatch, malformed `EntityRef` /
`portfolio_ref` / deterministic identifier / relationship /
`in_language` — FR-1a); `400` for a malformed body; `409` for a
real-time duplicate on create (FR-11a).

### 9.2 Service REST API — operational sub-resources

Each sub-resource is nested under its work item, in any collection.
None enters the matcher payload (§5.6); all writes emit events + audit
rows.

| Method | Path | Purpose |
|---|---|---|
| GET/POST | `/api/{collection}/{pid}/goals` · PUT/DELETE `…/goals/{id}` | Goals — writes also mutate `data.goals[]` (§5.3) |
| GET/POST | `/api/{collection}/{pid}/tasks` · PUT/DELETE `…/tasks/{id}` | Tasks (board / list; status, assignee, estimate, due, nesting) |
| GET/POST | `/api/{collection}/{pid}/issues` · PUT/DELETE `…/issues/{id}` | Issues (kind / severity / status / assignee) |
| GET | `/api/{collection}/{pid}/timeline` | Derived Gantt projection (read-only) |
| GET | `/api/{collection}/{pid}/burndown` | Derived remaining-vs-estimate (read-only) |

### 9.3 Front-end routes

The front-end repeats the same route set under each collection;
`{collection}` is one of `portfolios` / `projects` / `products` /
`programs`.

| Route | Purpose | Endpoints consumed |
|---|---|---|
| `/{collection}` | List work items (child kinds may filter by portfolio) | `GET /api/{collection}` |
| `/{collection}/new` | Create form (`kind` fixed by collection) | `POST /api/{collection}` |
| `/{collection}/[pid]` | Detail + delete + check-duplicates + sub-resource tabs (portfolio detail rolls up its children) | `GET`, `DELETE /api/{collection}/{pid}`; `POST …/check-duplicates` |
| `/{collection}/[pid]/edit` | Edit form | `GET`, `PUT /api/{collection}/{pid}` |
| `/{collection}/[pid]/goals` · `/tasks` · `/issues` | Sub-resource workspaces | the §9.2 sub-resource endpoints |
| `/{collection}/[pid]/timeline` · `/[pid]/burndown` | Derived views | `GET …/timeline`, `GET …/burndown` |

### 9.4 Library API (matcher)

The matcher's SemVer surface is its `lib.rs` re-exports: `WorkItem`,
`WorkItemKind`, `Goal`, `WorkItemIdentifier`, `IdentifierScheme`,
`WorkItemStatus`, `GoalStatus`, `WorkItemRelationship`, `RelationKind`,
`MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`,
`Confidence`, `Error`, `Result`. Entry point:

```rust
let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_work_items(&a, &b);
// different kinds → score 0.0 (the kind gate, §5.5)
// also: engine.rank(&query, &candidates), engine.find_matches(…)
```

The sub-resource types (`Task`, `Issue`) are **not** in the matcher
crate — they have no match role and are owned by the service crate
(§5.6, §5.7).

### 9.5 Cross-service links (write-side)

Per [`agents/share/cross-service-linking.md` §4](../../agents/share/cross-service-linking.md),
the portfolio service mounts the write-side link surface so a work item
/ goal / task / issue can link to **any** index entity:

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/{collection}/{pid}/links` | Create / upsert an outbound edge (`{kind, to_ref, role?, valid_from?, valid_to?}`) |
| GET | `/api/{collection}/{pid}/links` | List this work item's outbound edges |
| DELETE | `/api/{collection}/{pid}/links/{id}` | Soft-delete (emits `unlinked`) |

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
**portfolio-specific** bits; the shared doc is the source of truth for
everything else.

The five endpoints (shared doc §4) mount under each work-item
collection:

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/{collection}/import` | `202 {job_id}` — body: `format`, `dedupe_mode`, `dry_run`; file upload |
| `GET` | `/api/{collection}/import/{id}` | Job status + counts + `errors_url` + `review_url` |
| `POST` | `/api/{collection}/export` | `202 {job_id}` — body: `format`, `filter`, `fields`, `include_soft_deleted`, `masking_profile` |
| `GET` | `/api/{collection}/export/{id}` | Job status + `download_url` |
| `GET` | `/api/{collection}/bulk-jobs` | List (filter by `kind`/`status`); `GET .../{id}` for one |

Each job is scoped to one collection (one `kind`); a row whose `kind`
disagrees with the collection is a per-row error (§7 of the shared
doc). **Stable key(s) for upsert** (shared doc §6, §10): a row upserts
in place when it carries either:

- a **deterministic scheme-scoped identifier** — the same
  `identifiers` the matcher short-circuits on (R-0, §6.3), keyed by
  `(scheme, value)`: `Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`,
  `TrelloBoardId`, `MsProjectId`, `GitHubProjectId`, `LinearId`
  (these are exactly the external-PM-tool ids a migration carries); or
- the **owner-scoped code** keyed by `(owner_org_id, code)` — never
  globally unique, so it upserts **only within the same
  `owner_org_id`** (the cross-owner-uniqueness invariant, §5.8); the
  matcher likewise never short-circuits across owners; or
- the record **`pid`** (the work-item UUID) when present in the row.

A row with neither runs the normal duplicate detection
(`check-duplicates` path, same-kind) routing likely duplicates to the
review queue with `provenance = import` (matching the
cross-service-linking provenance vocabulary). Bulk import covers the
**thin work-item record**; sub-resource bulk import is a roadmap
extension of the same job (§15).

**CSV column set + flattening** (shared doc §5). CSV is the operator /
spreadsheet format and is lossy for deep nesting — steer
fidelity-sensitive loads to **JSONL** (the lossless reference). Flat
columns:

- **scalar** (one column each): `pid`, `kind`, `name`, `code`,
  `owner_org_id`, `owner_org_name`, `lead_ref`, `portfolio_ref`,
  `status`, `start_date`, `target_date`, `in_language`, `active`;
- **arrays / arrays-of-objects** → a single **JSON-encoded cell** each:
  `alternate_names`, `goals` (`{title, description?, target_date?,
  status?}`), `keywords`, `tags`, `identifiers` (`{scheme, value}`),
  `same_as`, and `relationships` (`{relation, work_item_id}`).

There is no single-nested-object scalar field, so no dotted columns;
every repeated / nested field is a JSON-encoded cell. JSONL
round-trips the whole thin `WorkItem` payload (including `kind` and
`goals[]`) losslessly.

**Export sensitivity** (shared doc §8). Work items are **operational /
business data**; the **personal-data** angle is the `EntityRef`s to
people — `lead_ref`, task assignees (§12). Export defaults to
**masked** per the shared contract; full / unmasked output (which
reveals the people references) requires a `masking_profile` selecting
elevated authorisation and must never reveal more than the caller could
read one record at a time. `include_soft_deleted` defaults `false` and
is gated. **Every export is audited** (actor, collection, filter,
format, row count, masking profile, timestamp — written even for a
zero-row export).
