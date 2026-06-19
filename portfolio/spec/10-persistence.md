## 10. Persistence

PostgreSQL via SeaORM 1.1 + `sea-orm-migration`, owned entirely by the
service crate. Family conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md). All
tables below are **planned** — the entity is spec-only (§14).

### 10.1 Tables

The thin matchable record lives in **one table per kind** —
`portfolios`, `projects`, `products`, `programs` — each with the same
shape; the operational sub-resources each get their own table keyed by
`(parent_kind, parent_pid)` (the partition, §5.6).

**`portfolios` / `projects` / `products` / `programs`** — the thin
matchable record (one table per `WorkItemKind`, identical columns):

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `pid` | `UuidUniq` | Public id (route param) |
| `name` | `String` | Denormalised `data.name` for listing |
| `data` | `JsonBinary` (JSONB) | Full thin `WorkItem` payload (incl. `kind`, `goals[]`) |
| `portfolio_pid` | `UuidNull` | **Child kinds only** — denormalised `data.portfolio_ref` for cheap roll-up of a portfolio's children (indexed); absent on `portfolios` |
| `active` | `BooleanWithDefault(true)` | Registry flag |
| `deleted_at` | `TimestampWithTimeZoneNull` | Soft delete |

The `data.kind` of every row in a table matches that table's kind (a
project row carries `kind = Project`); the controller pins it on create
(FR-1a). Matching only ever compares rows of one table, and the
matcher's kind gate (§5.5) is the defence-in-depth backstop.

**Operational sub-resources** — one table each, every row carrying
`parent_kind` (`Portfolio`\|`Project`\|`Product`\|`Program`) +
`parent_pid UUID` (indexed together) + `pid UUID unique` + `deleted_at`
for soft delete. None is serialised into any `data` column; none
reaches the matcher.

| Table | Key columns (beyond `id`, `pid`, `parent_kind`, `parent_pid`, `deleted_at`) |
|---|---|
| `tasks` | `title`, `description?`, `assignee_ref?`, `status`, `goal_id?`, `parent_task_id?`, `estimate?`, `remaining?`, `due_date?` |
| `issues` | `title`, `kind`, `severity`, `status`, `assignee_ref?` |
| `task_snapshots` | `task_pid`, `estimate`, `remaining`, `observed_at` — feeds the burndown derived view (§6.4) |

Goals are **not** a separate table: they live in `data.goals[]` on the
parent work item's row (§5.3), exposed as a sub-resource via JSONB
array mutation (§10.2).

**Family-baseline tables:**

- **`audit_logs`** — one row per CRUD action on a work item **or any
  sub-resource**: `entity_kind` (which collection), `entity_pid` (the
  work item), `sub_kind?` (which sub-resource, NULL for the work item
  itself), `action` (`created`/`updated`/`deleted`/`merged`), `actor`
  (token `sub`, NULL until auth enforced), `snapshot` (JSONB at the
  time).
- **`merge_records`** — one row per record-merge: `entity_kind`,
  `main_pid`, `duplicate_pid`, `reason?`, `actor?`, `transferred`
  (snapshot of the duplicate's payload, plus a tally of re-homed
  sub-resources).
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
the single schema for the matchable record across all four collections.
Adding a field to `WorkItem` is a matcher-crate change plus a CHANGELOG
entry — **no service migration** — because serde fills missing fields
with defaults on read. The trade-off: querying inside the payload needs
JSONB operators (or GIN indexes, roadmap), and the only relational
projections of the thin record are `pid`, `name`, and (child kinds)
`portfolio_pid`.

`goals[]` is the **only** sub-resource inside the payload. Goal CRUD
endpoints (FR-12) read / mutate the parent row's `data.goals[]` array
transactionally and rewrite `data`, so the matchable payload and the
goal sub-resource view never diverge. The other sub-resources are
ordinary relational tables (querying tasks by status, listing issues by
severity, etc. is relational), which is why they are *not* in the JSONB
— high-volume, frequently-queried, non-identity-bearing.

### 10.3 Operations

- `auto_migrate` is on in development (`cargo loco start` migrates).
- Soft delete is application-level (`deleted_at`); all queries filter
  `deleted_at IS NULL`. Soft-deleting a work item hides its
  sub-resources from read paths (§5.8); merge re-homes them to the
  survivor (§6.3).
- The `portfolio_pid` column on child tables is indexed for the
  portfolio roll-up (FR-2 `?portfolio=` filter); it is kept consistent
  with `data.portfolio_ref` by the model layer (§5.6 / §5.8).
- Open question OQ-3: index the high-traffic sub-resource columns
  (`tasks.status`, `tasks.assignee_ref`, `issues.severity`,
  `(parent_kind, parent_pid)`) and add JSONB GIN on each table's `data`
  once search / candidate blocking lands.

### 10.4 Bulk import / export — `bulk_jobs`

Async bulk operations (§9.6) add one table, `bulk_jobs`, per the
family-wide schema in
[bulk import/export §3](../../agents/share/bulk-import-export.md) — the
canonical column list lives there and is **not** restated. It tracks
each import/export job (`kind`, `format`, `status`, `params`, the
`rows_total`/`rows_created`/`rows_upserted`/`rows_to_review`/`rows_errored`
counts, `actor`, artifact URLs, and `expires_at` TTL), with
`UNIQUE (entity, kind, idempotency_key)` so a retried submit maps to
the same job. The `entity` discriminator distinguishes the four
collections. Jobs run on the loco `bg_pg` worker; artifacts (uploaded
source, export output, error report) live in the config-driven
artifact store (S3-compatible in deployment, local fs in dev),
referenced by short-lived access-controlled URLs.

Portfolio-specific notes:

- **Idempotency** is anchored on the §9.6 stable keys (a deterministic
  scheme-scoped external-PM identifier, the owner-scoped
  `(owner_org_id, code)`, or `pid`): re-submitting a file re-upserts
  the same rows to the same state, within the same collection.
- The matcher's **single schema** holds for the thin record: the JSONB
  `data` column is the full thin `WorkItem` payload, so JSONL
  round-trips losslessly and CSV flattens per §9.6 (every repeated /
  nested field a JSON-encoded cell); no bulk path bypasses the JSONB
  round-trip invariant (§5.8). Bulk import targets the thin work-item
  record; sub-resource bulk is a roadmap extension.
