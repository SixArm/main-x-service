## 14. Implementation Status

**All three subprojects are implemented and green** (matcher + service +
front-end). Rust rows verified 2026-08-26 by running them, not by
reading them; front-end rows last run 2026-08-23:

| Check | Result |
|---|---|
| matcher — `cargo test` | 58 unit + 6 integration + 10 property + 7 doctests, 0 failed (2026-08-26) |
| service — `cargo test` | 353 unit + 3 non-ignored integration, 0 failed (2026-08-26, incl. the two-way OpenAPI route-agreement tests) |
| service — `cargo test -- --ignored` (needs Postgres 18) | 74 request tests collected (`#[ignore]`d, DB-gated); the suite was observed green via `scripts/ci-check.sh test-db` during the §13 T-15 … T-27 verifications, 2026-08-25/26 |
| service — `cargo clippy --all-targets -- -D warnings` | one in-flight finding (2026-08-26): `too_many_lines` in `src/openapi.rs`'s route-agreement test, which is mid-landing under PRO-P16 (OpenAPI coverage of the new route groups) at the time of this snapshot; clean everywhere else (`#![warn(clippy::pedantic)]`) |
| front-end — `pnpm run check` | 770 files, **0 errors 0 warnings** (2026-08-23) |
| front-end — `pnpm run test` (vitest) | 75 tests across 9 files, 0 failed (2026-08-23) |
| front-end — `pnpm run test:e2e` (Playwright, API-stubbed) | 25 tests, 0 failed (2026-08-23) |

> **This section was materially wrong until 2026-08-23** and the
> correction is worth recording rather than quietly overwriting. Its
> header had been updated when the trio landed (2026-06-19), but §14.1
> still read *"Nothing else is delivered"* and §14.2 *"The entire build
> is open"* — a document that contradicted itself in consecutive
> paragraphs, listing "No matcher crate" and "No service crate" as gaps
> against a tree containing both. A status file that is not regenerated
> is worse than no status file: it is the one place a reader goes for
> exactly the question it answers wrongly. Every row below now names
> what backs it.

The four former work-item kinds (Portfolio / Project / Product /
Program) were unified into one recursive `Plan` on 2026-07-20 (§13
T-10): `kind` is an optional descriptive label that neither gates nor
scores, containment is general `parent_ref`, and one `/api/plans`
collection sits over one `plans` table.

The 2026-08-25/26 delivery (§13 T-15 … T-27, service 0.3.0) added the
**project-management suite**: custom workflows, the OKR engine, effort
/ time tracking with per-person utilisation, sprint ceremonies, project
phases, Flow Distribution, Total Project Control, the controls
register, and value realization / strategic performance — nine
migrations (`m20260825_000001` … `m20260826_000003`) and a matcher
0.2.0 wire addition (`Plan.phase`, pinned never-scored).

### 14.1 Delivered

| Subproject | Capability | Backed by |
|---|---|---|
| (entity) | Canonical specification | This §1–§18 entity spec: the domain model (§5, the canonical home) — the recursive `Plan`, its optional `kind` label, kind-agnostic matching, the matchable/operational partition — the cross-subproject DTO contract, and the family-integration adoptions |
| (entity) | Time-based analysis contract | The cross-cutting [`time-based-analysis.md`](time-based-analysis.md) (§1–§18): the transition log, cycle versus lead time, flow efficiency, first-pass yield, the service level expectation, constraint ranking, and queueing-theory flow |
| matcher | Kind-agnostic `Plan` matching | Name (Jaro-Winkler + Soundex), goal-title & keyword Jaccard, owner-scoped code, owner org, `parent_ref`, timeframe proximity, relationships & tags; deterministic short-circuits on Jira / Asana / Trello / MS-Project / GitHub / Linear ids / URI / UUID, same-owner code, `sameAs` URL. **No kind gate** — `MatchBreakdown.kind_gate_blocked` is vestigial and always `false` |
| matcher | `Plan.phase` / `PlanPhase` (0.2.0) | An additive `#[serde(default)]` DTO field — the five process groups (`initiating` … `closing`) — informational-only and **pinned never-scored** (`phase_is_not_scored`), following the `PlanStatus` precedent |
| matcher | Input-size caps + never-panic | SEC-M1 per-field and array caps; `proptest` property harness; `fuzz/` targets |
| service | Plans CRUD + matching + merge | One `/api/plans` collection over one `plans` table (nullable `kind`, nullable `parent_pid`); the API DTO **is** the matcher's `Plan`, stored verbatim as JSONB; `match`, `check-duplicates`, `merge` + `merges/recent`; containment-cycle `422` |
| service | Tantivy full-text search | `src/search/` — fuzzy + phonetic name search; `kind` is a **search filter** (`?kind=`), deliberately never a duplicate-detection gate |
| service | Operational sub-resources | Goals / objectives, tasks (Kanban board with WIP-limit caps), sprints + notes, milestones, risks, budget lines, benefits, ideas, proposals, scenarios, gate reviews, allocations, dependencies, automations, notifications, report definitions |
| service | Derived views | Burndown (honest — real `done_at` stamps only, no ideal line), velocity, standup digest, executive health / decisions / benefits / alignment, financial variance + exposure, technology radar / debt / flow, board pack, auditor trail + findings + evidence pack, compliance register, risk heatmap, regulator extract |
| service | **Time-based analysis** | `task_transitions` (append-only, written in-transaction by the existing task create/move calls, with a labelled backfill); the pure `src/tba.rs`; `GET /api/plans/{pid}/{time-analysis,constraints,aging-wip,flow}`, `GET /api/plans/{pid}/tasks/{t_pid}/{transitions,time-analysis}`, `GET /api/flow-classes`. See [`time-based-analysis.md`](time-based-analysis.md) §16 |
| service | Audit log + durable outbox events | `audit_logs`; `event_outbox` with `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_TRANSPORT` (default `memory`); `src/relay.rs` relay + `FluvioSink` behind the off-by-default `fluvio` feature |
| service | Auth + ABAC | Offline PASETO v4.public verification via `authentication-verifier`; the blanket `/api/*` guard behind `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` (**default off**); record-level `authorize_record` + `mask` obligations; proved end-to-end by `tests/enforcement.rs` |
| service | Privacy masking | `src/privacy.rs` + `tests/masking.rs` |
| service | Integrity | `src/compliance/` — record content hashing, audit integrity, keyed HMAC MACs via the shared `integrity-mac` crate |
| service | Custom workflows (T-15 / FR-26) | Pure `src/workflow.rs`; migration `m20260825_000005_workflows` (three tables, schema-enforced state categories, one-initial-state partial unique index); `POST`/`GET /api/workflows`, `DELETE /api/workflows/{pid}`, `GET /api/plans/{pid}/workflow`; the task create/move paths validate against the workflow in force; `done_at` stamped from the state **category**; flow classes derived per plan |
| service | OKR engine (T-16 / FR-27) | Pure `src/okr.rs`; migration `m20260825_000006_key_results`; key results anchored to `objectives` (never `goals[]`, which carry no id); `POST`/`GET /api/objectives/{pid}/key-results`, `POST`/`GET /api/key-results/{pid}/check-ins`, `GET /api/plans/{pid}/okr` weighted by the existing `objective_links` weight |
| service | Effort / time tracking (T-17 / FR-28) | Pure `src/effort.rs`; migration `m20260826_000001_effort`; `POST`/`GET /api/plans/{pid}/time-entries`, `GET /api/plans/{pid}/effort` — roll-ups per plan/task/assignee, every one labelled asserted; uncategorised effort reported separately; > 1440 min/day refused |
| service | Sprint ceremonies (T-18 / FR-29) | Migration `m20260826_000002_ceremonies`; `POST`/`GET /api/sprints/{pid}/ceremonies`, `POST /api/sprints/{pid}/commit` (once-only, refused twice by handler *and* partial unique index), `GET /api/sprints/{pid}/commitment`; every ceremony kind reported even at zero |
| service | Project phases (T-19 / FR-30) | Pure `src/phase.rs`; migration `m20260825_000003_phase_transitions` (denormalised `plans.phase` + CHECK, append-only log, no backfill); `PUT /api/plans/{pid}/phase` (a skip is `422` naming the skipped phase; regression needs a reason), `GET /api/plans/{pid}/phase-history`; `DELETE` on the history is `405` |
| service | Flow Distribution (T-20 / FR-31) | Pure `src/distribution.rs`; migration `m20260825_000004_flow_type` (nullable CHECK-constrained `tasks.flow_type`; `unclassified` not storable); `GET /api/plans/{pid}/flow-distribution` with the subtree rollup; the work type is **declared** on task create, never derived |
| service | `plan_phase_changed` automation trigger (T-21, partial) | Its own trigger, not folded into `plan_stage_changed`; phase filters validate against phases, not task statuses; the phase change commits before the rule fires |
| service | Value realization (T-22 / FR-33) | Pure `src/value.rs`; migration `m20260826_000003_value` (four tables); `POST /api/plans/{pid}/{business-case,value-points,adoption,satisfaction}`, `GET /api/plans/{pid}/value-realization` — no value points is `unrealized` never a total loss; Time to Value is a p50/p85 distribution; mixed currencies withhold the ROI; the measured-vs-asserted evidence mix is disclosed |
| service | Strategic performance (T-23 / FR-34 / FR-36, partial) | `satisfaction_responses` + `GET /api/plans/{pid}/performance`: NPS always with its response count (`null` + `no_responses`, never zero); responses store a role, never an identity; SPI/CPI report `null` with `no_baseline`, never `1.0` |
| service | Per-person utilisation (T-24 / FR-35) | `working_time_configs` + `non_working_periods`; `GET /api/capacity/utilization?by=plan\|team\|person` under the [`time-based-analysis.md`](time-based-analysis.md) §7.1 obligations — leave leaves the denominator (`null` + `all_non_working`, never 0%), suppression is visible, team ratios sum rather than average, ≥ 100% flags and never clamps; integer arithmetic throughout |
| service | Total Project Control (T-25 / FR-37) | Pure `src/tpc.rs` (DIPP / progress index / banding / triage; minor units + basis points, no float); migration `m20260825_000001_total_project_control` with the DPI as a `GENERATED ALWAYS` column; `POST`/`GET /api/plans/{pid}/tpc`, `GET …/tpc/report`, `GET /api/tpc`. Mapping doc: [`total-project-control/index.md`](total-project-control/index.md) |
| service | Controls register (T-26 / FR-38–39, partial) | Pure `src/controls.rs` (three timings, four comparators, verdict derivation, coverage); migration `m20260825_000002_controls` (the `verdict` CHECK keeps `unmeasured` a real third value); nine routes under `/api/plans/{pid}/controls`, `/api/controls`, `/api/readings`, `/api/actions`; feedforward may block, feedback may only record; `POST /api/actions/{pid}/convert` turns an action into a task on its own plan (issue conversion deferred — no `issues` store) |
| service | OpenAPI + Swagger | Hand-written `src/openapi.rs`, served at `/api-docs/openapi.json` + `/swagger-ui`, with tests pinning that the documented paths match the mounted routes. All mounted route groups — including the new workflow / effort / ceremony / value groups — are documented; that coverage is landing as PRO-P16 concurrently with this snapshot (2026-08-26) |
| service | API versioning | Version-free URLs; `Accepts-version` header negotiation (`src/version.rs`) |
| front-end | Operator SPA | 34 routes: plans list / new / detail / edit / merge, per-plan board / schedule / governance, plus dashboard, executive, financials, technology, board, auditor, compliance, regulator, risk, security, scenarios, proposals, ideas, objectives, prioritisation, capacity, calendar, gantt, engineering, lifecycle, reports, reviews, automations, signin, verify |
| front-end | Stack + chrome | SvelteKit 2 + Svelte 5 runes, SVAR DataGrid / Kanban / Gantt, Lily Design System headless, 13-locale i18n, BFF server routes (`src/lib/server/`) so no token reaches the browser |
| service | Cross-plan rollup | `GET /api/plans/{pid}/rollup` — flow across a plan and everything it contains: the union of the subtree's tasks (never an average of the children's ratios) plus the per-plan comparison, over a bounded, cycle-safe containment walk that reports its own caps |
| service | Delivery forecasting | `GET /api/plans/{pid}/forecast` — Monte-Carlo over the plan's **throughput** history (both "how long for N items" and "how many in N periods"), deterministic by seed, refusing below a minimum history rather than computing from noise. Correcting a claim this spec itself had wrong: a batch forecast needs throughput, not cycle time |
| service | Flow gauges | `src/flow_metrics.rs` — a default-off refresh loop publishing the `ppm_flow_*` family: flow efficiency, p85 cycle time, WIP, first-pass yield and over-cap column count per plan, capped and small-board-suppressed because `/metrics.prom` is on the public allow-list |
| front-end | Time-based analysis | `/plans/{pid}/flow`: the SLE badge, aging-WIP board, an inline-SVG cumulative flow diagram (no charting dependency, validated palette, table view), constraints, Little's-Law rates, and the cycle/lead distributions. `src/lib/api/tba.ts` + `src/lib/components/CumulativeFlow.svelte` |
| front-end | Tests | 75 vitest units across 9 files (client, plans, ppm, tba, capabilities, merge-validation, plan-form, i18n, layout) + Playwright e2e (`tests/e2e/ppm.spec.ts`, `flow.spec.ts`, `smoke.spec.ts`) |

### 14.2 Open gaps

Live gaps, each pointing at the task that closes it. Absent from this
list means delivered above.

| Gap | Task |
|---|---|
| No cross-service link write-side (`entity_links`, `linked`/`unlinked` events) — the trio references person / worker / organization by `EntityRef` but originates no edges | T-7 |
| No bulk import / export (`bulk_jobs`, the five endpoints, JSONL/CSV codecs) — unlike person / organization / case | T-8 |
| No FHIR surface — **deliberate**, not a gap: no FHIR resource models a plan ([`fhir.md`](../../agents/share/fhir.md) §3 puts portfolio out of scope) | — |
| **No `goals` sub-resource** — FR-12 specifies `GET`/`POST`/`PUT`/`DELETE /api/plans/{pid}/goals` and §9.2 lists them, but no route exists; `goals[]` is reachable only by rewriting the whole plan payload. Found 2026-08-25 while building the OKR engine, which had been specified to anchor key results to a `goal_id` that does not exist — `Goal` carries no identifier at all | FR-12 |
| **No `issues` sub-resource** — FR-14 specifies it and §9.2 / §10.1 list its endpoints and table, but no migration, entity or controller exists. Found 2026-08-25 while building Flow Distribution, which had been specified to derive `defect` from an issue's `kind`. A requirement stated in three sections and built in none is exactly what §14.3 rule 1 is for | FR-14 |
| No duplicate **review queue** table — `check-duplicates` returns candidates, but there is nowhere to persist a pending/confirmed/rejected decision (person / worker / place / thing / organization have one) | T-4 follow-up |
| Automation breadth: no field-change, date-arrival or SLE-breach triggers, and no **multi-action rules** (the schema holds one action per rule — a migration plus an engine change, not a validation tweak) | T-21 |
| SPI / CPI are **permanently unmeasured** — the phased budget baseline they divide by does not exist; NPV, Strategic Alignment Index and defect density likewise each need an input the service does not hold | T-23 |
| Controls: action → **task** conversion landed 2026-09-02 (`POST /api/actions/{pid}/convert`); issue conversion stays undone since there is no `issues` store (`converted_issue_pid` stays reserved, `NULL`, until FR-14 lands). The controls that already exist in all but name (gate readiness, WIP limits, the SLE, retrospectives) are still not registered, so coverage reports only newly-authored controls — investigated and found to need an owner decision (default thresholds for controls whose feedforward verdict can block a write), not a mechanical fix | T-26, PRO-P33 |
| No workflow **edit** route (withdraw and re-register); the withdraw guard checks tasks only | T-15 |
| No front-end routes for the new PM-suite surfaces (`/workflows`, `/plans/[pid]/okr`, `/plans/[pid]/distribution`, TPC, controls, ceremonies, value realization) | T-15 / T-16 / T-20 not-built notes |
| `blocked` carries no reason vocabulary, so the constraint finding stops at "8 days blocked" rather than naming what blocked it | [time-based-analysis.md §17](time-based-analysis.md) |
| `pnpm run lint` (prettier) fails on two pre-existing files (`src/lib/api/client.ts`, `src/lib/i18n.svelte.ts`); no `.svelte` file is prettier-checked at all, since `prettier-plugin-svelte` is not installed | §15 |
| No posts / comments / members sub-resources (deferred from the plan lineage) | §15 |
| Front-end e2e tests are API-stubbed and not wired into CI | §15 |
| No cross-service link **aggregator** (`link-graph-service`) — out of this trio's scope | §15 / [cross-service-linking.md](../../agents/share/cross-service-linking.md) |

### 14.3 Keeping this section honest

This file is a **snapshot with an expiry date**, and the 2026-08-23
correction above is what happens when nobody regenerates one. Two rules
for editing it:

1. **A row claims only what a command proves.** The table at the top
   names the command and its result; a capability row names the module
   or endpoint that backs it. "Implemented" without either is a claim,
   not a status.
2. **Moving a row out of §14.2 and into §14.1 is part of the PR that
   closes it** — the same three-part discipline as a spec change. A gap
   that outlives its fix, or a delivered row that outlives its feature,
   is the failure mode this section already had once.
