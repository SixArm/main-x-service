## 6. Functional Requirements

Each requirement names its owning subproject. The entity is
**spec-only; no code exists yet** (§14), so every requirement is a
target tracked in §13 / §15.

There is one plans collection — `/api/plans` — where `{pid}` is a plan
record. All matching, dedup, and merge endpoints are **kind-agnostic**:
any two plans may be compared (§5.5); the optional `kind` label neither
gates nor scores.

### 6.1 Registry CRUD — service (thin record)

- **FR-1** Create a plan: `POST /api/plans` with a `Plan` body (optional
  `kind` label); reject a blank `name` with `422`; return `{pid, name}`.
- **FR-1a** Validate the body (service): reject with `422` a blank
  `name`, a malformed `EntityRef` (`owner_org_id`, `lead_ref` — must
  parse as `<entity_type>:<id>`), a malformed `parent_ref` (must be a
  valid plan `pid`) or a `parent_ref` that would form a **containment
  cycle** (points a plan at itself or at one of its descendants), a
  malformed deterministic `identifiers` entry (UUID / URI / external-PM-id
  shape — rejecting a malformed *deterministic* identifier matters
  because a shared value short-circuits the match to 1.0, R-0), a
  relationship that self-references or names an unknown plan, or a
  malformed `in_language` (BCP-47). All problems are reported in one
  response.
- **FR-2** List active plans: `GET /api/plans` returns
  `{pid, name}` refs, most-recent first, capped at 100. A
  `?parent=<pid>` filter rolls up one plan's children.
- **FR-3** Read: `GET /api/plans/{pid}` returns the stored thin
  `Plan`; `404` for unknown or soft-deleted `pid`.
- **FR-4** Update: `PUT /api/plans/{pid}` replaces the whole
  thin payload (and the denormalised `name` / `parent_pid`); same
  validation as FR-1a.
- **FR-5** Soft delete: `DELETE /api/plans/{pid}` sets
  `deleted_at`; the record and its sub-resources disappear from list /
  read / match.

### 6.2 Name search — service

- **FR-5a** `GET /api/plans/search?q=` — case-insensitive name
  search (Postgres `ILIKE` on the denormalised `name`, cap 50,
  wildcards escaped); blank `q` → `400`. Tantivy full-text over the
  payload is roadmap (§15).

### 6.3 Matching — matcher (algorithm) + service (endpoints)

Algorithm reference: [`AGENTS/matching.md`](../AGENTS/matching.md) and
the matcher [spec §5–§18](../project-portfolio-management-matcher-rust-crate/spec/index.md).

- **FR-6 (kind-agnostic matching)** The matcher compares any two plans
  regardless of their optional `kind` label; there is **no kind gate**.
  Service endpoints feed candidate plans without kind filtering.
- **FR-7** Deterministic short-circuits (matcher): score pins to 1.0
  on —
  - **R-0**: any shared value on a **deterministic identifier scheme**
    (`Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`, `TrelloBoardId`,
    `MsProjectId`, `GitHubProjectId`, `LinearId`); the owner-scoped
    `Code` / `LocalId` and `Custom` are **excluded**;
  - **R-1**: same non-empty `owner_org_id` + equal normalised `code`;
  - **R-2**: any case-folded `same_as` URL overlap.
- **FR-8** Probabilistic components (matcher), renormalised weighted
  average over the components both records carry:

  | Component | Weight | Algorithm |
  |---|---:|---|
  | Name | 0.30 | Best Jaro-Winkler over `name` + `alternate_names`; Soundex +0.05 bonus capped at 0.95 |
  | Goals | 0.15 | Jaccard over folded goal *titles* |
  | Code | 0.15 | Same `owner_org_id`: 1.0/0.0; differing or missing owner: skipped |
  | Owner org | 0.10 | `owner_org_id` exact `EntityRef`; skipped when either unset |
  | Parent | 0.08 | `parent_ref` exact parent-plan match; skipped when either unset |
  | Timeframe | 0.07 | Date proximity over `start_date` / `target_date`; skipped when no dates |
  | Keywords | 0.05 | Jaccard over folded sets |
  | Relationships | 0.05 | Typed-set Jaccard over `(relation, plan_id)`; skipped when either empty |
  | Tags | 0.05 | Set Jaccard over case-folded sets; skipped when either empty |

  Weights sum to 1.00. (`status` and the optional `kind` label are
  informational-only and carry no weight; the parent component replaces
  the plan-family `plan_type` weight.)
- **FR-9** Explainability (matcher): every result carries `score`,
  `Confidence` (`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`),
  `is_match` (threshold 0.85 default; `strict` 0.95 / `lenient`
  0.70), and a per-component `MatchBreakdown`.
- **FR-10** Ad-hoc ranking (service): `POST /api/plans/match`
  scores a `{query, candidates}` set without persistence, returning
  ranked `(index, MatchResult)` pairs.
- **FR-11** Duplicate check (service):
  `POST /api/plans/check-duplicates` matches a query against
  stored plans and returns hits above threshold as
  `{pid, name, score, confidence, is_match}`, sorted by score
  descending.
- **FR-11a** Real-time duplicate detection on create (service):
  `POST /api/plans` returns `409 Conflict` with candidate
  matches when a likely duplicate is detected (family baseline,
  [`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md));
  a `force` flag bypasses for deliberate near-duplicates.
- **FR-11b** Record merge (service): `POST /api/plans/merge`
  folds a confirmed-duplicate plan into a surviving one (any two plans;
  no kind restriction) — union the list fields, keep the duplicate's
  name as an `alternate_names` entry, **re-home the duplicate's
  sub-resources** (tasks / issues re-keyed to the survivor's `pid`),
  soft-delete the duplicate, write a `merge_records` history row
  (snapshot of the transferred payload), and publish a `Merged` event.
  Equal `main_pid`/`duplicate_pid` → `422`; unknown pid → `404`.
  `GET /api/plans/merges/recent` lists the history.

### 6.4 Operational sub-resources — service (project-management tool)

Each is a child resource of a plan, keyed by `parent_pid`, in its own
Postgres table (§10.1). None enters the matcher payload (§5.6). All
sub-resource writes emit events and audit rows (§6.6). The sub-resources
hang off **any** plan.

- **FR-12 Goals.** CRUD goals under a plan
  (`/api/plans/{pid}/goals`); goal writes mutate `data.goals[]` on
  the parent so the matchable payload and the sub-resource stay
  consistent (§5.3, §10.2). `Goal { title, description?, target_date?,
  status }`.
- **FR-13 Tasks.** CRUD tasks under a plan
  (`/api/plans/{pid}/tasks`). `Task { pid, parent_pid, title,
  description?, assignee_ref? (EntityRef), status:
  Todo|InProgress|InReview|Done|Blocked, goal_id?, parent_task_id?,
  estimate?, remaining?, due_date? }`. A task may attach to a goal and
  nest under a parent task.
- **FR-14 Issues.** CRUD issues under a plan
  (`/api/plans/{pid}/issues`). `Issue { pid, parent_pid, title,
  kind: Bug|Risk|Blocker|Question|Improvement,
  severity: Low|Med|High|Critical, status:
  Open|InProgress|Resolved|Closed, assignee_ref? }`.
- **FR-15 Timeline (derived).** `GET /api/plans/{pid}/timeline`
  returns a Gantt-style projection: goals with a `target_date` as
  milestones + tasks with `due_date` / date ranges as bars. Read-only;
  computed, not stored.
- **FR-16 Burndown (derived).** `GET /api/plans/{pid}/burndown`
  returns remaining-vs-estimate over time, from periodic snapshots of
  task `estimate` / `remaining`. Read-only; computed from snapshots.

### 6.4a Collaboration, automation, and prioritisation — service

Delivered 2026-07-22 (service spec §9.4a). None of this enters the
matcher payload (§5.6); every mutation audits (§6.6).

- **FR-16a Collaborative review.** Delegate an idea / proposal / plan
  to one internal or external expert (`/api/reviews`), track
  accept / decline, collect a verdict (`score` 0–100 optional,
  `recommendation` advance|hold|reject), and read the consensus.
  Reviewers are `EntityRef` URNs, **never raw email** — an external
  expert is a record in the person registry — and `reviewer_scope`
  (internal / external) is recorded explicitly, because disclosure to an
  outsider is a decision, not an inference. Only a reviewer who
  **accepted** may submit, so an unanswered invitation never becomes
  evidence; consensus reports a **strict** majority only (a tie or a
  plurality reports none), a `null` mean until somebody scores, and the
  count still outstanding.
- **FR-16b Assignee management.** Assign / unassign a task
  (`…/tasks/{t_pid}/assign`; `null` unassigns) and read the open
  workload per assignee, which surfaces the **unassigned** pile rather
  than dropping it.
- **FR-16c Workflow automation.** Rules configured once
  (`/api/automations`) fire as work crosses the Kanban board (or when a
  plan review is submitted) and apply one action: assign, add label,
  notify, schedule an action, or set a task status. Action shapes are
  validated at **write** time; a failing rule is recorded as a `failed`
  run and **never** undoes the operator's move; actions are applied
  without re-entering the engine, so automations cannot cascade. Every
  firing — applied, skipped, or failed — is logged.
- **FR-16d Set and forget.** A deadline configured once
  (`/api/scheduled-actions`) is held until due and fired **exactly
  once**: the sweep claims each row with a conditional update, so the
  optional in-process ticker and a manual sweep cannot double-fire.
  Notifications are **in-app only** — no email, no push.
- **FR-16e Smart Score (data-driven prioritisation).** A derived,
  renormalised weighted average over ROI, strategic alignment, expert
  review, risk (inverted), demand, MoSCoW priority, and momentum, with a
  full per-component breakdown. **Absent evidence is dropped and
  disclosed** (`missing` + `coverage`), never scored zero; no evidence at
  all ⇒ `null` / `unscored`, sorted last rather than as a zero. ROI is
  computed only within a single currency (no FX conversion). Weights are
  deployment-tunable as a complete basis-point map; anything else falls
  back to the documented defaults with a warning. Nothing is stored, so
  the score cannot drift from its inputs.
- **FR-16f Bird's-eye visibility.** The challenge funnel
  (`/api/lifecycle`) reports every phase even at zero, with live and
  stalled counts, and items in an unresolvable phase counted separately.
  Per plan (`/api/plans/{pid}/lifecycle`), readiness for the next gate
  is a five-check **checklist** — each check reported with the count
  behind it — so "ready" means every check ran and passed.

### 6.5 Cross-service links & bulk — service

- **FR-17 Cross-service links.** A plan / goal / task / issue can
  link to **any** index entity. The service ships the **write-side**:
  `POST`/`GET`/`DELETE /api/plans/{pid}/links` over an `entity_links`
  table and `linked` / `unlinked` events, per
  [`agents/share/cross-service-linking.md` §4](../../agents/share/cross-service-linking.md).
  Links are **never** a match signal (§7 there). The read-model
  aggregator (`link-graph-service`) is out of scope here (§2.3).
- **FR-18 Bulk import / export.** Async, job-based bulk per
  [`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md):
  the five endpoints on `bg_pg`, JSONL / CSV / Parquet codecs,
  upsert-by-stable-key + dedupe-to-review, per-row error report, and
  export masking + audit, on the one plans collection.
  Portfolio-specific stable keys and CSV columns are in §9.6.

### 6.6 Auditability — service

- **FR-19** Audit log: a best-effort `audit_logs` row per create /
  update / delete / merge on a plan **and on every sub-resource
  write** (action + JSON snapshot + actor + timestamp), durable in
  Postgres; read at `/api/plans/audit/recent` and
  `/api/plans/{pid}/audit`. Per
  [`agents/share/auditability.md`](../../agents/share/auditability.md).
- **FR-20** Event streaming: a `PlanEvent`
  (`created`/`updated`/`deleted`/`merged`, plus sub-resource and
  `linked`/`unlinked` events) per write to an in-memory stream (MVP),
  read at `/api/plans/events/recent`; the durable bus
  ([`agents/share/event-bus.md`](../../agents/share/event-bus.md)) is
  roadmap (§15).

### 6.7 Security — service

- **FR-21** Offline PASETO v4 public token verification against the
  central auth-service's published Ed25519 key (embeds
  `authentication-verifier`, now a PASETO verifier); `whoami`
  protected; the audit / merge `actor` is stamped from the token when
  present. Blanket `/api/*` enforcement and paseto-keys-over-HTTP fetch are
  follow-ups (§13, awaiting the coordinated family SSO rollout). Auth
  source of truth (supersedes the RS256-JWT model):
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).

### 6.8 Operator UI — front-end

- **FR-22** List active plans at `/plans`; create at `/plans/new`;
  detail at `/plans/[pid]` (render, edit, delete, check-duplicates);
  edit at `/plans/[pid]/edit`. A plan detail rolls up its child plans.
- **FR-23** Sub-resource workspaces under `/plans/[pid]`: goals, tasks
  (board + list), issues; plus the timeline and burndown views (§9.3).
- **FR-24** Check-duplicates posts the current record and lists matches
  (name, score, confidence), excluding the record itself; a merge
  action initiates `POST /api/plans/merge` (roadmap leg of the
  front-end).
- **FR-25** The plan form edits the full thin DTO: an optional `kind`
  label selector, comma-list inputs for names / keywords / tags /
  sameAs, row editors for goals, identifiers, and relationships, an
  `EntityRef` picker for `owner_org_id` / `lead_ref`, and a
  parent-plan picker for `parent_ref`.
