## 10. Persistence

PostgreSQL via SeaORM 1.1 + `sea-orm-migration`, owned entirely by the
service crate. Family conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md). All
tables below are **planned** — the entity is spec-only (§14).

### 10.1 Tables

The thin matchable record lives in **one `plans` table** (a nullable
`kind` label, a nullable `parent_pid` containment link); the operational
sub-resources each get their own table keyed by `parent_pid` (the
partition, §5.6).

**`plans`** — the thin matchable record:

| Column | Type | Notes |
|---|---|---|
| `id` | `PkAuto` | Internal row id |
| `pid` | `UuidUniq` | Public id (route param) |
| `name` | `String` | Denormalised `data.name` for listing |
| `data` | `JsonBinary` (JSONB) | Full thin `Plan` payload (incl. optional `kind`, `goals[]`) |
| `kind` | `StringNull` | Denormalised optional `data.kind` label (`Portfolio`\|`Project`\|`Product`\|`Program`\|`Practice`\|`Process`\|`Purpose`\|`Pathway`\|`Proposal`); nullable — descriptive only, not a discriminator |
| `parent_pid` | `UuidNull` | Denormalised `data.parent_ref` for cheap roll-up of a plan's children (indexed); NULL for a root plan |
| `active` | `BooleanWithDefault(true)` | Registry flag |
| `deleted_at` | `TimestampWithTimeZoneNull` | Soft delete |

The optional `kind` label rides in the payload verbatim; when present
the denormalised `kind` column mirrors it. `kind` does **not** map to a
table or gate matching — every plan lives in the one `plans` table and
matching is **kind-agnostic** (§5.5): any two plan rows may be compared.

**Operational sub-resources** — one table each, every row carrying
`parent_pid UUID` (indexed) + `pid UUID unique` + `deleted_at` for soft
delete. None is serialised into any `data` column; none reaches the
matcher.

| Table | Key columns (beyond `id`, `pid`, `parent_pid`, `deleted_at`) |
|---|---|
| `tasks` | `title`, `description?`, `assignee_ref?`, `status`, `goal_id?`, `parent_task_id?`, `estimate?`, `remaining?`, `due_date?` |
| `issues` | `title`, `kind`, `severity`, `status`, `assignee_ref?` |
| `task_snapshots` | `task_pid`, `estimate`, `remaining`, `observed_at` — feeds the burndown derived view (§6.4) |

Goals are **not** a separate table: they live in `data.goals[]` on the
parent plan's row (§5.3), exposed as a sub-resource via JSONB array
mutation (§10.2).

**Family-baseline tables:**

- **`audit_logs`** — one row per CRUD action on a plan **or any
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
the single schema for the matchable record on the one `plans` table.
Adding a field to `Plan` is a matcher-crate change plus a CHANGELOG
entry — **no service migration** — because serde fills missing fields
with defaults on read. The trade-off: querying inside the payload needs
JSONB operators (or GIN indexes, roadmap), and the only relational
projections of the thin record are `pid`, `name`, the optional `kind`
label, and `parent_pid`.

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
  `deleted_at IS NULL`. Soft-deleting a plan hides its
  sub-resources from read paths (§5.8); merge re-homes them to the
  survivor (§6.3).
- The `parent_pid` column on `plans` is indexed for the child roll-up
  (FR-2 `?parent=` filter); it is kept consistent with
  `data.parent_ref` by the model layer (§5.6 / §5.8).
- Open question OQ-3: index the high-traffic sub-resource columns
  (`tasks.status`, `tasks.assignee_ref`, `issues.severity`,
  `parent_pid`) and add JSONB GIN on the `plans` `data` column
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
the same job. The `entity` discriminator is the one `plans` collection.
Jobs run on the loco `bg_pg` worker; artifacts (uploaded source, export
output, error report) live in the config-driven artifact store
(S3-compatible in deployment, local fs in dev), referenced by
short-lived access-controlled URLs.

Portfolio-specific notes:

- **Idempotency** is anchored on the §9.6 stable keys (a deterministic
  scheme-scoped external-PM identifier, the owner-scoped
  `(owner_org_id, code)`, or `pid`): re-submitting a file re-upserts
  the same rows to the same state.
- The matcher's **single schema** holds for the thin record: the JSONB
  `data` column is the full thin `Plan` payload, so JSONL round-trips
  losslessly and CSV flattens per §9.6 (every repeated / nested field a
  JSON-encoded cell); no bulk path bypasses the JSONB round-trip
  invariant (§5.8). Bulk import targets the thin plan record;
  sub-resource bulk is a roadmap extension.

### 10.5 Collaboration / automation tables (2026-07-22)

Five tables behind §6.4a, none of them part of the matcher payload:

| Table | Holds |
|---|---|
| `reviews` | One subject (idea / proposal / plan) delegated to one expert: `reviewer_ref` (EntityRef), `reviewer_scope`, `expertise`, status, due date, verdict (`score`, `recommendation`, `comment`). A partial unique index keeps one **live** invitation per subject + reviewer, so a fresh round is possible after a verdict |
| `automations` | One rule: optional `plan_pid` scope, trigger (`trigger_kind` + optional `from_status` / `to_status`), action (`action_kind` + JSONB `action_value`), `enabled` |
| `automation_runs` | Append-only log of every firing: outcome (`applied` / `skipped` / `failed`) + detail |
| `scheduled_actions` | The set-and-forget queue: `due_at`, `status`, payload, the automation that scheduled it, and the fired stamp + outcome |
| `notifications` | The in-app inbox written by automations and by the sweep; `read_at` for the read stamp |

**No Smart Score table.** The score (§6.4e) is derived on read from rows
the service already stores, so it can never drift from the evidence it
claims to summarise.
