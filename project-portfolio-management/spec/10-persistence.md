## 10. Persistence

PostgreSQL via SeaORM 1.1 + `sea-orm-migration`, owned entirely by the
service crate. Family conventions:
[`agents/share/postgresql.md`](../../agents/share/postgresql.md).

> **Corrected 2026-08-25.** This paragraph read *"All tables below are
> **planned** — the entity is spec-only (§14)"*, which §14 contradicts:
> the service is implemented and green. The tables in §10.1–§10.5 exist;
> §10.6 is the one section that is genuinely still planned, and says so.

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


### 10.6 Full-suite tables (§1.4–§1.6, §6.4c) — planned

**Not yet built** (§2.3). Every table carries `pid UUID unique` and
`deleted_at` for soft delete; every plan-scoped table carries an indexed
`plan_pid UUID`. **None is serialised into any `data` column and none
reaches the matcher** (§5.6).

| Table | Holds | FR |
|---|---|---|
| `workflows` | One configuration: optional `plan_pid` scope, `applies_to` (task \| issue), `is_default` | FR-26 |
| `workflow_states` | `workflow_pid`, `key`, `label`, **`category`** (`todo`\|`active`\|`waiting`\|`done`, NOT NULL), `wip_limit?`, `is_initial`, `is_terminal` | FR-26 |
| `workflow_transitions` | `workflow_pid`, `from_key`, `to_key` | FR-26 |
| `key_results` | `goal_id`, `title`, `metric`, `start_value`, `target_value`, `current_value`, `direction`, `unit?`, `currency?`, `owner_ref?`, `due_date?`, `status` | FR-27 |
| `key_result_check_ins` | `key_result_pid`, `observed_at`, `value`, `confidence?`, `note?`, `actor` | FR-27 |
| `time_entries` | `plan_pid`, `task_pid?`, `actor_ref`, `spent_on`, `minutes`, `category`, `billable`, `note?` | FR-28 |
| `sprints` | `plan_pid`, `name`, `starts_on`, `ends_on`, `goal?`, `status` | FR-29 |
| `sprint_commitments` | `sprint_pid`, `task_pid`, `committed_at` — the planning snapshot | FR-29 |
| `ceremonies` | `sprint_pid`, `kind`, `held_at`, `facilitator_ref?` | FR-29 |
| `ceremony_notes` | `ceremony_pid`, `category`, `text`, `converted_task_pid?` | FR-29 |
| `phase_transitions` | `plan_pid`, `from?`, `to`, `occurred_at`, `actor?`, `reason?` — append-only | FR-30 |
| `business_case_targets` | `plan_pid`, `metric`, `baseline_value`, `target_value`, `unit?`, `currency?`, `promised_by`, `source`, `approved_at`, `approved_by_ref?` | FR-33 |
| `value_points` | `plan_pid`, `benefit_pid?`, `observed_at`, `value`, `is_first_measurable`, `method`, `evidence_ref?`, `actor` | FR-33 |
| `adoption_snapshots` | `plan_pid`, `observed_at`, `active_users`, `target_users`, `window_days`, `definition` | FR-33 |
| `satisfaction_responses` | `plan_pid`, `surveyed_at`, `instrument`, `score`, `respondent_role`, `comment?` — **no respondent identity** | FR-36 |
| `working_time_configs` | `scope_ref?`, `hours_per_day`, `working_days`, `holidays` — the declared capacity basis | FR-35 |
| `non_working_periods` | `person_ref`, `starts_on`, `ends_on`, `kind`, `note?` — **subtracts from the denominator**, never counted as idle | FR-35 |
| `total_project_control` | `plan_pid`, `currency`, `observed_at`, `dipp`, `dipp_progress_index_numerator`, `dipp_progress_index_denominator`, `dipp_progress_index_ratio` (**`GENERATED ALWAYS`**), `expected_monetary_value`, `cost_estimate_to_complete` | FR-37 |
| `controls` | `plan_pid`, `name`, `timing`, `metric`, `target_value`, `unit?`, `currency?`, `comparator`, `tolerance?`, `source_kind`, `source_ref?`, `cadence?`, `owner_ref?`, `enabled` | FR-38 |
| `control_readings` | `control_pid`, `observed_at`, `value?`, `verdict`, `gap?`, `method` — **append-only** | FR-38 |
| `control_actions` | `reading_pid`, `kind`, `description`, `owner_ref?`, `due_date?`, `converted_task_pid?`, `converted_issue_pid?`, `closed_at?`, `outcome?` | FR-38 |

Schema-level constraints that enforce spec rules rather than trusting
the handlers:

- `workflow_states.category` is `NOT NULL` with a CHECK on the four
  values, so an uncategorised state cannot exist even by direct SQL
  (§5.9.1). A partial unique index enforces one `is_initial` state per
  workflow.
- `phase_transitions` is **append-only** — no update or delete path,
  matching `task_transitions`
  ([time-based-analysis.md](time-based-analysis.md) §5.1) — because a
  phase history that can
  be rewritten cannot support a duration claim.
- `business_case_targets.approved_at` has no update path once set: the
  Time-to-Value clock start must not move (§5.9.6).
- `adoption_snapshots` requires `target_users > 0`; a rate with a zero
  denominator is refused at write, not divided at read.

**No table for any derived figure.** Transformation ROI, Value
Realization Rate, Time to Value, Adoption Rate, SPI, CPI, NPV, Flow
Distribution and the OKR scores are all computed on read from the rows
above, so none can drift from its evidence — the rule §10.5 already
applies to Smart Score.
