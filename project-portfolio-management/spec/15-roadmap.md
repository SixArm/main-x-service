## 15. Roadmap

Roadmap items become §13 tasks when they are concrete enough to size
and accept.

### PPM feature roadmap (proposed 2026-07-18, with the rename)

The subproject rename (`portfolio` → `project-portfolio-management`)
repositions this trio as a **project portfolio management product**,
not just a matchable registry of work-item identities. The catalogue
below maps the standard PPM capability set onto what the trio already
ships (registry CRUD + within-kind matching/merge across the four
collections; goals / tasks / issues sub-resources; timeline + burndown
views; audit + events; bulk import/export; cross-service people/org
links; ABAC). Design doctrine for every item: new capabilities are
**operational sub-resources** (tables keyed `(parent_kind,
parent_pid)`, like goals/tasks/issues), every mutation audits + emits,
record-level ABAC attributes gate the governance actions, and nothing
here ever becomes a **matcher signal** (the §8 partition rule —
sameness evidence and operational state stay separate).

**Pillar 1 — Strategic alignment & pipeline management**

- **PPM-1 Work intake.** ✅ *Delivered 2026-07-18 (service T-PPM-A).* A `proposals` pipeline (draft → submitted →
  in-review → approved / rejected → promoted): demand records with
  sponsor, strategic rationale, rough sizing, and requested funding.
  Promotion mints the real work item with `provenance = intake` and a
  link back to the proposal. Bonus the registry heritage makes cheap:
  run the **matcher at intake** so a duplicate demand is flagged
  against both existing proposals and live work items before it is
  funded.
- **PPM-2 Idea management.** ✅ *Delivered 2026-07-18 (service T-PPM-C).* Lightweight `ideas` (title, pitch, tags,
  votes) convertible to proposals in one action; the roadmap's
  posts/comments sub-resources attach here first so brainstorming
  threads live with the idea. Ideas are deliberately schema-thin —
  the funnel is idea → proposal → work item, each step adding rigour.
- **PPM-3 Phase-gate approvals.** ✅ *Delivered 2026-07-18 (service T-PPM-A).* A per-work-item `stage` plus
  first-class `gate_reviews` (gate name, decision, conditions,
  approver `worker:` ref, date). Writes on a gate-locked work item are
  policy-refusable via a `resource.stage` ABAC attribute ("deny write
  past gate-3 unless `access=admin`"), making governance a policy
  statement, not code. Approvals are audited governance events.
- **PPM-4 Scenario planning.** ✅ *Delivered 2026-07-18 (service T-PPM-C).* `scenarios`: named candidate portfolios
  (a set of work items / proposals + constraint knobs — budget cap,
  capacity cap, must-include). A pure-core evaluator scores each
  scenario against PPM-8 capacity and PPM-10 budget data (total cost,
  demand vs capacity, alignment score) so what-if comparison is
  arithmetic over live data, patient-flow-at-a-glance style.
  Committing a scenario stamps the chosen items' funding state.
- **PPM-5 OKR / objective alignment.** ✅ *Delivered 2026-07-18 (service T-PPM-C).* An org-level `objectives`
  registry (OKRs) and a work-item → objective mapping with weights;
  alignment rolls up per collection and per parent portfolio, so
  "how much of the portfolio serves objective X" is a query. Work-item
  `goals` already exist; this adds the strategic layer above them.

**Pillar 2 — Execution & visibility**

- **PPM-6 Roadmap & dependency views.** ✅ *Delivered 2026-07-18 (service T-PPM-B).* Timeline/Gantt exists;
  add cross-work-item `dependencies` (finish-start edges between
  tasks/work items, with lag) + milestone records, and derive the
  critical path + slipping-dependency warnings in the timeline view.
- **PPM-7 Portfolio dashboards.** ✅ *Delivered 2026-07-18 (service T-PPM-B).* A portfolio-level at-a-glance
  endpoint (the patient-flow pattern, ETag-conditional): per-collection
  rollups of RAG health, schedule variance (timeframe vs today), open
  risks/issues by severity, budget variance (PPM-10), capacity
  hot-spots (PPM-8), gate-stage distribution — plus site-tile
  headlines for the executive view.
- **PPM-8 Resource capacity planning.** ✅ *Delivered 2026-07-18 (service T-PPM-B).* `allocations`: person/worker
  `EntityRef` + work item (or task) + percentage + timeframe. Rollup
  per person against a configurable weekly capacity ⇒ over-allocation
  detection, a capacity heatmap, and reassignment suggestions
  (largest-slack first). People stay references — no demographics
  copied (family doctrine), and allocations are never matcher
  signals.
- **PPM-9 Custom reporting.** ✅ *Delivered 2026-07-18 (service T-PPM-B; synchronous runs — scheduled/bulk-artifact runs await the family bulk machinery).* Saved report definitions (filter +
  field projection + grouping) executed through the existing bulk
  export machinery (JSONL/CSV; Parquet when the family lands it), a
  KPI/OKR snapshot endpoint for stakeholder decks, and scheduled
  report generation as a `bg_pg` job with the artifact-store TTL
  posture from `agents/share/bulk-import-export.md`.

**Pillar 3 — Value realization & governance**

- **PPM-10 Budget tracking.** ✅ *Delivered 2026-07-18 (service T-PPM-A).* `budget_lines` per work item
  (capex/opex, currency amount via `bigdecimal`, period) + recorded
  actuals (manual or bulk-imported from finance) ⇒ projected-vs-actual
  variance in the dashboards, rolled up the parent-portfolio
  hierarchy. Centralises financial oversight without becoming a
  ledger — actuals are imported facts, not double-entry bookkeeping.
- **PPM-11 Benefits tracking.** ✅ *Delivered 2026-07-18 (service T-PPM-C).* `benefits` per work item: category,
  metric, baseline, target, expected realization date — then recorded
  actuals over time ⇒ realized-vs-projected value and simple ROI
  (with PPM-10 costs). Benefits are reviewed at phase gates (PPM-3),
  closing the loop between the funding case and delivery.
- **PPM-12 Risk management.** ✅ *Delivered 2026-07-18 (service T-PPM-A).* `risks` (probability × impact scoring,
  owner, mitigation, review date) alongside the existing issues;
  escalation converts a materialised risk into an issue with lineage.
  Portfolio-level rollup (exposure by severity, overdue reviews) in
  PPM-7; cross-project dependency risks ride the PPM-6 edges.

**Suggested phasing** — A (governance core, ✅ service side delivered 2026-07-18): PPM-1, PPM-3, PPM-12,
PPM-10 — intake, gates, risks, budgets give the portfolio office its
control loop. B (visibility, ✅ service side delivered 2026-07-18): PPM-6, PPM-7, PPM-8, PPM-9 — dashboards
and capacity make the control loop observable. C (strategy, ✅ service side delivered 2026-07-18): PPM-2,
PPM-4, PPM-5, PPM-11 — scenarios, OKRs, and benefits need A + B's
data to be meaningful. **The whole catalogue is delivered service-side, and the
front-end views over it landed 2026-07-18** (dashboard, intake +
idea boards, per-item governance panel, portfolio schedule,
scenarios, objectives, capacity, reports — see the front-end
CHANGELOG). Locale catalogues for the new views are the remaining
follow-up. Each item lands as spec §13 tasks (three-part
rule) when accepted.

### Longer arc (pre-rename roadmap)

The items below predate the PPM repositioning and remain valid.

- **Collaboration sub-resources.** The plan-family lineage carried
  posts, comments, and membership sub-resources; this entity ships
  goals / tasks / issues only (§2.3). Add posts / comments (Markdown
  update threads on a work item / task / issue) and members (a user's
  membership of a work item with a role) when collaboration becomes a
  requirement — each a new sub-resource table keyed by `(parent_kind,
  parent_pid)`, with membership-scoped write authorisation.
- **Family parity — match / search / merge.** Beyond the MVP
  baseline, reach the mature-sibling shape
  ([`agents/share/match-search-merge.md`](../../agents/share/match-search-merge.md)):
  Tantivy full-text + fuzzy search over the JSONB payload (per
  collection), search-blocked duplicate candidates (replacing the
  in-memory scan, OQ-2), batch deduplicate scan, and a front-end merge
  action.
- **Auditability — durable event bus.** The MVP ships an in-memory
  `WorkItemEvent` stream (T-5); replace it with the durable event bus
  ([`agents/share/event-bus.md`](../../agents/share/event-bus.md)) so
  peer registries, analytics, and the cross-service link aggregator
  can subscribe across replicas. Work-item events are high-volume
  (every task write), so batched outbox emission matters.
- **Cross-service link aggregator.** Stand up (or join) the
  `link-graph-service`
  ([`agents/share/cross-service-linking.md` §4.3](../../agents/share/cross-service-linking.md))
  so a work item's `EntityRef`s and `entity_links` become a traversable
  graph (a work item's people → their orgs, related work items across
  departments). The portfolio trio ships only the write-side (T-7); the
  aggregator is a separate service.
- **Sub-resource bulk + linking.** Extend bulk import/export and the
  cross-service link write-side to the sub-resources (bulk-load tasks
  from a source PM tool; link a task to a `case` or a `thing`).
- **Security.** Blanket auth enforcement on `/api/*` (PASETO v4 public
  token / cookie session per
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
  superseding the RS256-JWT model) with **ABAC** authorisation over
  the token's `attrs` claim (per
  [`agents/share/authorization-attributes.md`](../../agents/share/authorization-attributes.md);
  delivered, supersedes the earlier role-based sketch) — deployments
  express read/write/destructive tiers and any read-integrator vs
  work-item-operator split as policy attributes, not fixed roles;
  rate limiting.
- **PM-tool sync.** Two-way sync with Jira / Asana / MS Project /
  Linear / GitHub Projects keyed on the deterministic external-id
  identifiers (R-0 schemes), so a registered project stays in step with
  its source-tool twin without becoming a full PM replacement (§8.7).
- **Localization.** Operator UI in the
  [`agents/share/locales.md`](../../agents/share/locales.md) locale
  set; multilingual work-item names via `alternate_names` +
  `in_language`; cross-language duplicate linkage through deterministic
  identifiers / `same_as`.
- **Scale-out and operations.** Multi-replica deployment, PostgreSQL
  replication, JSONB GIN + sub-resource indexing (OQ-3), OTLP
  observability pipeline, Prometheus metrics, backup / DR runbooks,
  container hardening.
- **gRPC.** Tonic stub once a high-throughput consumer exists. (The
  OpenAPI 3 doc + Swagger UI for the REST surface ship under T-9.)
