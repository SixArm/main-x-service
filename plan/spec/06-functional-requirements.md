## 6. Functional Requirements

Each requirement names its owning subproject. The entity is
**spec-only; no code exists yet** (§14), so every requirement is a
target tracked in §13 / §15.

### 6.1 Registry CRUD — service (thin record)

- **FR-1** Create a plan: `POST /api/plans` with a `Plan` body; reject
  a blank `name` with `422`; return `{pid, name}`.
- **FR-1a** Validate the body (service): reject with `422` a blank
  `name`, a malformed `EntityRef` (`owner_org_id`, `lead_ref` — must
  parse as `<entity_type>:<id>`), a malformed deterministic
  `identifiers` entry (UUID / URI / external-PM-id shape — rejecting a
  malformed *deterministic* identifier matters because a shared value
  short-circuits the match to 1.0, R-0), a relationship that
  self-references or names an unknown plan, or a malformed
  `in_language` (BCP-47). All problems are reported in one response.
- **FR-2** List active plans: `GET /api/plans` returns `{pid, name}`
  refs, most-recent first, capped at 100.
- **FR-3** Read: `GET /api/plans/{pid}` returns the stored thin `Plan`;
  `404` for unknown or soft-deleted `pid`.
- **FR-4** Update: `PUT /api/plans/{pid}` replaces the whole thin
  payload (and the denormalised `name`); same validation as FR-1a.
- **FR-5** Soft delete: `DELETE /api/plans/{pid}` sets `deleted_at`;
  the record and its sub-resources disappear from list / read / match.

### 6.2 Name search — service

- **FR-5a** `GET /api/plans/search?q=` — case-insensitive name search
  (Postgres `ILIKE` on the denormalised `name`, cap 50, wildcards
  escaped); blank `q` → `400`. Tantivy full-text over the payload is
  roadmap (§15).

### 6.3 Matching — matcher (algorithm) + service (endpoints)

Algorithm reference: [`AGENTS/matching.md`](../AGENTS/matching.md) and
the matcher [spec §5–§18](../plan-matcher-rust-crate/spec/index.md).

- **FR-6** Deterministic short-circuits (matcher): score pins to 1.0
  on —
  - **R-0**: any shared value on a **deterministic identifier scheme**
    (`Uri`, `Uuid`, `JiraProjectKey`, `AsanaGid`, `TrelloBoardId`,
    `MsProjectId`, `GitHubProjectId`, `LinearId`); the owner-scoped
    `PlanCode` / `LocalId` and `Custom` are **excluded**;
  - **R-1**: same non-empty `owner_org_id` + equal normalised
    `plan_code`;
  - **R-2**: any case-folded `same_as` URL overlap.
- **FR-7** Probabilistic components (matcher), renormalised weighted
  average over the components both records carry:

  | Component | Weight | Algorithm |
  |---|---:|---|
  | Name | 0.30 | Best Jaro-Winkler over `name` + `alternate_names`; Soundex +0.05 bonus capped at 0.95 |
  | Goals | 0.15 | Jaccard over folded goal *titles* |
  | Plan code | 0.15 | Same `owner_org_id`: 1.0/0.0; differing or missing owner: skipped |
  | Owner org | 0.10 | `owner_org_id` exact `EntityRef`; skipped when either unset |
  | Plan type | 0.08 | Exact enum 1.0/0.0; skipped when either unset |
  | Timeframe | 0.07 | Date proximity over `start_date` / `target_date`; skipped when no dates |
  | Keywords | 0.05 | Jaccard over folded sets |
  | Relationships | 0.05 | Typed-set Jaccard over `(relation, plan_id)`; skipped when either empty |
  | Tags | 0.05 | Set Jaccard over case-folded sets; skipped when either empty |

  Weights sum to 1.00.
- **FR-8** Explainability (matcher): every result carries `score`,
  `Confidence` (`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`),
  `is_match` (threshold 0.85 default; `strict` 0.95 / `lenient`
  0.70), and a per-component `MatchBreakdown`.
- **FR-9** Ad-hoc ranking (service): `POST /api/plans/match` scores a
  `{query, candidates}` set without persistence, returning ranked
  `(index, MatchResult)` pairs.
- **FR-10** Duplicate check (service): `POST /api/plans/check-duplicates`
  matches a query against stored plans and returns hits above
  threshold as `{pid, name, score, confidence, is_match}`, sorted by
  score descending.
- **FR-10a** Real-time duplicate detection on create (service):
  `POST /api/plans` returns `409 Conflict` with candidate matches when
  a likely duplicate is detected (family baseline,
  [`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md));
  a `force` flag bypasses for deliberate near-duplicates.
- **FR-10b** Record merge (service): `POST /api/plans/merge` folds a
  confirmed-duplicate plan into a surviving one — union the list
  fields, keep the duplicate's name as an `alternate_names` entry,
  **re-home the duplicate's sub-resources** (tasks / issues / posts /
  comments / members re-keyed to the survivor's `pid`), soft-delete
  the duplicate, write a `merge_records` history row (snapshot of the
  transferred payload), and publish a `Merged` event. Equal
  `main_pid`/`duplicate_pid` → `422`; unknown pid → `404`.
  `GET /api/plans/merges/recent` lists the history.

### 6.4 Operational sub-resources — service (project-management tool)

Each is a child resource of a `Plan`, keyed by the plan `pid`, in its
own Postgres table (§10.1). None enters the matcher payload (§5.6).
All sub-resource writes emit events and audit rows (§6.6).

- **FR-11 Goals.** CRUD goals under a plan (`…/plans/{pid}/goals`);
  goal writes mutate `data.goals[]` on the parent plan so the
  matchable payload and the sub-resource stay consistent (§5.3, §10.2).
  `Goal { title, description?, target_date?, status }`.
- **FR-12 Tasks.** CRUD tasks under a plan (`…/plans/{pid}/tasks`).
  `Task { pid, plan_id, title, description?, assignee_ref?
  (EntityRef), status: Todo|InProgress|InReview|Done|Blocked, goal_id?,
  parent_task_id?, estimate?, remaining?, due_date? }`. A task may
  attach to a goal and nest under a parent task.
- **FR-13 Issues.** CRUD issues under a plan (`…/plans/{pid}/issues`).
  `Issue { pid, plan_id, title, kind:
  Bug|Risk|Blocker|Question|Improvement, severity:
  Low|Med|High|Critical, status: Open|InProgress|Resolved|Closed,
  assignee_ref? }`.
- **FR-14 Posts & comments.** CRUD posts (`…/plans/{pid}/posts`) and
  comments (`…/plans/{pid}/comments`). `Post { pid, plan_id,
  author_ref (EntityRef), title, body_markdown }`; `Comment { pid,
  plan_id, target: (post|task|issue, id), author_ref, body_markdown }`.
  Markdown text only; binary attachments are out of scope (§1.3).
- **FR-15 Members.** CRUD plan membership (`…/plans/{pid}/members`).
  `Member { plan_id, user_ref (EntityRef), role:
  Owner|Lead|Member|Viewer }`. Membership scopes who may write the
  plan's sub-resources (roadmap enforcement once auth lands, §15).
- **FR-16 Timeline (derived).** `GET …/plans/{pid}/timeline` returns a
  Gantt-style projection: goals with a `target_date` as milestones +
  tasks with `due_date` / date ranges as bars. Read-only; computed, not
  stored.
- **FR-17 Burndown (derived).** `GET …/plans/{pid}/burndown` returns
  remaining-vs-estimate over time, from periodic snapshots of task
  `estimate` / `remaining`. Read-only; computed from snapshots.

### 6.5 Cross-service links & bulk — service

- **FR-18 Cross-service links.** A plan / goal / task / issue can link
  to **any** index entity. The service ships the **write-side**:
  `POST`/`GET`/`DELETE …/{pid}/links` over an `entity_links` table and
  `linked` / `unlinked` events, per
  [`agents/share/cross-service-linking.md` §4](../../agents/share/cross-service-linking.md).
  Links are **never** a match signal (§7 there). The read-model
  aggregator (`link-graph-service`) is out of scope here (§2.3).
- **FR-19 Bulk import / export.** Async, job-based bulk per
  [`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md):
  the five endpoints on `bg_pg`, JSONL / CSV / Parquet codecs,
  upsert-by-stable-key + dedupe-to-review, per-row error report, and
  export masking + audit. Plan-specific stable keys and CSV columns
  are in §9.6.

### 6.6 Auditability — service

- **FR-20** Audit log: a best-effort `audit_logs` row per create /
  update / delete / merge on the plan **and on every sub-resource
  write** (action + JSON snapshot + actor + timestamp), durable in
  Postgres; read at `…/audit/recent` and `…/{pid}/audit`. Per
  [`agents/share/auditability.md`](../../agents/share/auditability.md).
- **FR-21** Event streaming: a `PlanEvent`
  (`created`/`updated`/`deleted`/`merged`, plus sub-resource and
  `linked`/`unlinked` events) per write to an in-memory stream (MVP),
  read at `…/events/recent`; the durable bus
  ([`agents/share/event-bus.md`](../../agents/share/event-bus.md)) is
  roadmap (§15).

### 6.7 Security — service

- **FR-22** Offline PASETO v4 public token verification against the
  central auth-service's published Ed25519 key (embeds
  `authentication-verifier`, now a PASETO verifier); `whoami`
  protected; the audit / merge `actor` is stamped from the token when
  present. Blanket `/api/*` enforcement and paseto-keys-over-HTTP fetch are
  follow-ups (§13, awaiting the coordinated family SSO rollout). Auth
  source of truth (supersedes the RS256-JWT model):
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).

### 6.8 Operator UI — front-end

- **FR-23** List active plans at `/`; create at `/new`; detail at
  `/[pid]` (render, edit, delete, check-duplicates); edit at
  `/[pid]/edit`.
- **FR-24** Sub-resource workspaces under `/[pid]`: goals, tasks (board
  + list), issues, posts / comments, members; plus the timeline and
  burndown views (§9.2).
- **FR-25** Check-duplicates posts the current record and lists matches
  (name, score, confidence), excluding the record itself; a merge
  action initiates `POST /merge` (roadmap leg of the front-end).
- **FR-26** The plan form edits the full thin DTO: comma-list inputs
  for names / keywords / tags / sameAs, row editors for goals,
  identifiers, and relationships, and `EntityRef` pickers for
  `owner_org_id` / `lead_ref`.
