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
> Tantivy full-text/fuzzy/phonetic search + Prometheus + OpenAPI/Swagger.
> **Deferred** (spec §13): the operational
> sub-resources (goals / tasks / issues) + derived views, `deduplicate` +
> review queue, cross-service links, bulk import/export.
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
| Plan CRUD | `POST`/`GET` `/plans`, `GET`/`PUT`/`DELETE` `/plans/{pid}`, `GET /plans/search?q=` (Tantivy: `?fuzzy=`, `?phonetic=`, `?kind=`) |
| Match | `POST /plans/match` · `/plans/check-duplicates` (kind-agnostic) |
| Merge | `POST /plans/merge` (`422` equal pids, `404` unknown) · `GET /plans/merges/recent` |
| Strategy (PPM Phase C) | `/ideas` (+ `vote`/`dismiss`/`convert`) · `/scenarios` (+ `/{pid}/evaluate`/`commit`) · `/objectives` (+ `/{pid}/alignment`) · `/plans/{pid}/objectives` · `/plans/{pid}/benefits` (+ `/{b_pid}/realize`) |
| Visibility (PPM Phase B) | `POST`/`GET /dependencies` (+ `DELETE /{pid}`) · `GET /plans/{pid}/schedule` · `/plans/{pid}/milestones` (+ `/{m_pid}/complete`) · `/plans/{pid}/allocations` (+ `DELETE /{a_pid}`) · `GET /capacity` · `/reports` (+ `/{pid}/run?format=json|csv`) · `GET /at-a-glance` (ETag) |
| Governance (PPM Phase A) | `POST`/`GET /proposals` (+ `/{pid}` + `submit`/`review`/`approve`/`reject`/`promote`/`duplicates`) · `/plans/{pid}/gate-reviews` · `/risks` (+ `/{risk_pid}` + `escalate`) · `/budget-lines` (+ `/{line_pid}/actual` · `/{line_pid}/release` — stage-gated tranches) · `GET /plans/{pid}/governance` |
| Executive insights | `GET /executive/{health,decisions,benefits,alignment}` · `/financials/{variance,exposure}` · `/technology/{dependency-risk,radar,debt,flow}` · `/scenarios/compare?a=&b=` (read-only derived views; ETag + `as_of`) |
| Engineering | `POST`/`GET /plans/{pid}/tasks` (+ `PUT`/`PATCH`(move; WIP-limit env caps)/`DELETE /{t_pid}`; story points) · `/plans/{pid}/sprints` (+ `/{s_pid}/notes` retro/feedback + `convert`) · `GET /plans/{pid}/burndown?sprint=` (honest, done_at-only) · `GET /plans/{pid}/velocity` · `GET /plans/{pid}/standup` · `GET /engineering/{blocked,moscow,delivery-links,milestone-calendar}` |
| DevOps | `POST /devops/events` (deploy/incident/recovery ingest) · `GET /devops/metrics` (from ingested events only) · `GET /devops/releases` |
| Oversight areas | `GET /board/{pack,investments,trends}` + `POST /board/snapshots` · `/auditor/{trail,findings,evidence-pack}` · `/compliance/{register,findings}` · `/risk/heatmap` · `/security/register` · `/regulator/extract` (persona gating = ABAC policy config) |
| Collaboration | `POST`/`GET /reviews` (+ `/consensus` · `/{pid}/respond` · `/{pid}/submit` · `DELETE /{pid}`) · `POST /plans/{pid}/tasks/{t_pid}/assign` · `GET /assignees/workload` · `GET /notifications` (+ `/{pid}/read`) |
| Automation | `POST`/`GET /automations` (+ `/{pid}/enable`·`/disable` · `DELETE /{pid}` · `GET /runs`) · `POST`/`GET /scheduled-actions` (+ `/sweep` · `DELETE /{pid}`) |
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
default** — gated by `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`. Deferred (spec §13):
durable event bus Phases 2–3 (outbox +
Fluvio), privacy, front-end merge action, bulk import/export, the
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
├── controllers/collaboration.rs  collaborative review + assignees + notifications
├── controllers/automation.rs PPM automations + runs + scheduled actions + sweep
├── controllers/prioritisation.rs Smart Score + ranked queue + bird's-eye lifecycle
├── controllers/docs.rs       OpenAPI JSON + Swagger UI
├── controllers/metrics.rs    root /metrics.prom Prometheus endpoint
├── metrics.rs                process-wide Prometheus registry (plan counters)
├── auth.rs                   offline PASETO v4.public verification (AuthUser/MaybeAuthUser) via authentication-verifier
├── merge.rs                  pure record-merge logic (any two plans; self-merge rejected)
├── governance.rs · visibility.rs · strategy.rs   pure domain logic for the three phases
├── collaboration.rs          pure review state machine + consensus + assignee workload
├── automation.rs             pure trigger matching + action validation + due-ness
├── prioritisation.rs         the pure, explainable Smart Score (renormalised, self-describing)
├── lifecycle.rs              pure bird's-eye funnel + next-phase readiness checklist
├── scheduler.rs              optional set-and-forget sweep ticker (env-gated, default off)
├── openapi.rs                OpenAPI 3 document
├── relay.rs                  durable-bus Phase 2 outbox relay (poll/ack loop)
├── search/                   Tantivy full-text/fuzzy/phonetic index (index.rs schema + mod.rs engine; kind is a search filter, never a dedup gate)
├── streaming.rs              CRUD/merge event stream — durable Envelope + EventPublisher seam (in-memory default, outbox transport); indexes/deindexes on every write
├── tasks/search.rs           `search_reindex` CLI task + boot-time rebuild-if-empty
├── validation.rs             name + goal-title + identifier + BCP-47 + parent_ref checks → 422
├── models/
│   ├── plans.rs              CRUD helpers over the stored payload
│   ├── governance.rs · visibility.rs · strategy.rs   phase record helpers
│   ├── capabilities.rs       reviews / automations / runs / scheduled actions / notifications
│   ├── event_outbox.rs       durable-bus Phase 2 outbox enqueue + relay poll/ack
│   ├── audit_logs.rs         audit-trail record/query helpers
│   ├── merge_records.rs      merge-history record/query helpers
│   └── _entities/…           SeaORM entities
migration/src/                …_000001_plans, …_000002_audit_logs,
                              …_000003_merge_records, …_000004_event_outbox,
                              …_000005_governance, …_000006_visibility,
                              …_000007_strategy
config/                       development/production/test yaml
```
</content>
