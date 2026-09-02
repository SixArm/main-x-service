# AGENTS.md — Portfolio Service

Entry point for AI coding agents working in the
`project-portfolio-management-service` crate: a registry of **plan**
records — one recursive collection with an optional Portfolio / Project
/ Product / Program / Practice / Process / Purpose / Pathway / Proposal
kind label — **and** a project-management tool.

> Read [`spec/index.md`](./spec/index.md) first — the living spec for this
> crate. The entity-wide contract and canonical `Plan` model live in
> the [portfolio entity spec](../spec/index.md).
>
> **Implemented (MVP, v0.1.0).** The crate builds, `cargo test` is green
> (DB-free unit + matcher-embedding + JSON round-trip; the request suite is
> `#[ignore]`d, needs Postgres), `clippy --all-targets --all-features -- -D
> warnings` is clean, `cargo fmt` is clean, zero `#[allow]`. Scope shipped:
> single-collection plan CRUD + kind-agnostic matching + validation +
> record merge + audit + in-memory events + offline PASETO auth +
> Tantivy full-text/fuzzy/phonetic search + field masking / GDPR export
> (thin — `lead_ref` + owner org only) + Prometheus + OpenAPI/Swagger.
> **Deferred** (spec §13): the operational
> sub-resources (goals / tasks / issues) + derived views, `deduplicate` +
> review queue, cross-service links, bulk import/export.
>
> **Project-management suite (v0.3.0, 2026-08-25/26).** Spec §13
> T-15 … T-27: custom workflows (the task board validates against the
> workflow in force), the OKR engine, project phases (a fourth ordered
> vocabulary — see §1.5.1: lifecycle funnel, gate stage, phase, and the
> task workflow are deliberately uncoupled), Flow Distribution, effort /
> time tracking with per-person **utilisation** (under the
> `time-based-analysis.md` §7.1 obligations — never per-person cycle
> time or throughput), sprint ceremonies with a once-only commitment
> snapshot, Total Project Control (Devaux's DIPP), the controls
> register, and value realization / strategic performance. Nine
> migrations (`m20260825_000001` … `m20260826_000003`); the embedded
> matcher is 0.2 (`Plan.phase`, pinned never-scored). See the API
> surface below and `CHANGELOG.md` 0.3.0.
>
> **Durable event bus, real-broker sink (BUS-3, 2026-08-03).** Following
> BUS-1's case-service reference, `src/relay.rs` gained `FluvioSink` — a
> real-broker `impl EventSink` behind this crate's own `fluvio` Cargo
> feature (off by default; dependency tree and boot behaviour of a
> default build unchanged). `PROJECT_PORTFOLIO_MANAGEMENT_FLUVIO_ENDPOINT`
> selects it over the default `LoggingSink`; set without the `fluvio`
> feature compiled in ⇒ the relay refuses to start (logged `error`), not
> a silent no-broker fallback. `compose.fluvio.yaml` +
> `Dockerfile.fluvio-cli` provision a local broker for opt-in manual
> runs only.
>
> **Persistence note.** All plans live in **one `plans` table** with a
> **nullable `kind`** column (the optional label) and a `parent_pid`
> column (the containment parent). There is one REST collection
> (`/api/plans`); a plan may contain any other plan via `parent_ref` (a
> recursive tree), and matching is not gated by kind.

## What this is

A **loco.rs** service for plan records: CRUD + matching, embedding the
canonical
[`project-portfolio-management-matcher`](../project-portfolio-management-matcher-rust-crate).
The API DTO **is** `project_portfolio_management_matcher::Plan` — stored
verbatim (JSONB) and matched with the same type, so there is no separate
model or adapter to drift (mirrors care-pathway). A `Plan` carries an
**optional** `kind` label (Portfolio / Project / Product / Program /
Practice / Process / Purpose / Pathway / Proposal) for display /
grouping; it does not gate matching and does not fix a collection. All
plans live in one REST collection (`/api/plans`), one `plans` table (see
the persistence note above). Any plan may contain any other plan via a
`parent_ref` to its parent (a recursive tree; a cycle is rejected
`422`). A plan is additionally intended to **own** high-volume
operational sub-resources (goals, tasks, issues) — **excluded** from the
matcher payload (only goal **titles** bridge in) — which are
**deferred** (spec §13).

| Question | Answer |
|---|---|
| Framework | loco.rs 1.0.1 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration` 2.0). |
| Build / test | `cargo build` · `cargo test` (DB-free) · `cargo test -- --ignored` (request tests, need Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `plans` table (`pid`, nullable `kind`, `name`, `data` JSONB `Plan`, `parent_pid`, `active`, soft-delete); + `audit_logs` + `merge_records`. Sub-resource tables + `entity_links` deferred (spec §13). |

## API surface

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

Routes under `/api/`; plans live in one collection at `/api/plans`, and
plan-scoped sub-resources hang off `/api/plans/{pid}/...`. See
[spec §9](./spec/index.md) for the full contract. Highlights:

| Group | Paths |
|---|---|
| Plan CRUD | `POST`/`GET` `/plans`, `GET`/`PUT`/`DELETE` `/plans/{pid}` (record-level ABAC; a `mask`-obligation allow returns the redacted view), `GET /plans/{pid}/masked`, `GET /plans/{pid}/export` (GDPR), `GET /plans/search?q=` (Tantivy: `?fuzzy=`, `?phonetic=`, `?kind=`) |
| Match | `POST /plans/match` · `/plans/check-duplicates` (kind-agnostic) |
| Merge | `POST /plans/merge` (`422` equal pids, `404` unknown) · `GET /plans/merges/recent` |
| Strategy (PPM Phase C) | `/ideas` (+ `vote`/`dismiss`/`convert`) · `/scenarios` (+ `/{pid}/evaluate`/`commit`) · `/objectives` (+ `/{pid}/alignment`) · `/plans/{pid}/objectives` · `/plans/{pid}/benefits` (+ `/{b_pid}/realize`) |
| Visibility (PPM Phase B) | `POST`/`GET /dependencies` (+ `DELETE /{pid}`) · `GET /plans/{pid}/schedule` · `/plans/{pid}/milestones` (+ `/{m_pid}/complete`) · `/plans/{pid}/allocations` (+ `DELETE /{a_pid}`) · `GET /capacity` · `/reports` (+ `/{pid}/run?format=json|csv`) · `GET /at-a-glance` (ETag) |
| Governance (PPM Phase A) | `POST`/`GET /proposals` (+ `/{pid}` + `submit`/`review`/`approve`/`reject`/`promote`/`duplicates`) · `/plans/{pid}/gate-reviews` · `/risks` (+ `/{risk_pid}` + `escalate`) · `/budget-lines` (+ `/{line_pid}/actual` · `/{line_pid}/release` — stage-gated tranches) · `GET /plans/{pid}/governance` |
| Integrity verification | `GET /compliance/records/verify` (recomputes each plan's SHA-256/SHA3-256/HMAC digests) · `GET /compliance/audit/verify` (recomputes each audit row's MAC) — `?limit=`, capped 10 000; `mac_absent` (not a mismatch) when `PORTFOLIO_INTEGRITY_MAC_KEY[_FILE]` is unset |
| Executive insights | `GET /executive/{health,decisions,benefits,alignment}` · `/financials/{variance,exposure}` · `/technology/{dependency-risk,radar,debt,flow}` · `/scenarios/compare?a=&b=` (read-only derived views; ETag + `as_of`) |
| Engineering | `POST`/`GET /plans/{pid}/tasks` (+ `PUT`/`PATCH`(move; WIP-limit env caps)/`DELETE /{t_pid}`; story points) · `/plans/{pid}/sprints` (+ `/{s_pid}/notes` retro/feedback + `convert`) · `GET /plans/{pid}/burndown?sprint=` (honest, done_at-only) · `GET /plans/{pid}/velocity` · `GET /plans/{pid}/standup` · `GET /engineering/{blocked,moscow,delivery-links,milestone-calendar}` |
| Time-based analysis | `GET /plans/{pid}/{time-analysis,constraints,aging-wip,flow,cumulative-flow,forecast,rollup}` · `GET /plans/{pid}/tasks/{t_pid}/{transitions,time-analysis}` · `GET /flow-classes` — cycle vs lead time, flow efficiency, rework/first-pass yield, the service level expectation from the plan's own history, aging WIP, and Little's Law. Read-only: transitions are written by the task create/move calls, in-transaction. `forecast` is Monte-Carlo over the plan's **throughput** history (not its cycle times), deterministic by seed |
| Workflows | `POST`/`GET /workflows` · `DELETE /workflows/{pid}` · `GET /plans/{pid}/workflow` — per-plan status vocabularies (resolution: plan's own, else deployment default, else built-in; empty transition set = unconstrained). The task create/move paths validate against the workflow in force |
| Phases | `PUT /plans/{pid}/phase` (one-step advance; a skip is `422` naming the skipped phase; a backward move needs a reason) · `GET /plans/{pid}/phase-history` (append-only; `DELETE` is `405`). The `phase` payload field and `plans.phase` column agree by invariant |
| OKR | `POST`/`GET /objectives/{pid}/key-results` · `POST`/`GET /key-results/{pid}/check-ins` · `GET /plans/{pid}/okr` — key results anchor to `objectives` (never `goals[]`, which carry no id); the plan score is weighted by the existing `objective_links` weight |
| Flow Distribution | `GET /plans/{pid}/flow-distribution` — the feature/defect/risk/tech-debt mix over **declared** `tasks.flow_type`, with the subtree rollup; `unclassified` stands alone and is not storable |
| Effort / utilisation | `POST`/`GET /plans/{pid}/time-entries` · `GET /plans/{pid}/effort` (per plan/task/assignee; labelled asserted) · `POST /working-time` · `POST /non-working` · `GET /capacity/utilization?by=plan\|team\|person` (leave leaves the denominator — `null` with a reason, never 0%) |
| Ceremonies | `POST`/`GET /sprints/{pid}/ceremonies` (planning/daily/review; retro stays `sprint_notes`) · `POST /sprints/{pid}/commit` (once-only commitment snapshot) · `GET /sprints/{pid}/commitment` (names scope added/removed afterwards) |
| Total Project Control | `POST`/`GET /plans/{pid}/tpc` · `GET /plans/{pid}/tpc/report` (DIPP, band, progress index, stored-vs-computed divergence) · `GET /tpc?currency=` (triage, highest DIPP first; foreign-currency and undefined entries set aside, never ranked as zero) |
| Controls | `POST`/`GET /plans/{pid}/controls` · `GET /plans/{pid}/controls/coverage` · `GET /controls/coverage` · `DELETE /controls/{pid}` · `POST`/`GET /controls/{pid}/readings` · `POST /readings/{pid}/actions` · `POST /actions/{pid}/convert` — feedforward may block, feedback may only record; `unmeasured` is a real third verdict, never a pass; `convert` turns an action into a task on its own plan (no issue conversion yet — no `issues` store) |
| Value realization | `POST /plans/{pid}/{business-case,value-points,adoption,satisfaction}` · `GET /plans/{pid}/value-realization` (Time-to-Value as a p50/p85 distribution; single-currency ROI or withheld) · `GET /plans/{pid}/performance` (NPS with response count; SPI/CPI `null` with `no_baseline`, never `1.0`) |
| DevOps | `POST /devops/events` (deploy/incident/recovery ingest) · `GET /devops/metrics` (from ingested events only) · `GET /devops/releases` |
| Oversight areas | `GET /board/{pack,investments,trends}` + `POST /board/snapshots` · `/auditor/{trail,findings,evidence-pack}` · `/compliance/{register,findings}` · `/risk/heatmap` · `/security/register` · `/regulator/extract` (persona gating = ABAC policy config) |
| Collaboration | `POST`/`GET /reviews` (+ `/consensus` · `/{pid}/respond` · `/{pid}/submit` · `DELETE /{pid}`) · `POST /plans/{pid}/tasks/{t_pid}/assign` · `GET /assignees/workload` · `GET /notifications` (+ `/{pid}/read`) |
| Automation | `POST`/`GET /automations` (+ `/{pid}/enable`·`/disable` · `DELETE /{pid}` · `GET /runs`) · `POST`/`GET /scheduled-actions` (+ `/sweep` · `DELETE /{pid}`) · `POST /automations/milestones/sweep` (claim-based `milestone_due` date-arrival trigger; a rule/milestone pair fires exactly once, ever) |
| Prioritisation | `GET /plans/{pid}/smart-score` · `/prioritisation` · `/lifecycle` · `/plans/{pid}/lifecycle` (derived; ETag + `as_of`) |
| Audit / events | `GET /plans/audit/recent` · `/plans/{pid}/audit` · `/plans/events/recent` |
| Auth | `GET /plans/whoami` (`401` without a valid token) |
| Docs / metrics | `GET /api-docs/openapi.json` · `/swagger-ui` · `/metrics.prom` |

Plus loco's default `/_health`, `/_ping`. Every CRUD action (plan
and phase record) writes an `audit_logs` row and publishes a
`created`/`updated`/`deleted` (and `merged`) event.
**Matching is kind-agnostic** — there is no kind gate, so any two plans
may match regardless of their optional `kind` labels.

**Deferred endpoints (spec §13 — specified, not yet wired):**

- `POST /plans/deduplicate` — batch scan → review queue
- `/plans/{pid}/{goals,issues}` — remaining operational sub-resource CRUD (tasks landed 2026-07-20)
- `GET /plans/{pid}/timeline` — derived view (burndown landed 2026-07-20)
- `POST`/`GET`/`DELETE /plans/{pid}/links` — cross-service entity
  links (would emit `linked`/`unlinked`)

## MVP scope

CRUD + Tantivy full-text/fuzzy/phonetic search (`src/search/`; replaces
the earlier `ILIKE` name search) + matching (embed `project-portfolio-management-matcher`,
`MatchingEngine::new(MatchConfig::default())`) over one recursive `plans`
collection, real-time create duplicate detection (`409`,
kind-agnostic), record merge, payload validation (`src/validation.rs`: UUID /
PM-tool-id / URI identifier shapes; non-blank goal titles; BCP-47
`in_language`; `parent_ref` UUID + containment-cycle check), OpenAPI 3 + Swagger UI, an
audit log + in-memory event stream (durable-bus Phase 1 — see
`agents/share/event-bus.md`), offline PASETO v4.public verification
(`src/auth.rs`, embeds `authentication-verifier`; `/whoami` + audit
`actor`; boot-time published-key fetch when `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL`
is set — fetched key set wins, env `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS` fallback, the
service always boots; spec §13, done 2026-07-04), and blanket `/api/*`
auth enforcement wired but **off by
default** — gated by `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`. **Privacy**
(`src/privacy.rs`) provides field masking — `lead_ref` (the plan lead, a
`person:`/`worker:` ref) dropped entirely, `owner_org_id`/`owner_org_name`
masked to their tail — the always-masked `/masked` view, and the
audited GDPR `/export`, wired to the ABAC `mask` obligation on
`GET /{pid}`. Deliberately the thinnest of the four privacy modules in
the family: most of a `Plan` (name, code, goals, status, dates,
identifiers, tags, `relationships`, `parent_ref`) is operational
content the registry exists to serve up, not personal data. The durable
event bus's Phases 2–3 (outbox relay + retention, then the `FluvioSink`
real-broker sink, BUS-3, 2026-08-03, feature-gated and off by default)
have landed. Deferred (spec §13):
front-end merge action, bulk import/export, the
`posts` / `comments` / `members` collaboration
sub-resources, gRPC.

> Auth model (intended): the human session is a server-side cookie
> session; peers verify a short-lived **PASETO v4.public** token offline
> against the auth-service's published **Ed25519 key** (replacing RS256
> JWT + JWKS). Front-ends use a BFF (no browser token). Source of truth:
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
> (RS256/JWKS not used).

## Golden rules

1. **Spec-first.** Update `spec/index.md` with behavioural changes; the
   entity-wide [portfolio spec](../spec/index.md) owns the canonical model
   (§5).
2. **Loco-idiomatic.** Endpoints are loco controllers in `app.rs`; new
   tables are `sea-orm-migration` migrations.
3. **Reuse the matcher type.** Do not fork a `Plan` DTO.
4. **One recursive collection.** All plans live in one `/api/plans`
   collection backed by one `plans` table; do not reintroduce per-kind
   collections. `kind` is an **optional** Portfolio / Project / Product /
   Program / Practice / Process / Purpose / Pathway / Proposal label
   (display / grouping), never a discriminator that fixes a
   collection.
5. **Kind-agnostic matching.** There is no kind gate — any two plans may
   match, and dedup / check-duplicates / merge are not scoped by kind.
   Containment is expressed with `parent_ref` (any plan may contain any
   other; a self- or descendant-cycle is rejected `422`). This extends to
   search: `src/search/` indexes `kind` so `GET /plans/search?kind=` can
   **narrow** a query — an opt-in the caller requests — but
   `SearchEngine::candidates` (the `check-duplicates` blocking query)
   never filters on it. Do not add a kind filter to `candidates`.
6. **Partition rule.** Operational sub-resources and cross-service
   `entity_links` are **never** fed to the matcher; only the thin
   `Plan` payload is (goal titles bridge via `data.goals[]`).
   Within-payload `relationships` **are** a matcher signal.
7. **Auth credentials** come from the central
   [authentication-service](../../authentication/authentication-service-with-loco)
   (cookie session for humans; offline PASETO v4.public for peers).

## Layout

```
src/
├── app.rs                    loco Hooks (routes, truncate)
├── bin/main.rs               loco CLI entrypoint
├── controllers/plans.rs      plan CRUD + match + check-duplicates + merge + audit/events + whoami + parent_ref cycle check
├── controllers/governance.rs PPM Phase A: proposals, gate reviews, risks, budget lines
├── controllers/visibility.rs PPM Phase B: dependencies, schedule, milestones, allocations, capacity, reports, at-a-glance
├── controllers/strategy.rs   PPM Phase C: ideas, scenarios, objectives, benefits
├── controllers/insights.rs   executive insight areas: health/decisions/benefits/alignment, financials, technology
├── controllers/oversight.rs  board/auditor/compliance-register/risk-heatmap/security/regulator views
├── controllers/engineering.rs tasks board (+ move/story points/WIP limits), sprints, burndown, velocity, standup, DevOps events/metrics/releases, estate views
├── controllers/tba.rs        time-based analysis reads: per-task and plan flow, constraints, aging WIP, Little's Law
├── controllers/workflow.rs   custom workflow registry + the per-plan resolved vocabulary
├── controllers/phase.rs      project phase set (one-step / reasoned regression) + append-only history
├── controllers/okr.rs        key results + check-ins + the alignment-weighted per-plan OKR view
├── controllers/distribution.rs  Flow Distribution over declared tasks.flow_type + subtree rollup
├── controllers/effort.rs     time entries, effort roll-ups, working time, non-working periods, utilisation
├── controllers/ceremony.rs   sprint ceremonies + the once-only commitment snapshot
├── controllers/tpc.rs        Total Project Control: observations, derived report, portfolio triage
├── controllers/controls.rs   controls register: standards, readings, actions, coverage
├── controllers/value.rs      business-case targets, value points, adoption, satisfaction, performance
├── controllers/collaboration.rs  collaborative review + assignees + notifications
├── controllers/automation.rs PPM automations + runs + scheduled actions + sweep + milestone-due sweep (claim-based)
├── controllers/prioritisation.rs Smart Score + ranked queue + bird's-eye lifecycle
├── controllers/compliance.rs `/api/compliance/{records,audit}/verify` — row-level integrity verification
├── controllers/docs.rs       OpenAPI JSON + Swagger UI
├── controllers/metrics.rs    root /metrics.prom Prometheus endpoint
├── metrics.rs                process-wide Prometheus registry (plan counters)
├── auth.rs                   offline PASETO v4.public verification (AuthUser/MaybeAuthUser) via authentication-verifier
├── version.rs                `Accepts-version` header negotiation middleware (agents/share/api-versioning.md)
├── merge.rs                  pure record-merge logic (any two plans; self-merge rejected)
├── governance.rs · visibility.rs · strategy.rs   pure domain logic for the three phases
├── insights.rs               pure derivations behind the executive insight views (no I/O)
├── engineering.rs            pure rules: task statuses, honest burndown, MoSCoW bands, milestone kinds
├── tba.rs                    pure time-based analysis over the task transition log: interval
│                             derivation, cycle vs lead time, VA/NNVA/UNVA splits, rework and
│                             rolled first-pass yield, nearest-rank percentiles, the service
│                             level expectation, constraint ranking, Little's Law. No I/O
├── flow_metrics.rs           default-off `ppm_flow_*` Prometheus gauge refresh loop (TBA-10; env-gated, capped, small-board-suppressed)
├── workflow.rs               pure workflow resolution (plan → default → built-in) + category-derived flow classes
├── phase.rs                  pure one-step phase advancement, reasoned regression, per-phase durations
├── okr.rs                    pure key-result progress + the objective-link-weighted plan score
├── distribution.rs           pure Flow Distribution shares + range-guarded intended-mix gaps
├── effort.rs                 pure effort roll-ups + integer capacity/utilisation arithmetic (leave leaves the denominator)
├── tpc.rs                    pure DIPP / progress index / banding / triage — minor units + basis points, no float, undefined ≠ zero
├── controls.rs               pure control timings, comparators, verdict derivation, coverage rollup
├── value.rs                  pure value realization: time-to-value distribution, single-currency ROI, NPS, SPI/CPI refusals
├── snapshots.rs               point-in-time estate snapshots behind the board/CRO trend views (explicit capture or the optional ticker)
├── collaboration.rs          pure review state machine + consensus + assignee workload
├── automation.rs             pure trigger matching + action validation + due-ness
├── prioritisation.rs         the pure, explainable Smart Score (renormalised, self-describing)
├── lifecycle.rs              pure bird's-eye funnel + next-phase readiness checklist
├── scheduler.rs              optional set-and-forget sweep ticker (env-gated, default off; sweeps scheduled actions and milestone-due rules)
├── openapi.rs                OpenAPI 3 document
├── privacy.rs                field masking (lead_ref, owner org) + GDPR export envelope
├── compliance/                keyed integrity: mac.rs (integrity-mac binding) + record_integrity.rs (plans) + audit_integrity.rs (audit_logs); default off without PORTFOLIO_INTEGRITY_MAC_KEY[_FILE]
├── relay.rs                  durable-bus Phase 2/3 outbox relay (poll/ack loop) + FluvioSink (fluvio feature, BUS-3)
├── search/                   Tantivy full-text/fuzzy/phonetic index (index.rs schema + mod.rs engine; kind is a search filter, never a dedup gate)
├── streaming.rs              CRUD/merge event stream — durable Envelope + EventPublisher seam (in-memory default, outbox transport); indexes/deindexes on every write
├── tasks/search.rs           `search_reindex` CLI task + boot-time rebuild-if-empty
├── validation.rs             name + goal-title + identifier + BCP-47 + parent_ref checks → 422
├── initializers/              empty (loco extension point, reserved)
├── workers/downloader.rs      loco worker-queue scaffold, carried over unwired (not real work; kept so the queue has a concrete worker to register)
├── models/
│   ├── plans.rs              CRUD helpers over the stored payload
│   ├── governance.rs · visibility.rs · strategy.rs   phase record helpers
│   ├── capabilities.rs       reviews / automations / runs / scheduled actions / notifications
│   ├── event_outbox.rs       durable-bus Phase 2 outbox enqueue + relay poll/ack
│   ├── audit_logs.rs         audit-trail record/query helpers
│   ├── merge_records.rs      merge-history record/query helpers
│   └── _entities/…           SeaORM entities
├── observability.rs          structured logging + real OpenTelemetry OTLP export (PRO-H12 slice 7, the last — see below)
migration/src/                …_000001_plans, …_000002_audit_logs,
                              …_000003_merge_records, …_000004_event_outbox,
                              …_000005_governance, …_000006_visibility,
                              …_000007_strategy, m20260719_000002_insight_columns,
                              m20260719_000003_insight_snapshots,
                              m20260720_000001_engineering,
                              m20260720_000002_engineering_moderate,
                              m20260722_000001_capabilities,
                              m20260728_000001_integrity_digests,
                              m20260823_000001_time_based_analysis,
                              m20260825_000001_total_project_control,
                              m20260825_000002_controls,
                              m20260825_000003_phase_transitions,
                              m20260825_000004_flow_type,
                              m20260825_000005_workflows,
                              m20260825_000006_key_results,
                              m20260826_000001_effort,
                              m20260826_000002_ceremonies,
                              m20260826_000003_value,
                              m20260902_000001_automation_multi_action,
                              m20260902_000002_automation_milestone_fires
config/                       development/production/test yaml
tests/otlp_export.rs          real OTLP/gRPC export proof, in-process collector, no database
tests/otlp_middleware.rs      the mounted `trace_mw` layer proved end to end over a real HTTP request
tests/otlp_collector/         the shared in-process OTLP/gRPC collector both otlp_* binaries use
```

## OpenTelemetry OTLP export

`src/observability.rs` (repo `tasks.md` PRO-H12 slice 7 of 7 — the
last, landed 2026-09-02) is a close port of case-service's
`src/observability.rs` — itself a port of care-pathway's, itself
organization's, itself course's, itself person's, itself
link-graph-service's, the family's first working exporter. This crate
carried **no** `src/observability` module at all before this change,
and is the **fourth and last of the four loco-idiomatic registries**
(organization, care-pathway, case, portfolio — `src/controllers/`, not
`src/api/rest/`) to carry it — the final slice of PRO-H12.
`App::init_logger` installs it (loco's own `EnvFilter` + formatted
layer, plus the `tracing-opentelemetry` bridge over an OTLP/gRPC
exporter); `App::on_shutdown` flushes it. Export is **on by default** —
set `OTLP_ENDPOINT=""` to disable it — at `OTLP_ENDPOINT` (default
`http://localhost:4317`) with `service.name` from `OTLP_SERVICE_NAME`
(default `project-portfolio-management-service`); both variables are
**deliberately unprefixed**, matching every other crate that carries
this pipeline, not the per-service
`PROJECT_PORTFOLIO_MANAGEMENT_*` convention
`PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` and its siblings use.

**Where this crate's shape forced real adaptation**, confirmed rather
than assumed:

- **Exactly one router-construction surface**, unlike the person-style
  crates' two. This crate is genuinely loco-idiomatic: `App::routes` +
  `App::after_routes` in `src/app.rs` is the only place a router gets
  built — confirmed by grepping `src/` and `tests/` for a second
  `Router::new()`/`create_router`: the one hit (`src/auth.rs`) is a
  unit test for the auth middleware itself, not an app-level router.
  `observability::trace_mw` is therefore layered **once**, as the
  outermost middleware in `after_routes` — the same precedent
  `require_auth_mw` and `require_version_mw` already set by being
  layered there, and the fourth of four loco-idiomatic registries to
  confirm the identical shape.
- **No `tonic` rename needed** — this crate declares no `tonic`
  dependency of its own (no gRPC stub — `agents/share/overview.md`'s
  capability matrix), so the in-process OTLP collector tests' `tonic
  0.14` dev-dependency is a plain, un-renamed dependency, exactly as
  the other three loco-idiomatic ports' were.
- **No SOUP-register step** — unlike care-pathway and case, this crate
  carries no IEC 62304 SOUP register, so this was the simplest of the
  four loco-idiomatic ports: no `compliance/soup.tsv` bookkeeping at
  all.

`tests/otlp_export.rs` and `tests/otlp_middleware.rs` (ported from
case-service, with `tests/otlp_collector/` — an in-process OTLP/gRPC
collector, unchanged) prove real export against a real gRPC listener
in a normal `cargo test` run: a `tracing` span and a metric both reach
the collector's decoded protobuf, and a served HTTP request returns a
`traceparent` whose trace id matches the exported span's. None of this
needs a database. Landing this raised `cargo test --lib` from 353 to
361 (8 new `src/observability.rs` unit tests), plus 4 new tests across
the two `tests/otlp_*.rs` binaries. Verified independently: `cargo fmt
--check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny
check`, `cargo bench --no-run`, and the MSRV check (`cargo +1.96 check
--all-targets`) all clean.

**This closes repo `tasks.md` PRO-H12**: every entity registry in the
family — all ten, plus the cross-cutting link-graph-service — now
exports real OpenTelemetry OTLP traces and metrics.

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`,
`project-portfolio-management-matcher`) live outside
`project-portfolio-management/project-portfolio-management-service-with-loco/`:

```sh
podman build \
  -f project-portfolio-management/project-portfolio-management-service-with-loco/Dockerfile \
  -t project-portfolio-management-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /_health` returns `200`. This exercise found and
fixed a real bug: `config/production.yaml`'s `mailer.smtp.auth.user`/
`password` used an unquoted Tera `{{ get_env(name="…", default="") }}`
call, which renders as YAML `null` (not `""`) when the env var is
unset — loco's `SmtpAuth` fields are `String`, not `Option<String>`, so
this failed config parsing at boot with "invalid type: unit value,
expected a string". This crate's `.gitignore` also excluded
`config/production.yaml` entirely (a loco scaffold default nobody had
removed), which is why the bug had never been caught — the file never
left this machine, so no other checkout could exercise it. Both are
fixed (the file is now tracked; see the `.gitignore` for the
reasoning). See `.containerignore` at the repository root (excludes
every crate's `target/`, or the build context would try to copy
hundreds of GB of build artifacts). The wired multi-service
`examples/compose/` stacks (DEP-1) that build on this are not yet
written.
