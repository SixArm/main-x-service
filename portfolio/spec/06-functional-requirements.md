## 6. Functional Requirements

Each requirement names its owning subproject. The entity is
**spec-only; no code exists yet** (§14), so every requirement is a
target tracked in §13 / §15.

The four work-item collections — `portfolios`, `projects`, `products`,
`programs` — have the **identical** controller shape; below,
`{collection}` stands for any one of the four (`{pid}` is a record in
that collection). All matching, dedup, and merge endpoints operate
**within a single collection** — the matcher's kind gate (§5.5)
enforces that a project is never compared with a product.

### 6.1 Registry CRUD — service (thin record)

- **FR-1** Create a work item: `POST /api/{collection}` with a
  `WorkItem` body whose `kind` matches the collection; reject a blank
  `name` with `422`; return `{pid, name}`.
- **FR-1a** Validate the body (service): reject with `422` a blank
  `name`, a `kind` that does not match the collection, a malformed
  `EntityRef` (`owner_org_id`, `lead_ref` — must parse as
  `<entity_type>:<id>`), a malformed `portfolio_ref` (must be a valid
  portfolio `pid`; present only on a child kind), a malformed
  deterministic `identifiers` entry (UUID / URI / external-PM-id shape
  — rejecting a malformed *deterministic* identifier matters because a
  shared value short-circuits the match to 1.0, R-0), a relationship
  that self-references or names an unknown work item, or a malformed
  `in_language` (BCP-47). All problems are reported in one response.
- **FR-2** List active work items: `GET /api/{collection}` returns
  `{pid, name}` refs, most-recent first, capped at 100. For child
  collections a `?portfolio=<pid>` filter rolls up one portfolio's
  children.
- **FR-3** Read: `GET /api/{collection}/{pid}` returns the stored thin
  `WorkItem`; `404` for unknown or soft-deleted `pid`.
- **FR-4** Update: `PUT /api/{collection}/{pid}` replaces the whole
  thin payload (and the denormalised `name` / `portfolio_pid`); same
  validation as FR-1a.
- **FR-5** Soft delete: `DELETE /api/{collection}/{pid}` sets
  `deleted_at`; the record and its sub-resources disappear from list /
  read / match.

### 6.2 Name search — service

- **FR-5a** `GET /api/{collection}/search?q=` — case-insensitive name
  search within the collection (Postgres `ILIKE` on the denormalised
  `name`, cap 50, wildcards escaped); blank `q` → `400`. Tantivy
  full-text over the payload is roadmap (§15).

### 6.3 Matching — matcher (algorithm) + service (endpoints)

Algorithm reference: [`AGENTS/matching.md`](../AGENTS/matching.md) and
the matcher [spec §5–§18](../portfolio-matcher-rust-crate/spec/index.md).

- **FR-6 (kind gate, R-GATE)** Before any other rule the matcher
  short-circuits to **0.0** when `A.kind != B.kind`: two work items of
  different kind are distinct record types and never match. Service
  endpoints only ever feed same-kind candidates, so the gate is a
  defence-in-depth guarantee.
- **FR-7** Deterministic short-circuits (matcher), evaluated only after
  the kind gate passes: score pins to 1.0 on —
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
  | Portfolio | 0.08 | `portfolio_ref` exact parent match (child kinds); skipped when either unset or for the Portfolio kind |
  | Timeframe | 0.07 | Date proximity over `start_date` / `target_date`; skipped when no dates |
  | Keywords | 0.05 | Jaccard over folded sets |
  | Relationships | 0.05 | Typed-set Jaccard over `(relation, work_item_id)`; skipped when either empty |
  | Tags | 0.05 | Set Jaccard over case-folded sets; skipped when either empty |

  Weights sum to 1.00. (`status` is informational-only and carries no
  weight; the parent-portfolio component replaces the plan-family
  `plan_type` weight — kind is a gate, not a weight.)
- **FR-9** Explainability (matcher): every result carries `score`,
  `Confidence` (`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`),
  `is_match` (threshold 0.85 default; `strict` 0.95 / `lenient`
  0.70), and a per-component `MatchBreakdown` (a kind-gated no-match
  reports score 0.0 with the gate as the reason).
- **FR-10** Ad-hoc ranking (service): `POST /api/{collection}/match`
  scores a `{query, candidates}` set without persistence, returning
  ranked `(index, MatchResult)` pairs. The query and candidates are all
  of the collection's kind.
- **FR-11** Duplicate check (service):
  `POST /api/{collection}/check-duplicates` matches a query against
  stored work items **of the same kind** and returns hits above
  threshold as `{pid, name, score, confidence, is_match}`, sorted by
  score descending.
- **FR-11a** Real-time duplicate detection on create (service):
  `POST /api/{collection}` returns `409 Conflict` with candidate
  matches when a likely duplicate is detected (family baseline,
  [`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md));
  a `force` flag bypasses for deliberate near-duplicates.
- **FR-11b** Record merge (service): `POST /api/{collection}/merge`
  folds a confirmed-duplicate work item into a surviving one **of the
  same kind** — union the list fields, keep the duplicate's name as an
  `alternate_names` entry, **re-home the duplicate's sub-resources**
  (tasks / issues re-keyed to the survivor's `pid`), soft-delete the
  duplicate, write a `merge_records` history row (snapshot of the
  transferred payload), and publish a `Merged` event. Equal
  `main_pid`/`duplicate_pid` → `422`; unknown pid → `404`.
  `GET /api/{collection}/merges/recent` lists the history.

### 6.4 Operational sub-resources — service (project-management tool)

Each is a child resource of a work item, keyed by `(parent_kind,
parent_pid)`, in its own Postgres table (§10.1). None enters the
matcher payload (§5.6). All sub-resource writes emit events and audit
rows (§6.6). The sub-resources hang off **any** work item, in any of
the four collections.

- **FR-12 Goals.** CRUD goals under a work item
  (`…/{collection}/{pid}/goals`); goal writes mutate `data.goals[]` on
  the parent so the matchable payload and the sub-resource stay
  consistent (§5.3, §10.2). `Goal { title, description?, target_date?,
  status }`.
- **FR-13 Tasks.** CRUD tasks under a work item
  (`…/{collection}/{pid}/tasks`). `Task { pid, parent_kind,
  parent_pid, title, description?, assignee_ref? (EntityRef), status:
  Todo|InProgress|InReview|Done|Blocked, goal_id?, parent_task_id?,
  estimate?, remaining?, due_date? }`. A task may attach to a goal and
  nest under a parent task.
- **FR-14 Issues.** CRUD issues under a work item
  (`…/{collection}/{pid}/issues`). `Issue { pid, parent_kind,
  parent_pid, title, kind: Bug|Risk|Blocker|Question|Improvement,
  severity: Low|Med|High|Critical, status:
  Open|InProgress|Resolved|Closed, assignee_ref? }`.
- **FR-15 Timeline (derived).** `GET …/{collection}/{pid}/timeline`
  returns a Gantt-style projection: goals with a `target_date` as
  milestones + tasks with `due_date` / date ranges as bars. Read-only;
  computed, not stored.
- **FR-16 Burndown (derived).** `GET …/{collection}/{pid}/burndown`
  returns remaining-vs-estimate over time, from periodic snapshots of
  task `estimate` / `remaining`. Read-only; computed from snapshots.

### 6.5 Cross-service links & bulk — service

- **FR-17 Cross-service links.** A work item / goal / task / issue can
  link to **any** index entity. The service ships the **write-side**:
  `POST`/`GET`/`DELETE …/{pid}/links` over an `entity_links` table and
  `linked` / `unlinked` events, per
  [`agents/share/cross-service-linking.md` §4](../../agents/share/cross-service-linking.md).
  Links are **never** a match signal (§7 there). The read-model
  aggregator (`link-graph-service`) is out of scope here (§2.3).
- **FR-18 Bulk import / export.** Async, job-based bulk per
  [`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md):
  the five endpoints on `bg_pg`, JSONL / CSV / Parquet codecs,
  upsert-by-stable-key + dedupe-to-review, per-row error report, and
  export masking + audit, **per collection**. Portfolio-specific stable
  keys and CSV columns are in §9.6.

### 6.6 Auditability — service

- **FR-19** Audit log: a best-effort `audit_logs` row per create /
  update / delete / merge on a work item **and on every sub-resource
  write** (action + JSON snapshot + actor + timestamp + collection),
  durable in Postgres; read at `…/audit/recent` and `…/{pid}/audit`.
  Per
  [`agents/share/auditability.md`](../../agents/share/auditability.md).
- **FR-20** Event streaming: a `WorkItemEvent`
  (`created`/`updated`/`deleted`/`merged`, plus sub-resource and
  `linked`/`unlinked` events, each carrying the collection / kind) per
  write to an in-memory stream (MVP), read at `…/events/recent`; the
  durable bus
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

- **FR-22** List active work items per collection at `/portfolios`,
  `/projects`, `/products`, `/programs`; create at `…/new`; detail at
  `…/[pid]` (render, edit, delete, check-duplicates); edit at
  `…/[pid]/edit`. A portfolio detail rolls up its child projects /
  products / programs.
- **FR-23** Sub-resource workspaces under `…/[pid]`: goals, tasks
  (board + list), issues; plus the timeline and burndown views (§9.3).
- **FR-24** Check-duplicates posts the current record and lists matches
  within the same collection (name, score, confidence), excluding the
  record itself; a merge action initiates `POST …/merge` (roadmap leg
  of the front-end).
- **FR-25** The work-item form edits the full thin DTO: the `kind`
  fixed by the collection, comma-list inputs for names / keywords /
  tags / sameAs, row editors for goals, identifiers, and relationships,
  an `EntityRef` picker for `owner_org_id` / `lead_ref`, and (on child
  kinds) a parent-portfolio picker for `portfolio_ref`.
