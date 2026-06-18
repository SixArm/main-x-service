## 10. Persistence

PostgreSQL via SeaORM 1.1 + `sea-orm-migration`, owned entirely by the
service crate. Family conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md). All
tables below are **planned** — the entity is spec-only (§14).

### 10.1 Tables

The thin matchable record lives in one `plans` table; the operational
sub-resources each get their own table keyed by the plan `pid` (the
partition, §5.6).

**`plans`** — the thin matchable record:

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `pid` | `UuidUniq` | Public id (route param) |
| `name` | `String` | Denormalised `data.name` for listing |
| `data` | `JsonBinary` (JSONB) | Full thin `Plan` payload (incl. `goals[]`) |
| `active` | `BooleanWithDefault(true)` | Registry flag |
| `deleted_at` | `TimestampWithTimeZoneNull` | Soft delete |

**Operational sub-resources** — one table each, every row carrying
`plan_pid UUID` (indexed) + `pid UUID unique` + `deleted_at` for soft
delete. None is serialised into `plans.data`; none reaches the matcher.

| Table | Key columns (beyond `id`, `pid`, `plan_pid`, `deleted_at`) |
|---|---|
| `tasks` | `title`, `description?`, `assignee_ref?`, `status`, `goal_id?`, `parent_task_id?`, `estimate?`, `remaining?`, `due_date?` |
| `issues` | `title`, `kind`, `severity`, `status`, `assignee_ref?` |
| `posts` | `author_ref`, `title`, `body_markdown` |
| `comments` | `target_kind` (post\|task\|issue), `target_id`, `author_ref`, `body_markdown` |
| `members` | `user_ref`, `role` (Owner\|Lead\|Member\|Viewer) — `UNIQUE (plan_pid, user_ref)` |
| `task_snapshots` | `task_pid`, `estimate`, `remaining`, `observed_at` — feeds the burndown derived view (§6.4) |

Goals are **not** a separate table: they live in `plans.data.goals[]`
(§5.3), exposed as a sub-resource via JSONB array mutation (§10.2).

**Family-baseline tables:**

- **`audit_logs`** — one row per CRUD action on the plan **or any
  sub-resource**: `entity_pid` (the plan), `sub_kind?` (which
  sub-resource, NULL for the plan itself), `action`
  (`created`/`updated`/`deleted`/`merged`), `actor` (token `sub`, NULL
  until auth enforced), `snapshot` (JSONB at the time).
- **`merge_records`** — one row per record-merge: `main_pid`,
  `duplicate_pid`, `reason?`, `actor?`, `transferred` (snapshot of the
  duplicate's payload, plus a tally of re-homed sub-resources).
- **`entity_links`** — the cross-service write-side table per
  [`agents/share/cross-service-linking.md` §4.1](../../agents/share/cross-service-linking.md):
  `from_pid`, `kind`, `to_ref` (`EntityRef` URN), `role?`,
  `confidence`, `provenance`, `valid_from?`, `valid_to?`, `deleted_at`,
  `UNIQUE (from_pid, kind, to_ref, valid_from)`. The canonical column
  list is in that doc and is **not** restated.
- **`bulk_jobs`** — the bulk import/export job table per
  [`agents/share/bulk-import-export.md` §3](../../agents/share/bulk-import-export.md),
  with `UNIQUE (entity, kind, idempotency_key)`. Canonical column list
  lives there; not restated (§10.4).

### 10.2 JSONB rationale and the goals bridge

The thin payload is stored verbatim so that the matcher type remains
the single schema for the matchable record. Adding a field to `Plan`
is a matcher-crate change plus a CHANGELOG entry — **no service
migration** — because serde fills missing fields with defaults on
read. The trade-off: querying inside the payload needs JSONB operators
(or GIN indexes, roadmap), and the only relational projections of the
thin record are `pid` and `name`.

`goals[]` is the **only** sub-resource inside the payload. Goal CRUD
endpoints (FR-11) read / mutate the `plans.data.goals[]` array
transactionally and rewrite `data`, so the matchable payload and the
goal sub-resource view never diverge. The other sub-resources are
ordinary relational tables (querying tasks by status, listing a
member's plans, etc. is relational), which is why they are *not* in the
JSONB — high-volume, frequently-queried, non-identity-bearing.

### 10.3 Operations

- `auto_migrate` is on in development (`cargo loco start` migrates).
- Soft delete is application-level (`deleted_at`); all queries filter
  `deleted_at IS NULL`. Soft-deleting a plan hides its sub-resources
  from read paths (§5.8); merge re-homes them to the survivor (§6.3).
- Open question OQ-3: index the high-traffic sub-resource columns
  (`tasks.status`, `tasks.assignee_ref`, `comments.(target_kind,
  target_id)`) and add JSONB GIN on `plans.data` once search /
  candidate blocking lands.

### 10.4 Bulk import / export — `bulk_jobs`

Async bulk operations (§9.6) add one table, `bulk_jobs`, per the
family-wide schema in
[bulk import/export §3](../../agents/share/bulk-import-export.md) — the
canonical column list lives there and is **not** restated. It tracks
each import/export job (`kind`, `format`, `status`, `params`, the
`rows_total`/`rows_created`/`rows_upserted`/`rows_to_review`/`rows_errored`
counts, `actor`, artifact URLs, and `expires_at` TTL), with
`UNIQUE (entity, kind, idempotency_key)` so a retried submit maps to
the same job. Jobs run on the loco `bg_pg` worker; artifacts (uploaded
source, export output, error report) live in the config-driven
artifact store (S3-compatible in deployment, local fs in dev),
referenced by short-lived access-controlled URLs.

Plan-specific notes:

- **Idempotency** is anchored on the §9.6 stable keys (a deterministic
  scheme-scoped external-PM identifier, the owner-scoped
  `(owner_org_id, plan_code)`, or `pid`): re-submitting a file
  re-upserts the same rows to the same state.
- The matcher's **single schema** holds for the thin record: the JSONB
  `data` column is the full thin `Plan` payload, so JSONL round-trips
  losslessly and CSV flattens per §9.6 (every repeated / nested field
  a JSON-encoded cell); no bulk path bypasses the JSONB round-trip
  invariant (§5.8). Bulk import targets the thin plan record;
  sub-resource bulk is a roadmap extension.
