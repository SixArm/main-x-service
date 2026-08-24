## 14. Implementation Status

**All three subprojects are implemented and green** (matcher + service +
front-end). Verified 2026-08-23 by running them, not by reading them:

| Check | Result |
|---|---|
| matcher — `cargo test` | 57 unit + 6 integration, 0 failed |
| service — `cargo test` | 236 unit, 0 failed |
| service — `cargo test -- --ignored` vs Postgres 18 | 44 request tests + enforcement + masking + matching, 0 failed |
| service — `cargo clippy --all-targets -- -D warnings` | clean (`#![warn(clippy::pedantic)]`) |
| front-end — `pnpm run check` | 770 files, **0 errors 0 warnings** |
| front-end — `pnpm run test` (vitest) | 75 tests across 9 files, 0 failed |
| front-end — `pnpm run test:e2e` (Playwright, API-stubbed) | 25 tests, 0 failed |

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

### 14.1 Delivered

| Subproject | Capability | Backed by |
|---|---|---|
| (entity) | Canonical specification | This §1–§18 entity spec: the domain model (§5, the canonical home) — the recursive `Plan`, its optional `kind` label, kind-agnostic matching, the matchable/operational partition — the cross-subproject DTO contract, and the family-integration adoptions |
| (entity) | Time-based analysis contract | The cross-cutting [`time-based-analysis.md`](time-based-analysis.md) (§1–§18): the transition log, cycle versus lead time, flow efficiency, first-pass yield, the service level expectation, constraint ranking, and queueing-theory flow |
| matcher | Kind-agnostic `Plan` matching | Name (Jaro-Winkler + Soundex), goal-title & keyword Jaccard, owner-scoped code, owner org, `parent_ref`, timeframe proximity, relationships & tags; deterministic short-circuits on Jira / Asana / Trello / MS-Project / GitHub / Linear ids / URI / UUID, same-owner code, `sameAs` URL. **No kind gate** — `MatchBreakdown.kind_gate_blocked` is vestigial and always `false` |
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
| service | OpenAPI + Swagger | Hand-written `src/openapi.rs`, served at `/api-docs/openapi.json` + `/swagger-ui`, with tests pinning that the documented paths match the mounted routes |
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
| No duplicate **review queue** table — `check-duplicates` returns candidates, but there is nowhere to persist a pending/confirmed/rejected decision (person / worker / place / thing / organization have one) | T-4 follow-up |
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
