# Portfolio Service — Specification

> **Single source of truth** for this crate's internals. Code conforms
> to this spec. Behavioural change = spec + code + test in one PR. Live
> work queue is §13.
>
> Entity-wide contract: [portfolio entity spec](../../spec/index.md) (the API
> DTO **is** the matcher's `Plan` type; canonical domain model in its §5).
> Sibling matcher: [project-portfolio-management-matcher](../../project-portfolio-management-matcher-rust-crate/spec/index.md).
> Sibling front-end: [project-portfolio-management-front-end-with-svelte](../../project-portfolio-management-front-end-with-svelte/spec/index.md).

## 1. Purpose and vision

A registry of **plan** records for the Main X Index family — and a
project-management tool. A *plan* is a matchable identity in one
recursive collection; its `kind` — **Portfolio**, **Project**,
**Product**, **Program**, **Practice**, **Process**, **Purpose**,
**Pathway**, or **Proposal** — is an **optional descriptive label** (a
display / grouping hint), not a required discriminator and not a separate
collection. The service has **two faces that share one record**: a
deduplicated, matchable identity registry (the thin `Plan` payload,
scored by the canonical project-portfolio-management-matcher) and a
project workspace (each plan *owns* operational sub-resources — goals,
tasks, issues — plus derived timeline / burndown views). All plans live
in **one collection and table**; any plan may contain any other plan via
a `parent_ref` to its parent (a recursive tree). Built on loco.rs.

## 2. Scope

MVP: CRUD + Tantivy full-text/fuzzy/phonetic search + matching + record merge + audit log +
in-memory event streaming (durable-bus Phase 1) over **one recursive
`plans` collection** + the operational sub-resources (goals, tasks,
issues) on any plan + derived timeline / burndown read views +
cross-service entity links (write side) + collaborative review +
workflow automation + the set-and-forget scheduler + the Smart Score /
bird's-eye lifecycle views (§9.4a) + OpenAPI/Swagger + Prometheus
metrics + offline PASETO v4 public token verification (Ed25519,
published key) + blanket `/api/*` auth enforcement (off by default) +
payload validation. Matching is **kind-agnostic** — any two plans may
match; the `kind` label does not gate matching (§5/§9.2). `kind` is an
optional Portfolio / Project / Product / Program / Practice / Process /
Purpose / Pathway / Proposal label, and any plan may contain any other
plan via `parent_ref` (a self- or descendant-cycle is rejected `422`).
Tantivy full-text/fuzzy/phonetic search (§9.1) and search-blocked
`check-duplicates` candidates — landed. Field masking (`lead_ref`
dropped, `owner_org_id`/`owner_org_name` masked to their tail) + the
masked view + audited GDPR export, wired to the ABAC `mask` obligation
— landed (§13 2026-08-02); the thinnest privacy module in the family,
since most of a `Plan` is operational content, not personal data.
Beyond the MVP, the crate is also a **full project-management tool**:
the PPM governance / visibility / strategy phases (§9.9–§9.11), the
executive insight and oversight views (§9.12–§9.13), the
engineering-team core — task board, sprints, burndown, velocity,
standup, DevOps ingest (§9.14) — and row-level integrity verification
over `plans`/`audit_logs` (§9.15) have all landed.
Deferred (§13): the goals/issues
sub-resource tables + the derived
timeline view (tasks + sprints + burndown landed 2026-07-20),
front-end merge action, bulk import/export, gRPC, and
the deferred `posts` / `comments` / `members` collaboration
sub-resources. (The durable event bus's Phase 2 outbox + Phase 3
relay/retention, then its `FluvioSink` real-broker sink landed
2026-08-03, feature-gated and off by default — §13. The
paseto-keys-over-HTTP fetch at boot landed
2026-07-04 — `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL`, §9.6/§13.)
Token **issuance** is out of scope — provided by the central
authentication-service. Auth source of truth (supersedes the RS256-JWT
model):
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).

## 3. Stakeholders and users

Portfolio / programme managers and delivery teams curating plans and
working the boards; peer services (person / worker / organization /
authentication, and the cross-service link aggregator); the portfolio
front-end.

## 4. Glossary

- **plan** — the matchable identity record. The canonical Rust type is
  `Plan`, with an **optional** `kind` label (`Plan::new(name)` leaves
  `kind` `None`).
- **kind** — an optional `Portfolio` | `Project` | `Product` |
  `Program` | `Practice` | `Process` | `Purpose` | `Pathway` |
  `Proposal` descriptive / grouping label; it does **not** fix a
  collection and does **not** gate matching.
- **portfolio** — a plan whose `kind` label is `Portfolio`;
  conventionally an umbrella under which other plans sit, but only by
  `parent_ref`, not by a separate collection.
- **pid** — public UUID of a plan record.
- **data** — the full `Plan` payload stored as JSONB.
- **parent_ref** — the parent plan `pid` a plan may carry (the
  containment link); any plan may contain any other (a recursive tree);
  optional.
- **sub-resource** — operational data a plan owns (goal, task, issue),
  in its own table keyed by the parent plan `pid`; **not** part of the
  matcher payload (except goal **titles** via `data.goals[]`).
- **EntityRef** — `<entity_type>:<uuid>` URN naming a record in another
  service (lead / assignee / owner org).
- **derived view** — a read-only projection (timeline / burndown)
  computed from goals and tasks; never persisted as its own row.

## 5. Domain model

The API DTO is `project_portfolio_management_matcher::Plan` — the canonical model lives
in the [portfolio entity spec §5](../../spec/index.md). Matchable payload
(thin identity): optional `kind` label, `name`, `alternate_names`, `code`,
`owner_org_id`, `owner_org_name`, `lead_ref`, `parent_ref`, `status`,
`goals` (titles + optional target dates), `start_date`, `target_date`,
`keywords`, `tags`, `identifiers`, `same_as`, `in_language`,
`relationships`. **The high-volume operational data is deliberately
excluded from the payload** — tasks and issues live in their own tables
(§10) and are never fed to the matcher; goals are the one bridge (their
titles feed matching via `data.goals[]`).

> **Matching is kind-agnostic.** There is no kind gate: two plans may
> match regardless of their (optional) `kind` labels
> ([project-portfolio-management-matcher §1–§25](../../project-portfolio-management-matcher-rust-crate/spec/index.md)).
> The matcher's `MatchBreakdown.kind_gate_blocked` remains only as a
> vestigial, always-`false` field. Containment is expressed with
> `parent_ref`, not with a collection boundary.

> **Partition rule — within-payload relationships vs cross-service
> links.** `relationships` inside the `Plan` payload are within-entity
> and **are** a matcher signal (Jaccard component). Cross-service links
> (§9.7) live only in `entity_links`, the `linked`/`unlinked` events, and
> the aggregator — they are **never** stored in the payload and **never**
> fed to the matcher. See
> [cross-service linking §7](../../../agents/share/cross-service-linking.md).

## 6. Functional requirements

All plans live in **one collection** — `/api/plans` — with an identical
controller shape for every plan. A plan carries an **optional** `kind`
label (Portfolio / Project / Product / Program / Practice / Process /
Purpose / Pathway / Proposal) and an optional `parent_ref` → parent plan
pid; any plan may contain any other (a recursive tree), so containment
is expressed by `parent_ref`, never by a collection boundary.

1. `POST /api/plans` — create; `kind` is an **optional** label (accepted
   as sent, never server-stamped and never mismatched against a
   collection), `name` required, `identifiers` structurally checked per
   scheme (canonical UUID for `Uuid`; external PM-tool id shapes for
   `JiraProjectKey` / `AsanaGid` / `TrelloBoardId` / `MsProjectId` /
   `GitHubProjectId` / `LinearId`; URI shape for `Uri`; non-blank for
   the rest), `goals` titles non-blank, and `in_language` checked for
   BCP-47 syntax; when present, `parent_ref` must be a well-formed UUID
   **and** must not introduce a containment cycle (a plan may not be its
   own ancestor); `422` on any problem, **all reported together** — also
   enforced on update. Real-time duplicate detection on create returns
   `409 Conflict` with candidate matches (kind-agnostic) when duplicates
   are found. Rules in [`src/validation.rs`](../src/validation.rs).
2. `GET /api/plans` — list active (cap 100), `{pid, name}`. `GET
   /api/plans?parent={pid}` — roll up a plan's **direct children** (by
   denormalised `parent_pid`). `GET /api/plans/search?q=` —
   Tantivy full-text search over name, alternate names, owner org name,
   identifiers, keywords, tags, and goal titles (`?fuzzy=true` for typo
   tolerance, `?phonetic=true` for Soundex, `?kind=` to narrow to one
   exact kind label — an opt-in filter, never a matching gate; blank `q`
   → `400`, an unavailable index → `503`).
3. `GET /api/plans/{pid}` — return the stored `Plan`.
4. `PUT /api/plans/{pid}` — replace the payload (`422` if `name` is
   blank, or any `goals` / `identifiers` / `in_language` entry is
   malformed, or `parent_ref` is malformed or would form a cycle).
5. `DELETE /api/plans/{pid}` — soft-delete.
6. `POST /api/plans/match` — rank an explicit `{query, candidates}` set
   (no persistence). Candidates are scored regardless of their
   (optional) `kind` labels — there is no kind gate.
7. `POST /api/plans/check-duplicates` — match a query against stored
   active plans; return those above threshold, ranked. `POST
   /api/plans/deduplicate` — batch scan of active plans into the review
   queue (status `Pending`/`Confirmed`/`Rejected`/`AutoMerged`).
8. `POST /api/plans/merge` — fold a duplicate into **any** survivor
   (union payload fields, former-name alias, soft-delete the duplicate,
   `merge_records` history + transferred snapshot, `Replaces` link from
   survivor → duplicate, `Merged` event); `422` equal pids, `404`
   unknown. Merge is **not** scoped by kind or collection — any two
   plans may merge. `GET /api/plans/merges/recent` — merge history.
9. Operational sub-resource CRUD on **any** plan (separate tables keyed
   by the parent plan `pid`):
- **Goals** — `…/plans/{pid}/goals` (also part of the matcher payload
  via `data.goals[]` mutation — the goals bridge).
- **Tasks** — `…/plans/{pid}/tasks` (`status` Todo/InProgress/
  InReview/Done/Blocked; `assignee_ref` EntityRef; `goal_id?`,
  `parent_task_id?`, `estimate`, `remaining`, `due_date`).
- **Issues** — `…/plans/{pid}/issues` (`kind` Bug/Risk/Blocker/
  Question/Improvement; `severity` Low/Med/High/Critical; `status`
  Open/InProgress/Resolved/Closed; `assignee_ref`). Full CRUD on each;
  every write audits + emits a `created`/`updated`/ `deleted` event
  scoped to the sub-resource and its parent plan. (Deferred §13: `posts`
  / `comments` / `members` collaboration sub-resources.)
10. Derived read views — `GET /api/plans/{pid}/timeline`
   (goals-with-target-date milestones + task date ranges → Gantt) and
   `GET /api/plans/{pid}/burndown` (remaining-vs-estimate over time from
   task estimate/remaining snapshots). Read-only projections; never
   persisted as their own row.
11. Cross-service links — `POST`/`GET`/`DELETE /api/plans/{pid}/links`
   (§9.7), emitting `linked` / `unlinked`.
12. `GET /api/plans/audit/recent` + `/{pid}/audit` — audit-log query;
   `GET /api/plans/events/recent` — in-memory event stream. Each
   create/update/delete/merge (plan and sub-resource) writes an
   `audit_logs` row and publishes a `created`/`updated`/
   `deleted`/`merged` (and `linked`/`unlinked`) event.
13. `GET /api/plans/whoami` — echo verified bearer-token claims (`401`
   without a valid token); proves offline PASETO verification.
14. `GET /api-docs/openapi.json` + `GET /swagger-ui` — OpenAPI 3
   document and a Swagger UI page rendering it.
15. `GET /metrics.prom` — Prometheus metrics in text-exposition format
   (`Content-Type: text/plain; version=0.0.4`), mounted at the root (not
   under `/api`) and public under blanket auth enforcement so a scraper
   needs no token. Exposes plan CRUD/merge counters
   (`portfolio_plan_created_total` / `_updated_total` / `_deleted_total`
   / `_merged_total`) plus `http_requests_total`.
16. Bulk import/export (deferred, §13) — async, job-based, on the loco
   `bg_pg` worker: the five endpoints on the plans collection (§9.8).
   The uniform family contract (execution model, JSONL/CSV/Parquet
   codecs, upsert-by-stable-key + dedupe-to-review, per-row error
   report, export masking + audit) is fixed in
   [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md);
   portfolio-specific bits are in §9.8 / §10.

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic + probabilistic
matching via the embedded `project-portfolio-management-matcher` (`MatchingEngine::new(
MatchConfig::default())` in
[`src/controllers/plans.rs`](../src/controllers/plans.rs));
soft-delete with audit-friendly timestamps. One shared controller core
serves the single plans collection (`kind` is an optional label, not a
route discriminator), so there is no per-kind drift. Sub-resource tables
are sized for high-volume operational churn and are kept off the matcher
hot path (the matcher only ever sees the thin JSONB payload). The dedup
scan ranges over the plans collection's active rows with no kind gate.

## 8. Architecture

loco `App` (`src/app.rs`) registers the single plan-collection controller
(one shared core) and the sub-resource controllers. One `plans` table
stores `pid` + denormalised `name` + the full `Plan` JSONB `data` + a
**nullable** `kind` column (the optional label) + a denormalised
`parent_pid` UUID (from `data.parent_ref`, for child roll-up queries);
the operational sub-resources live in their own tables keyed by the
parent plan `pid` (§10). Matching calls `project-portfolio-management-matcher` directly
on the deserialised payloads — **no adapter**, mirroring care-pathway —
with **no kind gate**, so any two plans may match. Cross-service
links use the hybrid topology
([cross-service linking §4](../../../agents/share/cross-service-linking.md)):
the service writes its outbound edges locally and emits events; a
standalone aggregator builds the queryable graph.

## 9. API surface

API URLs are version-free; a client selects the representation version
with the `Accepts-version` request header (default `1.0`) — see
[`agents/share/api-versioning.md`](../../../agents/share/api-versioning.md).

Raw loco JSON under `/api/`. All plans live at `/api/plans`. `404` for
unknown `pid`; `422` for a validation failure (blank `name`, a malformed
`goals` / `identifiers` / `in_language` entry, or a malformed / cyclic
`parent_ref` — family convention, via `Error::CustomError(
StatusCode::UNPROCESSABLE_ENTITY, …)`, every problem in one body); `400`
for a malformed body; `409 Conflict` for a real-time create duplicate
(kind-agnostic).

### 9.1 Plan CRUD

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/plans` | Create (`409` on duplicate) → `{pid, name}` |
| GET | `/api/plans` | List active (cap 100); `?parent={pid}` rolls up direct children |
| GET | `/api/plans/search?q=` | Tantivy full-text search (`?fuzzy=`, `?phonetic=`, `?kind=`) |
| GET | `/api/plans/{pid}` | Fetch the stored `Plan` (record-level ABAC; a `mask`-obligation allow returns the redacted view) |
| GET | `/api/plans/{pid}/masked` | The masked view: `lead_ref` dropped, `owner_org_id`/`owner_org_name` redacted |
| GET | `/api/plans/{pid}/export` | GDPR right-of-access export (audited; masked when the policy says so) |
| PUT | `/api/plans/{pid}` | Replace payload |
| DELETE | `/api/plans/{pid}` | Soft-delete |

`kind` is an optional descriptive label carried in the payload, never a
path segment; `parent_ref` (containment) is a payload field, denormalised
to `parent_pid` for the `?parent=` roll-up.

### 9.2 Match / dedupe / merge

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/plans/match` | Rank `{query, candidates}` (no persistence; kind-agnostic) |
| POST | `/api/plans/check-duplicates` | Match a query vs stored active plans |
| POST | `/api/plans/deduplicate` | Batch scan active plans → review queue |
| POST | `/api/plans/merge` | Merge a duplicate into any survivor (`422` equal pids, `404` unknown) |
| GET | `/api/plans/merges/recent` | Merge-history records |

Matching is **kind-agnostic** — there is no kind gate, so two plans may
match, dedupe, and merge regardless of their (optional) `kind` labels.

### 9.3 Sub-resources (keyed by the parent plan `pid`)

| Resource | Base path | Notable fields |
|---|---|---|
| Goals | `/api/plans/{pid}/goals` | title, target_date (also in payload via `data.goals[]`) — **deferred, §13**: not yet wired as a sub-resource; only goal titles exist today, via `data.goals[]` |
| Tasks | `/api/plans/{pid}/tasks` | title, assignee_ref, status, goal_id?, parent_task_id?, estimate, remaining, due_date, story_points — **landed 2026-07-20**, §9.14 |
| Issues | `/api/plans/{pid}/issues` | title, kind, severity, status, assignee_ref — **deferred, §13**: not yet wired |

Only **Tasks** are implemented today (§9.14 has the full route list,
incl. the Kanban-move `PATCH` and WIP limits); it supports `POST`
(create), `GET` (list), `PUT /{sub_pid}` (update), `PATCH /{sub_pid}`
(move), `DELETE /{sub_pid}` (soft-delete). Goals and Issues remain
deferred roadmap (§13), same as `posts` / `comments` / `members`.

### 9.4 Derived read views

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/plans/{pid}/timeline` | Goals-milestone + task date ranges → Gantt — **deferred, §13** |
| GET | `/api/plans/{pid}/burndown?sprint=` | Remaining-per-day from real `done_at` stamps only, no ideal line — **landed 2026-07-20**, §9.14 |

### 9.4a Collaboration, automation, and prioritisation

**Collaborative review** — delegate one idea / proposal / plan to the
right internal or external expert:

| Method | Path | Purpose |
|---|---|---|
| POST / GET | `/api/reviews` | Invite an expert; list invitations (`?subject_kind=&subject_pid=&reviewer=&status=&limit=&offset=`, paginated — §9.1's `X-Total-Count`/`X-Limit`/`X-Offset` contract) |
| GET | `/api/reviews/consensus` | Aggregate verdict for one subject (`?subject_kind=&subject_pid=`) |
| POST | `/api/reviews/{pid}/respond` | Accept / decline |
| POST | `/api/reviews/{pid}/submit` | Verdict: `score` 0–100 (optional), `recommendation` advance/hold/reject, `comment` |
| DELETE | `/api/reviews/{pid}` | Withdraw an invitation (a submitted verdict cannot be withdrawn) |

Reviewers are `EntityRef` URNs (`person:` / `worker:` /
`organization:`) — **never raw email**; an outside expert is a record in
the person registry, so no contact detail is duplicated here.
`reviewer_scope` (`internal` / `external`) is stored explicitly, because
sharing with an outsider is a disclosure decision, never an inference.
The state machine is `invited → accepted → submitted`, with
`declined` / `expired` / `withdrawn` terminal: **only a reviewer who
accepted may submit**, so an unanswered invitation never becomes
evidence. Consensus reports a **strict** majority only (a tie or
plurality reports none), a `mean_score` of `null` until somebody scores,
and the count still **outstanding**.

**Assignees + notifications:**

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/plans/{pid}/tasks/{t_pid}/assign` | Assign / unassign (`assignee_ref: null` unassigns); notifies the assignee |
| GET | `/api/assignees/workload` | Open work per assignee, busiest first, including the `unassigned` pile (`?plan=`) |
| GET | `/api/notifications` | One recipient's in-app inbox (`?recipient=&unread=`) |
| POST | `/api/notifications/{pid}/read` | Mark read (idempotent) |

Notifications are **in-app only** — this service has no email or push
transport and does not pretend to.

**Workflow automation** — configure once, fires as work crosses the
board:

| Method | Path | Purpose |
|---|---|---|
| POST / GET | `/api/automations` | Configure / list rules (`?plan=&trigger=&enabled=`) |
| POST | `/api/automations/{pid}/enable` · `/disable` | The off switch (keeps the rule and its history) |
| DELETE | `/api/automations/{pid}` | Soft-delete a rule |
| GET | `/api/automations/runs` | What actually fired (`?automation=&subject=&outcome=`) |

Triggers: `task_moved` (with optional `from_status` / `to_status`;
unset = any column), `review_submitted`, `plan_stage_changed`. Actions:
`assign`, `add_label`, `notify`, `schedule_action`, `set_task_status`.
Three invariants: an action's shape is validated at **write** time (a
malformed rule can never fail silently at fire time); a rule **never
breaks the operator's action** (a failure is a `failed` run, not an
undo); and actions are applied **without re-entering the engine**, so
automations cannot cascade. A rule scoped to a plan never fires on
another plan; a rule constraining a status the trigger does not carry
fails closed.

**Set and forget** — the deadline queue:

| Method | Path | Purpose |
|---|---|---|
| POST / GET | `/api/scheduled-actions` | Queue a deadline (`notify` / `expire_review`, `in_days` 1–365); list (`?status=&subject=&overdue=`) |
| POST | `/api/scheduled-actions/sweep` | Fire everything due |
| DELETE | `/api/scheduled-actions/{pid}` | Cancel a pending deadline |

The sweep **claims** each due row with a conditional update
(`pending → fired`) and only acts when it won the claim, so two
concurrent sweeps — or the optional in-process ticker
(`PROJECT_PORTFOLIO_MANAGEMENT_SCHEDULER_MINUTES`, default off) racing
the endpoint — cannot double-fire a deadline. Each sweep is capped and
reports whether it hit the cap.

**Data-driven prioritisation + bird's-eye visibility** (derived; no new
tables):

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/plans/{pid}/smart-score` | One plan's Smart Score **with its full breakdown** |
| GET | `/api/prioritisation` | Ranked queue, highest first (`?limit=&band=`) |
| GET | `/api/lifecycle` | The challenge funnel: live + stalled per phase |
| GET | `/api/plans/{pid}/lifecycle` | Phase, next gate, readiness checklist, review consensus |

The **Smart Score** is a renormalised weighted average over seven
components — `roi` (25 %), `strategic_alignment` (20 %),
`expert_review` (15 %), `risk` (15 %, inverted), `demand` (10 %),
`priority` (10 %, from the `moscow:<band>` tag), `momentum` (5 %) —
tunable via `PROJECT_PORTFOLIO_MANAGEMENT_SMART_SCORE_WEIGHTS` (a
complete map summing to 10 000 basis points; anything else is warned
about and ignored wholesale). **Absent evidence is absent, not zero**:
a missing component is dropped, the rest renormalise, every gap is
named in `missing`, and `coverage` reports how much of the weight had
data behind it. No evidence at all ⇒ `score: null` and band `unscored`,
which sorts **last** in the ranked queue rather than as a zero. ROI is
computed **only** when a plan's benefit and budget lines are all in one
currency — the family forbids FX conversion, so mixed-currency money is
reported as no ROI evidence rather than silently summed.

**Readiness** is a checklist, not a verdict: five fixed checks
(`gate_journey_open`, `no_severe_open_risks`, `reviews_answered`,
`no_overdue_actions`, `nothing_blocked`) are always all reported with
the count behind each, and `ready` is simply "no check failed". The
funnel reports every phase even at zero, and items whose phase cannot be
resolved are counted separately rather than absorbed.

### 9.5 Audit / events / docs / metrics

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/plans/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/plans/events/recent` | In-memory event stream |
| GET | `/api/plans/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (root path, public under auth enforcement) |

### 9.6 Auth

Every route may carry `Authorization: Bearer <token>` (a PASETO v4 public
token, verified offline against the auth-service's published Ed25519 key);
handlers take `MaybeAuthUser` to stamp the audit `actor`. Leads,
assignees, and owners are user / org identities from the
authentication-service and the index. Blanket `/api/*` enforcement is
wired (an `after_routes` middleware calling `auth::enforce`) but **off by
default** — gated by `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` (`1`/`true`/`yes`/`on` ⇒
on). When on, any `/api/*` route without a valid token is `401`; the
public paths `/_health`, `/_ping`, `/api-docs/openapi.json`,
`/swagger-ui*`, and `/metrics.prom` stay open (via
`src/auth.rs::is_public_path`). Keys/issuer/audience come from
`PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL` (optional URL of the auth-service's published
key set, e.g. `https://auth…/.well-known/paseto-keys`; set ⇒ fetched over
HTTP **once at boot** in `App::after_routes` via `auth::init` /
`Verifier::from_paseto_keys_url` — on success the fetched key set wins
over `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS`, on failure the service logs a warning and
falls back to the env path, so it always boots; no refresh loop — a
rotation-triggered refetch is a future item, §16) /
`PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS` (the auth-service's published Ed25519 public-key
set; absent ⇒ empty key set, all tokens rejected) / `PROJECT_PORTFOLIO_MANAGEMENT_TOKEN_ISSUER`
(default `authentication-service`) / `PROJECT_PORTFOLIO_MANAGEMENT_TOKEN_AUDIENCE` (default
`main-x-service`). Auth source of truth (supersedes the RS256-JWT model):
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md);
blanket-enforcement contract: [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md).

**Authorization (ABAC).** Inside the same guard — so only when
`PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` is on — a verified token is authorized by
**attribute-based access control** per
[`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md):
the request's action is derived from the HTTP method plus this crate's
destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`,
`/deduplicate`, `/import`), matched on path suffix so the corresponding
POST on the plans collection is `Destructive` rather than `Write`
(a `/links` DELETE is covered by the `DELETE`⇒`Delete` rule). The shared
engine in `authentication-verifier` 0.3 evaluates the policy over the
token's `attrs` claim, first-match-wins. Configure with
`PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY` (inline JSON) or `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY_FILE`
(path); unset or unparsable ⇒ warn-log + the built-in default policy (any
authenticated subject reads; `access=write` writes; `access=admin` adds
DELETE/merge/deduplicate/import; `svc=true` does everything). `401` =
missing/bad credential; `403` = valid credential, policy denied (the body
names the deciding rule). This supersedes the earlier per-crate roles/RBAC
sketch.

### 9.7 Cross-service entity links (write side)

Per [cross-service linking](../../../agents/share/cross-service-linking.md),
the Portfolio Service originates outbound cross-service edges from a plan
(or a goal / task / issue) to records in **any** index entity (case /
course / place / person / worker / organization / …). Edges are stored in
a dedicated `entity_links` table (§10), **separate** from the
within-payload `relationships` and **never** read by the matcher (the
partition rule, §5).

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/plans/{pid}/links` | Create / upsert an outbound edge; emits `linked` |
| GET | `/api/plans/{pid}/links` | List this plan's outbound edges |
| DELETE | `/api/plans/{pid}/links/{id}` | Soft-delete an edge; emits `unlinked` |

The write is **optimistic** — it stores the assertion and emits an event;
it does **not** call the target service. Verification status is the
aggregator's concern (it sees both ends).

### 9.8 Bulk import/export (deferred, §13)

The five uniform endpoints on the plans collection
([`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md) §4):

```
POST /api/plans/import         202 {job_id}
GET  /api/plans/import/{id}     status + counts + errors_url + review_url
POST /api/plans/export         202 {job_id}
GET  /api/plans/export/{id}     status + download_url
GET  /api/plans/bulk-jobs       list (filter by kind/status)
```

**Stable upsert key** = a deterministic external PM identifier
(`JiraProjectKey` / `AsanaGid` / `TrelloBoardId` / `MsProjectId` /
`GitHubProjectId` / `LinearId` / `Uri` / `Uuid`) or owner-scoped `code`
(with `owner_org_id`, same-owner only) or `pid`; keyless rows run the
normal duplicate detection → review queue (`provenance = import`),
kind-agnostic. CSV flattens every repeated/nested field to a
JSON-in-cell. Export is business/ops data (light masking) but **lead /
assignee / person refs are personal data**, so any export carrying them
is audited (even zero-row).

### 9.9 Governance (PPM Phase A)

| Method | Path | Purpose |
|---|---|---|
| POST / GET | `/api/proposals` | Intake a plan candidate; list (draft→submitted→in_review→approved/rejected→promoted) |
| GET / PUT | `/api/proposals/{pid}` | Fetch / edit a proposal |
| POST | `/api/proposals/{pid}/submit` · `/review` · `/approve` · `/reject` | Pipeline transitions |
| POST | `/api/proposals/{pid}/promote` | Mint the plan (`create_and_emit`, `provenance=intake`) |
| GET | `/api/proposals/{pid}/duplicates` | Matcher-backed duplicate-demand check vs live plans + sibling proposals |
| POST / GET | `/api/plans/{pid}/gate-reviews` | Strictly-ordered g0–g5 stage gates |
| POST / GET | `/api/plans/{pid}/risks` | 1–5×1–5 exposure-ranked risks |
| PUT | `/api/plans/{pid}/risks/{risk_pid}` | Update / `escalate` |
| POST / GET | `/api/plans/{pid}/budget-lines` | ISO-4217 minor-unit planned/actual/variance |
| POST | `/api/plans/{pid}/budget-lines/{line_pid}/actual` · `/release` | Accumulate actual; release a stage-gated tranche |
| GET | `/api/plans/{pid}/governance` | Per-plan governance summary |

Record-level ABAC (`auth::plan_resource_attrs` exposes `resource.stage`)
gates plan `PUT` and gate-review `POST`, so gate-locking is policy, not
code. Landed 2026-07-18 (T-PPM-A, §13).

### 9.10 Visibility (PPM Phase B)

| Method | Path | Purpose |
|---|---|---|
| POST / GET | `/api/dependencies` | Plan-to-plan dependencies (self/duplicate/cycle refused) |
| DELETE | `/api/dependencies/{pid}` | Remove a dependency |
| GET | `/api/plans/{pid}/schedule` | Violations + critical path (memoised longest-duration DFS) + undated members |
| POST / GET | `/api/plans/{pid}/milestones` | Milestones with overdue flags |
| POST | `/api/plans/{pid}/milestones/{m_pid}/complete` | Mark complete (`done_at` stamped) |
| POST / GET | `/api/plans/{pid}/allocations` | Per-person allocation |
| DELETE | `/api/plans/{pid}/allocations/{a_pid}` | Remove an allocation |
| GET | `/api/capacity` | Per-person rollup, flags > 100 % |
| POST / GET | `/api/reports` | Saved report definitions |
| DELETE | `/api/reports/{pid}` | Remove a saved report |
| GET | `/api/reports/{pid}/run?format=json\|csv` | Run synchronously (row cap 1000; RFC-4180 CSV) |
| GET | `/api/at-a-glance` | ETag-conditional dashboard: per-kind RAG/stage rollups + site tiles |

Landed 2026-07-18 (T-PPM-B, §13). Scheduled/artifact report runs await
the family bulk machinery (§9.8, deferred).

### 9.11 Strategy (PPM Phase C)

| Method | Path | Purpose |
|---|---|---|
| POST / GET | `/api/ideas` | Capture ideas |
| POST | `/api/ideas/{pid}/vote` · `/dismiss` · `/convert` | Vote; dismiss; convert to a draft proposal (`provenance=idea`) |
| POST / GET | `/api/scenarios` | What-if scenarios over live budgets/risks/alignment |
| GET | `/api/scenarios/{pid}/evaluate` | Evaluate (infeasible commits refused) |
| POST | `/api/scenarios/{pid}/commit` | Commit; the evaluation snapshot is audited |
| GET | `/api/scenarios/compare?a=&b=` | Two live evaluations side by side (per-currency deltas) |
| POST / GET | `/api/objectives` | The OKR registry |
| GET | `/api/objectives/{pid}/alignment` | Per-objective alignment rollup |
| POST / GET | `/api/plans/{pid}/objectives` | Weighted plan↔objective links |
| POST / GET | `/api/plans/{pid}/benefits` | Financial (minor-unit) or non-financial benefit targets |
| POST | `/api/plans/{pid}/benefits/{b_pid}/realize` | Accumulate realized value; per-currency ROI vs budget actuals |

Landed 2026-07-18 (T-PPM-C, §13). With Phases A/B/C landed, the full PPM
catalogue is delivered service-side.

### 9.12 Executive insights (read-only, derived)

Seven ETag-conditional views (`as_of` watermark, mirroring
`/at-a-glance`), deriving from existing tables:

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/executive/health` | Per-portfolio RAG briefing |
| GET | `/api/executive/decisions` | Gate reviews + scenario commits + decided proposals + merges, newest first |
| GET | `/api/executive/benefits` | Per-portfolio per-currency target vs realized |
| GET | `/api/executive/alignment` | Per-kind aligned/unaligned spend via `objective_links` |
| GET | `/api/financials/variance` | Minor-unit variance by kind/category/portfolio, one row per currency (no FX) |
| GET | `/api/financials/exposure` | Per-currency estate totals + held (gated) amounts |
| GET | `/api/technology/dependency-risk` | Top fan-out, cross-portfolio edges, RAG-red-predecessor edges |
| GET | `/api/technology/radar` | `tech:<name>[:<ring>]` tag convention, majority-ring vote |
| GET | `/api/technology/debt` | `risks.category` exposure-sorted register |
| GET | `/api/technology/flow` | Delivery-flow throughput/month + median lead days |

Landed 2026-07-19 (§13). Currencies are never summed across lines.

### 9.13 Oversight (board / auditor / compliance / CRO / CISO / regulator)

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/board/pack` | Period-scoped decisions/realizations/completions/releases + as-of-now health |
| GET | `/api/board/investments` | Investment rollup |
| POST | `/api/board/snapshots` | Capture an estate snapshot (also `src/snapshots.rs::spawn`, an optional ticker, `PROJECT_PORTFOLIO_MANAGEMENT_SNAPSHOT_HOURS`, default off) |
| GET | `/api/board/trends` | Stored snapshot series (no interpolated history) |
| GET | `/api/auditor/trail` | Filterable audit explorer + integrity stats |
| GET | `/api/auditor/findings` | Segregation-of-duties over recorded audit actors only |
| GET | `/api/auditor/evidence-pack` | JSON/CSV, capped 2000 rows |
| GET | `/api/compliance/register` · `/findings` | Category register + rule-disclosed conformance checks |
| GET | `/api/risk/heatmap` | Probability×impact cells, posture, declared appetite (`PROJECT_PORTFOLIO_MANAGEMENT_RISK_APPETITE`) |
| GET | `/api/security/register` | + disclosed no-security-risk-at-late-stage heuristic |
| GET | `/api/regulator/extract` | Deliberately coarse; honours the ABAC `mask` obligation (withholds names) |

Landed 2026-07-19/20 (§13); one new table (`insight_snapshots`). Persona
gating (board vs auditor vs regulator, …) is ABAC policy configuration
(`dept=board|audit|…`), not new code. **Note**: this
`/api/compliance/{register,findings}` regulatory-conformance register is
distinct from the row-level integrity `/api/compliance/{records,audit}/verify`
endpoints (§9.15) — the two happen to share the `compliance` path segment
for unrelated reasons.

### 9.14 Engineering-team features (tasks, sprints, DevOps)

| Method | Path | Purpose |
|---|---|---|
| POST / GET | `/api/plans/{pid}/tasks` | Create / list Kanban tasks (story points 0–100) |
| PUT | `/api/plans/{pid}/tasks/{t_pid}` | Update (never changes status — flow stamps stay true) |
| PATCH | `/api/plans/{pid}/tasks/{t_pid}` | Move (board column change; `status_changed_at` stamped; first `done` entry stamps `done_at`; `PROJECT_PORTFOLIO_MANAGEMENT_WIP_LIMITS` caps enforced, `422` on a full column) |
| DELETE | `/api/plans/{pid}/tasks/{t_pid}` | Soft-delete |
| POST / GET | `/api/plans/{pid}/sprints` | Time-boxed sprints (`ends_on >= starts_on`) |
| GET | `/api/plans/{pid}/burndown?sprint=` | Remaining-per-day from real `done_at` stamps only — no ideal line |
| GET | `/api/plans/{pid}/velocity` | Per-sprint done counts + point sums (never compare across teams/plans) |
| POST / GET | `/api/plans/{pid}/sprints/{s_pid}/notes` | Retro (`went_well`/`improve`/`action`) + feedback; `action`/`feedback` convertible once into a task |
| GET | `/api/plans/{pid}/standup` | Audit-derived last-24h digest |
| POST | `/api/devops/events` | Ingest deploy / incident / recovery |
| GET | `/api/devops/metrics` | DORA-style, derived only from ingested events |
| GET | `/api/devops/releases` | Deploy-event release register |
| GET | `/api/engineering/blocked` | Blocked-task aging |
| GET | `/api/engineering/moscow` | `moscow:<band>` tag convention rollup |
| GET | `/api/engineering/delivery-links` | External-tracker identifiers + untracked list |
| GET | `/api/engineering/milestone-calendar` | Milestone kinds: milestone/demo/release/checkpoint |

Landed 2026-07-20 (§13, "the operational core" + "moderate fits");
migrations `…_000001_engineering` + `…_000002_engineering_moderate`.
Tasks and sprints are operational data — **never** fed to the matcher
(§5 partition rule). The `goals` and `issues` sub-resources (§9.3) and
the `/{pid}/timeline` view (§9.4) remain deferred.

### 9.15 Integrity verification (`src/compliance/`)

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/compliance/records/verify?limit=` | Recomputes each plan's SHA-256 + SHA3-256 digests and a keyed HMAC-SHA256 MAC over the same pre-image; names any row that differs |
| GET | `/api/compliance/audit/verify?limit=` | Recomputes each `audit_logs` row's MAC; names any row whose content was altered |

Landed 2026-07-27/28. `limit` defaults 1000, capped 10 000
(`VERIFY_MAX_LIMIT` — an unbounded scan is a CPU DoS, the SEC-M1
bound-every-input invariant). Both endpoints sit under the blanket
`/api/*` guard (§9.6); as reads, the default policy admits any
authenticated caller. **The digests are unkeyed** (their pre-image
format is published here), so they catch careless modification; **the
MAC is what defends against a deliberate edit**, and is written only
when `PORTFOLIO_INTEGRITY_MAC_KEY` (or `..._KEY_FILE`) is configured —
unset ⇒ rows report `mac_absent`, never a false mismatch. This crate has
**no hash chain** (unlike person/worker/care-pathway/case): a MAC proves
a row's content is unchanged since written, and says nothing about a row
deleted wholesale.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. `auto_migrate` on in
development. Tables / migrations:

- `m20220101_000001_plans` — the single plan row table: `id` serial PK,
  `pid` UUID unique, `name` (denormalised from `data.name`), `data`
  JSONB (full `Plan`), a **nullable** `kind` column (the optional
  label), a denormalised `parent_pid` UUID column (from
  `data.parent_ref`, for parent roll-up queries; nullable), `active`
  bool, `deleted_at` timestamptz null.
- `m20220101_000002_audit_logs` — the CRUD audit trail (carries the
  optional `kind` of the affected plan).
- `m20220101_000003_merge_records` — record-merge history + snapshot.
- `m20220101_000004_tasks` / `…_goals` / `…_issues` — operational
  sub-resource tables, each keyed by the parent plan `pid`, with their
  own `pid`, soft-delete, and created/updated timestamps. **Excluded
  from the matcher payload** (goal titles bridge in via `data.goals[]`
  on the parent row).
- `m20220101_000005_entity_links` — the cross-service link write side
  (§9.7), separate from the within-payload `relationships`, per
  [cross-service linking
  §4.1](../../../agents/share/cross-service-linking.md).
- `m20260722_000001_capabilities` — the collaboration / automation
  tables: `reviews` (one subject delegated to one expert; partial
  unique index keeps one **live** invitation per subject+reviewer),
  `automations` + `automation_runs` (the rules and the append-only log
  of what each firing did), `scheduled_actions` (the set-and-forget
  queue, claimed atomically so a deadline fires exactly once), and
  `notifications` (the in-app inbox those two write into). **No Smart
  Score table**: the score is derived on read from rows the service
  already stores, so it cannot drift from the evidence it summarises.
- `bulk_jobs` (deferred, §13) — the shared bulk-job table
  ([`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md)
  §3, `UNIQUE (entity, kind, idempotency_key)`).
- `review_queue` — duplicate-review items (`Pending` / `Confirmed` /
  `Rejected` / `AutoMerged`, `provenance`) over the plans collection.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON round-trip +
kind-agnostic scoring: two plans match on content regardless of their
`kind` labels), the `src/validation.rs` unit tests
(UUID / PM-tool-id / URI identifier shapes, blank goal titles, BCP-47
`in_language` syntax, `parent_ref` UUID shape + containment-cycle
rejection), the `src/auth.rs`
unit tests (mint a real PASETO v4 public token + matching Ed25519 key
in-process, then assert valid → claims and missing / non-bearer / expired /
tampered / empty-verifier → `401`; plus `parse_bool` and `enforce` —
off+no-token → `Ok`, on+public → `Ok`, on+protected+{no/valid/expired/
tampered} → `401`/`Ok`), the `src/merge.rs` unit tests (former-name alias,
scalar fallback, list union, transferred snapshot, `is_self_merge`
equal-pid pin, any-two-plans merge with no kind scoping), the `escape_like`
unit test, the `src/metrics.rs` unit tests (rendered text carries every
plan metric name + `# HELP`/`# TYPE` preamble + content type), and the
derived-view unit tests (timeline ordering, burndown remaining-vs-estimate
from snapshots). Request-level tests (`tests/requests/plans.rs`, loco
testing harness) cover CRUD + match + dedupe + merge over the plans
collection, the `tasks` sub-resource CRUD + Kanban move + the burndown
view (`goals`/`issues` and `/timeline` remain deferred, §13),
unknown-pid `404` on GET / PUT / DELETE (and the merge `404`),
the `409` real-time create duplicate, the kind-agnostic matching
(a plan matches a plan of any/other kind label), the `parent_ref`
containment-cycle `422`, the `?parent=` child roll-up, the audit/event
trail, `whoami` (no token → `401`), blanket enforcement (with
`PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH=1`
in-test: un-authed `GET /api/plans` → `401`, public
`GET /api-docs/openapi.json` → `200`; `#[serial]`), and OpenAPI/Swagger —
these need Postgres, so they are `#[ignore]`-gated; run with
`cargo test -- --ignored` and a `DATABASE_URL`.

## 12. Compliance

Plans are business / operational data (no patient data), so the
default posture is light masking. However, **lead / assignee / member /
person refs are personal data** — leads, assignees, and owners are user
identities — so any read or export that surfaces them is audited, and
GDPR data-subject obligations apply to those people. The `entity_links`
edges follow the [cross-service linking §10](../../../agents/share/cross-service-linking.md)
governance for whatever target entity they reference. Honour the family
HIPAA/NHS/GDPR posture for audit and access controls.

## 13. Tasks (live work queue)


- [x] **2026-09-02 — PRO-H12 slice 7 of 7 (the last): OpenTelemetry
  OTLP export.** This crate carried no `src/observability` module at
  all before this change. Added it as a port of case-service's slice 6
  (repo `tasks.md` PRO-H12) — `src/observability.rs` (real OTLP/gRPC
  span + metric export, on by default at `OTLP_ENDPOINT`, disabled by
  setting it to the empty string), `App::init_logger`/`on_shutdown`
  wired in `src/app.rs`, and `observability::trace_mw` layered as the
  outermost middleware in `after_routes` — this crate's **only**
  router-construction surface, confirmed (not assumed) by grepping
  `src/`/`tests/` for a second `Router::new()`/`create_router`: the
  one hit is a unit test for the auth middleware, not an app-level
  router. No `otlp-test-tonic` rename was needed (this crate declares
  no `tonic` dependency of its own), and — unlike care-pathway/case —
  this crate carries no IEC 62304 SOUP register, so no
  `compliance/soup.tsv` bookkeeping was needed either: the simplest of
  the four loco-idiomatic ports. See
  [`agents/share/rust-tracing-opentelemetry-stack.md`](../../../agents/share/rust-tracing-opentelemetry-stack.md)
  for the full rollout status. Verified: `cargo fmt --check`, `cargo
  clippy --all-targets -D warnings`, `cargo deny check`, `cargo bench
  --no-run`, and the MSRV check (`cargo +1.96 check --all-targets`)
  all clean; `cargo test --lib` 361/361 (was 353, +8 new
  `src/observability.rs` unit tests); `cargo test --test otlp_export
  --test otlp_middleware` 4/4 (real protobuf crossing a real
  in-process gRPC socket). **This closes PRO-H12**: every entity
  registry in the family now exports real OTLP/gRPC traces and
  metrics.

- [x] **2026-08-21 — FUZZ-2: cargo-fuzz harness for the request-path
  logic.** Three coverage-guided targets over the pure, total code that
  faces the network: `validate_json` (bytes → `serde_json` → `Plan` →
  `validation::problems`), `validate_built` (the validator driven from
  raw bytes, so the fuzzer controls array cardinality directly rather
  than having to learn JSON first), and `merge_plans` (the merge fold over
  two arbitrary payloads). Invariants pinned: never-panic; validation is
  deterministic and its **problem report is bounded** independent of
  payload size (the generalisation of SEC-M8, which the unit tests pin
  only for one hand-written payload); merge keeps the survivor's
  `name` and is **absorbing**, so a retried merge cannot inflate
  the record. The sub-crate declares an empty `[workspace]` table
  because this crate is a workspace root, and no `rust-version` because
  cargo-fuzz is nightly-only. Verified: all three build and run clean,
  and the bounded-report assertion was confirmed live by lowering its
  ceiling until it fired.

- [x] **2026-08-21 — SEC-M8b: the `422` report is bounded, not just the
  work.** `validation::problems` reported an over-long array's
  cardinality violation once and then still walked every entry, so a
  payload with ten thousand blank/malformed entries returned ten thousand
  problem strings in one `422` body — a small request buying a large
  response. Thirteen per-entry loops across `problems` and
  `push_size_cap_problems` were involved, which is why the cap is a
  named helper rather than thirteen inline `.take(…)` calls. Every per-entry loop now walks the new `inspected()`
  helper (at most `MAX_ARRAY_LEN` entries, index preserved): the
  cardinality problem already rejects the payload, so inspecting the tail
  decides nothing, and bounding the **report** is the same input-bounding
  rule as bounding the work (SEC-M1). Named rather than inlined so a loop
  added later without it reads as different. Pinned by a test, which was
  confirmed to fail without the cap. Case was the reference
  (repo `tasks.md` SEC-M8/SEC-M8b).

- [x] **2026-08-20 — Search writer held for the process; Criterion
  benchmarks; declared MSRV.** `SearchEngine::index_plan` (and the
  delete/clear paths) built a **new Tantivy `IndexWriter` per call**.
  That allocates the whole 50 MB `WRITER_HEAP_MB` arena and spawns merge
  threads on construction, so every create / update / merge /
  soft-delete paid it synchronously on the request path — ~155 ms per
  indexed document, measured against a fresh index. It was also a
  concurrency hazard: an `IndexWriter` holds the index directory's
  exclusive lock, so taking and releasing it per call left two
  simultaneous writes able to collide on it. The engine now holds one
  `Mutex<IndexWriter>` created in `new()` (~78 ms per document; the
  remainder is the durable `commit()` plus reader `reload()` that
  read-after-write indexing inherently costs — see repo `tasks.md`
  PERF-2 for the open design question). Found by `benches/service_bench.rs`,
  new in the same pass: validation, merge, and search groups over the
  CPU-bound halves of a request, compiled in CI by the new
  `scripts/ci-check.sh bench` stage. `Cargo.toml` also declares
  `rust-version = "1.95"` — the repository's current-stable-minus-three
  floor (`spec/rust-msrv-n-minus-2/index.md`), enforced by
  `scripts/ci-check.sh msrv`.

- [x] **2026-08-05 — Pagination on the five operational sub-resource
  lists (repo tasks.md PG-1, closing the sub-bullet left open on
  2026-08-01).** `automations`, automation runs, the deadline queue,
  delegations (`/api/reviews`), and one inbox (`/api/notifications`)
  now take `?limit=`/`?offset=` and report `X-Total-Count`/`X-Limit`/
  `X-Offset`, exactly like `GET /api/plans` (§9.1, 2026-08-01) — same
  clamp (500), same out-of-bound-offset `400` (10 000), same
  default-reproduces-the-old-cap rule (`LIST_CAP` = 200 stays the
  default page size on all five). The `Page`/`with_page_headers`
  mechanics were promoted out of `controllers/plans.rs` into a new
  `controllers::pagination` module so five more call sites share one
  implementation instead of copying it a fifth and sixth time;
  `controllers/plans.rs` itself now imports from there too (no
  behavioural change, confirmed by its own unchanged pagination test).
  The deadline queue (`GET /api/scheduled-actions`) keeps its
  soonest-first `due_at` order under paging — a page is a contiguous
  slice of that order, not a reshuffled one, pinned by a dedicated
  assertion. *Verified:* 42 DB-gated (2 new: `automation_lists_are_paginated`,
  `collaboration_lists_are_paginated`) + 1 masking + 1 enforcement + 1
  outbox-audit green vs Postgres 18; 205 lib tests; fmt + clippy clean.
  Front-end: see the sibling `project-portfolio-management-front-end-with-svelte`
  `spec/index.md` §13 for the consuming-route changes.
- [x] **2026-08-02 — Privacy: field masking + GDPR export (repo
  tasks.md P-4, as P-1/organization; lower sensitivity).** The
  thinnest privacy module of the four in the family, by design: most
  of a `Plan` (`name`, `code`, `goals`, `status`, dates, `identifiers`,
  `tags`, `relationships`, `parent_ref`) is operational content the
  registry and the project-management tooling exist to serve up, not
  personal data. Two things are: `lead_ref` (the plan lead, a
  `person:`/`worker:` `EntityRef` — the most directly personal field on
  a `Plan`, so **dropped entirely** rather than partially shown — a
  partial UUID has no "still recognisable" value when the plan is
  already identified by `name`/`code`) and `owner_org_id` /
  `owner_org_name` (the sponsoring organisation — institutional,
  masked to its tail like organization's `telephone`/`email` and
  care-pathway's `provider_name`/`provider_id`). `src/privacy.rs`:
  `mask_plan` + `export_plan`. `GET /{pid}` (previously **unguarded** —
  no caller, no ABAC check at all) now honours the `mask` obligation via
  the existing `auth::authorize_record` + `auth::plan_resource_attrs`
  (PPM-3's `stage` attribute, unchanged); new `GET /{pid}/masked` and
  `GET /{pid}/export` (§9.1). The export is audited via the plain
  `AuditModel::record` (this crate has no HIPAA-style disclosure module,
  unlike case/care-pathway — matching organization's posture instead).
  The end-to-end obligation proof (`tests/masking.rs`) is its own test
  binary from the start, avoiding the `OnceLock`-sharing mistake made
  and fixed in case's P-3.
  *Verified:* 40 DB-gated request-suite + 1 dedicated masking binary +
  1 enforcement + 1 outbox-audit green vs Postgres 18; 205 lib tests;
  fmt + clippy clean.
- [x] **2026-08-02 — Tantivy full-text/fuzzy/phonetic search (repo
  tasks.md S-4).** Transfers the care-pathway/case Tantivy pattern
  (S-2/S-3) whole: `src/search/index.rs` (`PlanIndexSchema`/`PlanIndex`
  — `pid` STORED; `name`/`alternate_names`/`name_phonetic`/
  `identifiers`/`keywords`/`tags`/`goals`/`owner_org_name` TEXT;
  `code`/`owner_org_id`/`kind`/`status`/`active` STRING exact-match) and
  `src/search/mod.rs` (`SearchEngine`: `search_page`, `candidates`).
  Replaces the Postgres `ILIKE` name search (`GET /search?q=`, now
  `?fuzzy=`/`?phonetic=` too, true `X-Total-Count` from Tantivy's
  `Count` collector) and the capped 1000-row `check-duplicates` scan
  (now blocked on up to 200 fuzzy-name/phonetic/exact-identifier
  candidates). Indexing is wired into `streaming.rs`'s `*_and_emit` seam
  (best-effort, after commit); `tasks/search.rs` adds the
  `search_reindex` CLI task plus boot-time rebuild-if-empty. Both
  endpoints answer `503` (never silent-empty) when the index is down.
  **`kind` is indexed and filterable on search (`?kind=`) but is
  deliberately never applied to `candidates`** — the embedded matcher's
  own golden rule is that no two plans are excluded from matching by
  kind, and gating dedup by kind would silently reintroduce the
  per-kind collection boundary the data model unification removed (see
  AGENTS.md golden rule 5). `search::tests::candidates_ignore_kind` +
  `kind_filter_narrows_search_but_is_optional` pin both halves of that
  distinction, and a DB-gated test confirms it end to end.
  *Verified:* 38 DB-gated + 1 enforcement + 1 outbox-audit green vs
  Postgres 18; 199 lib tests; fmt + clippy clean.

- [x] **2026-07-22 — Collaboration / automation / prioritisation
  capabilities (§9.4a).** Migration `m20260722_000001_capabilities`
  (`reviews`, `automations`, `automation_runs`, `scheduled_actions`,
  `notifications`); four pure rule modules (`collaboration.rs`,
  `automation.rs`, `prioritisation.rs`, `lifecycle.rs`) with 56 DB-free
  unit tests; three controllers (`collaboration`, `automation`,
  `prioritisation`); the engine fires from `move_task` and from a
  submitted plan review; the optional sweep ticker
  (`PROJECT_PORTFOLIO_MANAGEMENT_SCHEDULER_MINUTES`, default off) in
  `src/scheduler.rs`; OpenAPI + a doc test; six `#[ignore]`d request
  tests. Every mutation audits. **Not built:** email / push transport
  (notifications are in-app only), a `votes` component on the Smart
  Score (a plan does not link back to its originating idea), and
  record-level ABAC on the new endpoints — they sit behind the blanket
  guard only.

- [x] **2026-07-22 — Accept the five new `kind` labels.**
  `parse_kind_label` maps `practice`/`process`/`purpose`/`pathway`/
  `proposal` (and their plurals) to the matcher's new `PlanKind`
  variants; the OpenAPI `Plan.kind` enum and the `kind_target` /
  `collection` validation messages list them. Matching stays
  kind-agnostic. The `Proposal` *label* is unrelated to the
  `proposals` intake resource (§9.4).

- [x] **2026-07-20 — Unify the four "work item" kinds into one recursive
  `plan`.** The former Portfolio / Project / Product / Program
  collections are unified into a **single** matchable entity, the
  **plan**: the type is `Plan` (was `WorkItem`), the table is `plans`
  (was per-kind tables),
  and the DTO is `project_portfolio_management_matcher::Plan` stored as
  JSONB. `kind` becomes `Option<PlanKind>` — an **optional** descriptive
  label that neither gates matching nor fixes a collection; the
  matcher's hard kind gate is **removed** (any two plans may match /
  dedupe / merge). The four REST collections collapse to **one**
  `/api/plans` collection (CRUD + match + check-duplicates + merge +
  audit/events + whoami, and all sub-resources moved under
  `/api/plans/{pid}/…`); `GET /api/plans?parent={pid}` rolls up a plan's
  direct children (was the per-portfolio roll-up), and `GET
  /api/plans/{pid}/schedule` is no longer restricted to a Portfolio-kind
  plan. Containment generalises from `portfolio_ref` (portfolio→child
  only) to `parent_ref` (any plan may contain any other, a recursive
  tree); the service adds a containment **cycle check** returning `422`.
  The denormalised column is `parent_pid` (was `portfolio_pid`) and the
  `kind` column is nullable. Merge proposals carry an optional
  `kind_target` label (blank = none). Builds and tests green.
- [x] loco boot + the single `plans` table/migration + a shared CRUD
  controller core with `422` validation (blank `name`; UUID / PM-tool-id
  / URI `identifiers` shapes; blank goal titles; BCP-47 `in_language`
  syntax; `parent_ref` UUID + containment-cycle check — all problems
  reported together) + real-time create duplicate detection (`409`,
  kind-agnostic). Implemented: `controllers/plans.rs` + `validation.rs`
  + `migration/…_000001_plans`.
- [x] **SEC-M1 — input-size caps in `src/validation.rs`** (2026-07-13).
  Bound every scalar text field (`MAX_TEXT_LEN = 1024` chars), every
  array (`MAX_ARRAY_LEN = 256` entries), and every string entry inside
  an array (`MAX_ITEM_LEN = 512` chars, incl. `goals[i].title` /
  `identifiers[i].value` / `relationships[i].plan_id`) → `422`, before
  store/match, to close the matcher's `O(n·m)` CPU/memory DoS vector
  (amplified by check-duplicates). `kind` is an enum, not capped.
- [x] Matching — embed `project-portfolio-management-matcher`
  (`MatchingEngine::new( MatchConfig::default())`); `POST /match`, `POST
  /check-duplicates` (blocked on the Tantivy index since 2026-08-02, §9.2).
  Matching is **kind-agnostic** — no kind gate, so any
  two plans may match regardless of their optional `kind` labels; covered
  in `tests/matching.rs`.
  - [ ] `POST /deduplicate` (batch scan → review queue) — not wired; no
    `review_queue` table exists yet for this crate.
- [x] Name search — `GET /search?q=` Tantivy full-text/fuzzy/phonetic
  search (§9.1, §13 2026-08-02), replacing the earlier Postgres `ILIKE`.
- [x] Operational sub-resources — `tasks` / `goals` / `issues` tables +
  CRUD controllers, each keyed by the parent plan `pid`, audited,
  emitting scoped events. **`tasks` landed 2026-07-20** (§9.14 —
  `controllers/engineering.rs`, incl. the Kanban `PATCH` move + WIP
  limits + story points).
  - [ ] `goals` / `issues` — not wired (`posts` / `comments` / `members`
    also deferred).
- [x] Derived views — `GET /{pid}/timeline` (Gantt) + `/{pid}/burndown`
  from task estimate/remaining snapshots; pure projection logic + unit
  tests. **`burndown` landed 2026-07-20** (§9.14 — honest, real
  `done_at`-stamps-only derivation, no ideal line).
  - [ ] `/{pid}/timeline` (Gantt) — not wired.
- [x] Record merge — `POST /merge` (union fields, former-name
  alias, soft-delete, `merge_records` history + snapshot, `Replaces`
  link, `Merged` event); merge any two plans (no kind scoping);
  `/merges/recent`; pure `src/merge.rs`. Implemented and tested.
- [x] Event streaming + audit log on CRUD (plan + sub-resources) —
  `audit_logs` table + best-effort row per write; in-memory event
  stream; `/audit/recent`, `/{pid}/audit`, `/events/recent`. **Phase 1**
  of the durable event bus (canonical versioned `Envelope` +
  `EventPublisher` seam + `InMemoryPublisher` ring buffer; frozen
  `EventView` projection); Phase 2 (transactional outbox) + Phase 3
  (relay + retention) are landed below; the Fluvio broker sink is the
  remaining infra-gated follow-up, designed in
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md). -
  [x] **Durable event bus — Phase 3 (relay + retention).**
  `src/relay.rs`: the `EventSink` trait (the bus seam), a working
  no-broker **`LoggingSink`** default, `drain_once` (`unpublished` →
  `sink.send` → `mark_published`, at-least-once, per-pid order preserved
  on a send failure), and `purge_published` (retention — **enforced**:
  deletes `published_at < now() - INTERVAL '<n> days'`). A background
  loop (`relay::spawn`, started in `App::after_routes`) ticks every
  `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_RELAY_INTERVAL_SECS` (default 5,
  floored at 1) and purges every N ticks using
  `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_RETENTION_DAYS` (default 7) —
  **gated by `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_TRANSPORT=outbox` AND
  `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_RELAY`** (truthy
  `1`/`true`/`yes`/`on`), so it is a no-op by default. Tests: DB-free
  `LoggingSink`/capturing-sink send + config defaults; the drain/ack
  seams (`unpublished`/`mark_published`) are DB-gated via the outbox
  suite.
- [x] **Durable event bus — Phase 3, `FluvioSink` (BUS-3, following
  BUS-1's case-service reference).** *(done 2026-08-03)* The real-broker
  `impl EventSink`, behind this crate's own `fluvio` Cargo feature (off
  by default — the dependency tree and boot behaviour of a default build
  are unchanged). One producer per topic
  (`fluvio::Fluvio::connect_with_config` + `topic_producer`, held for
  the sink's lifetime), partitioned by record `pid` per §7. Config:
  `PROJECT_PORTFOLIO_MANAGEMENT_FLUVIO_ENDPOINT` (the broker's SC
  address; unset ⇒ `LoggingSink`, unchanged default behaviour) and
  `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_TOPIC` (default `mxi.plan.events`
  — the `plan` streaming-entity token, per `src/streaming.rs::ENTITY`,
  not the `portfolio` token `src/auth.rs` uses for ABAC action naming).
  **No silent fallback**: an endpoint configured **without** the
  `fluvio` feature refuses to start the relay at all (logged at
  `error`), rather than a `LoggingSink` masquerade that would mark
  outbox rows `published_at` without ever reaching the broker the
  operator asked for — the same shape as the family's artifact-store "no
  fallback on an explicit backend choice" rule
  (`agents/share/bulk-import-export.md` §12). The initial connection
  retries indefinitely rather than falling back, for the same reason.
  `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a local
  SC+SPU broker (Fluvio's own documented Docker Compose layout,
  translated to this repo's Podman conventions, `mxi-project-portfolio-management-fluvio-*`
  container names) for opt-in manual runs; **not** wired into any
  automated CI stage. Tests: `cargo build`/`clippy --all-targets -D
  warnings`/`fmt --check` clean under both default features and
  `--features fluvio` (the real `fluvio` 0.50 API compiling is the
  actual verification of correct usage); `tests/fluvio_relay.rs` is a
  `#![cfg(feature = "fluvio")]`-gated, `#[ignore]`d round-trip (enqueue →
  `FluvioSink` → `drain_once` → assert `published_at`) with its run
  command documented inline — it needs a live broker, which no automated
  run in this repo stands up, so it is verified by compiling under the
  feature, not by an actual execution (same posture as person's
  `s3_round_trip_against_a_live_endpoint`, BLK-4, and case's
  `fluvio_relay_publishes_an_outbox_row_to_a_real_topic`, BUS-1). This
  crate has no `compliance/soup.tsv` (unlike case), so no SOUP register
  update was needed. BUS-2 (link-graph Fluvio consumer) and the
  remaining BUS-3 legs (rolling `FluvioSink` to the other entity
  services) remain.
  ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §5,
  §7, §8).
- [ ] Cross-service links — `m20220101_000005_entity_links` migration +
  `POST`/`GET`/`DELETE /api/plans/{pid}/links`; emit `linked` /
  `unlinked`; optimistic write (no cross-service call); never a matcher
  signal. Contract:
  [`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md).
- [x] OpenAPI/Swagger — `src/openapi.rs` (utoipa) served at
  `/api-docs/openapi.json` + `/swagger-ui` by `controllers/docs.rs`
  (covers the full surface, incl. the PPM phases / insights / oversight
  / engineering tags — not just the plans collection).
- [x] Prometheus metrics — `GET /metrics.prom` (root path, `text/plain;
  version=0.0.4`); process-wide `OnceLock` registry in `src/metrics.rs`
  (plan CRUD/merge counters + `http_requests_total`); public under
  blanket auth enforcement.
- [x] Offline PASETO v4 public verification (`authentication-verifier`);
  `src/auth.rs` per
  [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
  (supersedes the RS256-JWT model). `src/auth.rs` embeds
  `authentication-verifier` (a PASETO v4.public verifier); offline
  Ed25519 verification via a process-wide `Verifier` (env-configured
  keys/issuer/audience); `AuthUser`/`MaybeAuthUser` extractors;
  `/whoami` protected; audit `actor` from the token.
- [x] Blanket `/api/*` enforcement — `auth::enforce(require_auth, path,
  headers, verifier)` + an `after_routes` layer, gated per-request by
  `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`. Wired and tested; **off by
  default** per the family's activation-gate convention
  (`agents/share/security.md` §4) — a deployment opts in. Public
  paths stay open. Family contract: `agents/share/jwt-enforcement.md`. -
  [x] paseto-keys-over-HTTP fetch at boot — done 2026-07-04.
  `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL` set (non-blank) ⇒
  `auth::init` (called from `App::after_routes`, before serving) fetches
  the published key set once via `Verifier::from_paseto_keys_url`
  (verifier `fetch` feature); success ⇒ fetched key set wins over
  `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS` (`tracing::info!`), failure
  ⇒ `tracing::warn!` + fall back to the env path (the service always
  boots); unset/blank ⇒ prior behaviour unchanged. No refresh loop
  (rotation-triggered refetch → §16). Tests: a `#[tokio::test]` local
  ephemeral-port HTTP listener proves the fetch-built verifier accepts a
  token signed by the served key, and a fast-failing URL
  (`http://127.0.0.1:1/`) proves fallback without panic.
- [x] Privacy — masking of lead / assignee / person refs on read +
  export; GDPR obligations for those people. **Done 2026-08-02** — see
  the dated entry near the top of this section (`src/privacy.rs`,
  `GET /{pid}/masked`, `GET /{pid}/export`).
- [x] **Integrity verification (2026-07-27/28) — not previously tracked
  here.** `src/compliance/` (`mac.rs` binding the shared `integrity-mac`
  crate, `record_integrity.rs` over `plans`, `audit_integrity.rs` over
  `audit_logs`); `GET /api/compliance/records/verify` and
  `GET /api/compliance/audit/verify` (§9.15). Default off without
  `PORTFOLIO_INTEGRITY_MAC_KEY[_FILE]` (rows report `mac_absent`, never
  a false mismatch). No hash chain (unlike person/worker/care-pathway/
  case) — a MAC proves content integrity, not non-deletion. This landed
  alongside the same-day integrity rollout across every remaining entity
  service but had no `spec/13`/`spec/14`/`AGENTS.md` entry until this
  pass.
- [ ] Bulk import/export — `bulk_jobs` migration + the five endpoints on
  the plans collection (§9.8); `bg_pg` worker draining `queued → running
  → completed | completed_with_errors
  | failed`; JSONL/CSV/Parquet codecs (CSV JSON-in-cell flattening;
  Parquet export-only, feature-gated); per-row pipeline reusing
  `src/validation.rs` + the matcher + the review queue (upsert by
  external PM id / owner-scoped `code` / `pid`; keyless → dedupe →
  review, `provenance = import`, kind-agnostic); per-row error report;
  export masking + audit (lead/person refs personal data). Uniform
  contract:
  [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md).
- [ ] Collaboration sub-resources (deferred) — `posts` / `comments` /
  `members` tables + CRUD, mirroring the plan template's collaboration
  tier, if/when prioritised.
- [x] Tantivy full-text/fuzzy/phonetic search over the JSONB payload
  (§13 2026-08-02).

- [x] **Fix: fresh-Postgres `db migrate` failed in the `event_outbox`
  migration (2026-07-18).** The loco `create_table` helper pluralizes
  table names (`cruet::to_plural`: `event_outbox` → `event_outboxes`),
  so the migration's own index DDL (`ON event_outbox`) failed and
  rolled the whole fresh migrate back — no tables were ever created.
  The migration is now explicit SQL creating exactly `event_outbox`
  (matching the `SeaORM` entity), `IF NOT EXISTS`-guarded; same
  migration name (the old form could never have applied anywhere).
  Found and fixed family-wide from the patient-flow implementation
  round; verified by a live fresh-database migrate. Every other table
  this crate creates via the helper is already plural (no-op).

- [x] **T-PPM-A — Governance core (PPM-1/3/10/12; spec
  `../spec/15-roadmap.md`), delivered 2026-07-18.** Migration
  `…_000005_governance` (`proposals`, `gate_reviews`, `risks`,
  `budget_lines`, + operational `plans.stage`); pure rules in
  `src/governance.rs` (proposal pipeline state machine, strictly
  ordered g0–g5 gates, 1–5×1–5 risk exposure, ISO-4217 shape,
  overflow-safe minor-unit money — all DB-free unit-tested);
  `controllers/governance.rs` (intake pipeline
  draft→submitted→in_review→approved/rejected→promoted with the
  promote step minting the plan via `create_and_emit` and
  `provenance=intake` audit; matcher-backed duplicate-demand check
  over live plans + sibling proposals; gate reviews advancing
  `stage`; risks with exposure-ranked list + escalate; budget lines
  with per-currency planned/actual/variance and accumulate-actual;
  the per-plan `/governance` summary). Record-level ABAC:
  `auth::plan_resource_attrs` exposes `resource.stage` and
  `auth::authorize_record` gates plan `PUT` + gate-review
  `POST`, so gate-locking is policy. OpenAPI `governance` tag; every
  mutation audited. Tests: 4 pure-rule unit tests + 5 DB-gated
  request tests (`tests/requests/governance.rs`), all green vs
  Postgres 18. Events for governance mutations ride the audit trail
  only for now — envelope kinds for them arrive with the Phase-B
  dashboard work.

- [x] **T-PPM-B — Visibility (PPM-6/7/8/9; spec
  `../spec/15-roadmap.md`), delivered 2026-07-18.** Migration
  `…_000006_visibility` (`plan_dependencies`, `milestones`,
  `allocations`, `report_definitions`); pure rules in
  `src/visibility.rs` (flexible-date parsing `YYYY[-MM[-DD]]`,
  DFS cycle detection, finish-start violation checks with lag, a
  memoised longest-duration critical path, the documented RAG
  heuristic, window-overlap capacity sums, RFC-4180 CSV escaping —
  7 DB-free unit tests); `controllers/visibility.rs` (dependencies
  with self/duplicate/cycle refusal; `GET
  /plans/{pid}/schedule` with violations + critical path +
  undated members; milestones with overdue flags; allocations +
  `GET /capacity` per-person rollup flagging > 100 %; saved reports
  run synchronously as JSON or CSV, row cap 1000; the
  ETag-conditional `GET /at-a-glance` dashboard with per-kind
  RAG / stage rollups and site tiles). 5 DB-gated request tests,
  green vs Postgres 18. Scheduled/artifact report runs await the
  family bulk machinery (roadmap).

- [x] **T-PPM-C — Strategy (PPM-2/4/5/11; spec
  `../spec/15-roadmap.md`), delivered 2026-07-18.** Migration
  `…_000007_strategy` (`ideas`, `scenarios`, `objectives`,
  `objective_links`, `benefits`); pure rules in `src/strategy.rs`
  (scenario evaluation over prepared member facts — per-currency
  saturating sums, cap + must-include violations; ROI in basis
  points with the zero-cost guard; OKR weight bounds — 3 DB-free
  unit tests); `controllers/strategy.rs` (the idea funnel:
  capture / vote / dismiss / convert-to-draft-proposal with
  `provenance=idea`; scenarios evaluated over live budgets, risks,
  and alignment with **infeasible commits refused** and the
  evaluation snapshot audited on commit; the OKR registry with
  weighted per-pair-upserting plan mappings and per-kind
  alignment rollups; benefits with financial minor-unit targets or
  non-financial notes, accumulate-realize, and per-currency ROI
  against recorded budget actuals). 4 DB-gated request tests, green
  vs Postgres 18. The full PPM catalogue (Phases A+B+C) is now
  delivered service-side.

- [x] **2026-07-19 — Executive insight areas (CEO / CFO / CTO).**
  Seven read-only derived views over existing tables (no new
  migrations), each ETag-conditional with `as_of` (mirroring
  `/at-a-glance`): `GET /api/executive/health` (per-portfolio RAG
  briefing — worst-member status, overdue milestones, escalated risks,
  exposure, overrun currencies, staleness), `GET
  /api/executive/decisions` (gate reviews + scenario commits + decided
  proposals + merges, newest first), `GET /api/executive/benefits`
  (per-portfolio per-currency target vs realized; ratio only with a
  positive target), `GET /api/financials/variance` (minor-unit
  variance by kind / category / portfolio, one row per currency,
  no FX), `GET /api/financials/exposure` (per-currency estate totals,
  deliberately no cross-currency sum), `GET
  /api/technology/dependency-risk` (top fan-out, cross-portfolio
  edges, RAG-red-predecessor edges), `GET /api/technology/radar`
  (tag convention `tech:<name>[:<ring>]`, majority ring vote, cautious
  tie-break). Pure derivations in `src/insights.rs` (unit-tested);
  controller `src/controllers/insights.rs`; OpenAPI paths added.
  **Acceptance:** insights unit tests + the seeded seven-view request
  round-trip (incl. ETag 304 replay) green — full `--ignored` suite
  24/24 vs Postgres 18; clippy pedantic clean. Front-end `/executive`,
  `/financials`, `/technology` consume the views.

- [x] **2026-07-19 — Executive moderate fits.** Five follow-ups to the
  insight areas, one migration (`m20260719_000002_insight_columns`, all
  columns nullable so existing rows keep their behaviour):
  **stage-gated funding tranches** — `budget_lines.gate` +
  `released_at`; a gated line is HELD (actuals `422`) until the plan's
  stage reaches the gate and `POST
  /{plans}/{pid}/budget-lines/{line_pid}/release` succeeds
  (`rules::gate_reached`, fail-closed; audit `budget_line_released`;
  `financials/exposure` reports `held_minor` per currency);
  **technical-debt register** — `risks.category`
  (`delivery`/`tech_debt`/`compliance`/`security`/`other`, validated;
  absent reads `delivery`) + `GET /api/technology/debt`
  (exposure-sorted register); **delivery-flow metrics** —
  `milestones.done_at` stamped on complete + `GET /api/technology/flow`
  (throughput/month, median lead days; pre-stamp completions counted
  but never timed); **strategic-alignment coverage** — `GET
  /api/executive/alignment` (per-kind aligned/unaligned via
  `objective_links`, unaligned spend per currency, plans ranked by
  largest single-currency planned — disclosed heuristic, currencies
  never summed); **scenario comparison** — `GET
  /api/scenarios/compare?a=&b=` (two live evaluations side-by-side,
  per-currency deltas b−a, exposure/alignment deltas). The capex/opex
  split needed no work: `BUDGET_CATEGORIES` is already the closed
  `{capex, opex}` set and `financials/variance` already rolls up
  by category. **Acceptance:** `gate_reached` unit pins; the seeded
  moderate-fits request round-trip (held→release→actual lifecycle,
  held_minor drop, debt filter, flow timing, alignment flip, compare
  deltas) green — full `--ignored` suite 25/25 vs Postgres 18; clippy
  pedantic clean; FE svelte-check 0, vitest 45, Playwright 12.

- [x] **2026-07-19/20 — Oversight areas (board / auditor / compliance /
  CRO / CISO / regulator).** Thirteen endpoints in
  `controllers/oversight.rs`, one new table
  (`m20260719_000003_insight_snapshots`): `GET /api/board/pack`
  (period-scoped decisions / realizations / completions / releases +
  as-of-now health), `GET /api/board/investments`, `POST
  /api/board/snapshots` + `GET /api/board/trends` (stored estate
  snapshots only — no interpolated history; optional env-gated ticker
  `PROJECT_PORTFOLIO_MANAGEMENT_SNAPSHOT_HOURS`, default off), `GET
  /api/auditor/trail` (filterable explorer + integrity stats), `GET
  /api/auditor/findings` (segregation-of-duties over recorded audit
  actors only — never cross-identifier-space), `GET
  /api/auditor/evidence-pack` (JSON/CSV, capped 2000), `GET
  /api/compliance/{register,findings}` (category register +
  rule-disclosed conformance checks), `GET /api/risk/heatmap`
  (probability×impact cells, posture, 25%-disclosed concentration,
  hygiene, declared appetite via
  `PROJECT_PORTFOLIO_MANAGEMENT_RISK_APPETITE` or an honest absence),
  `GET /api/security/register` (+ the disclosed
  no-security-risk-at-late-stage heuristic), `GET
  /api/regulator/extract` (deliberately coarse; honours the ABAC
  `mask` obligation by withholding names). Persona gating = ABAC
  policy configuration (`dept=board|audit|…`), not new code.
  **Acceptance:** heatmap/appetite unit pins; the seeded oversight
  request round-trip green — full `--ignored` suite 26/26 vs Postgres
  18; clippy pedantic clean; FE `/board` `/auditor` `/compliance`
  `/risk` `/security` `/regulator` pages with svelte-check 0, vitest
  45, Playwright 18.

- [x] **2026-07-20 — Engineering-team features (the §13 operational
  core).** Migration `m20260720_000001_engineering` (`tasks` +
  `sprints` tables, `milestones.kind`): the **tasks** sub-resource
  (CRUD + the PATCH board move; `status_changed_at` stamped per move,
  first entry into `done` stamps `done_at` and keeps it; PUT refuses
  status changes so flow stamps stay true; statuses
  todo/in_progress/in_review/done/blocked; `task_created`/`task_moved`
  audits with from→to snapshots), **sprints** (time-boxed;
  `ends_on >= starts_on`) + the honest **burndown**
  (`GET .../burndown?sprint=` — remaining per day from real `done_at`
  stamps only, derivation served, no ideal line), the **standup
  digest** (`GET .../standup` — audit-derived last-24h), and the
  estate views `GET /api/engineering/{blocked,moscow,delivery-links,
  milestone-calendar}` (blocked aging; `moscow:<band>` tag convention
  with untagged counted never guessed; external-tracker identifiers +
  untracked list; milestone kinds milestone/demo/release/checkpoint).
  Tasks/sprints are operational data — never fed to the matcher (the
  §5 partition rule). Timeline view and issues/goals tables remain
  deferred. **Acceptance:** pure burndown/MoSCoW unit pins; the seeded
  engineering request round-trip green first run — full `--ignored`
  suite 27/27 vs Postgres 18; clippy pedantic clean; FE board page
  (drag-to-move Kanban + burndown + standup), `/calendar`,
  `/engineering` with svelte-check 0, vitest 45, Playwright 21.

- [x] **2026-07-20 — Engineering moderate fits.** Migration
  `m20260720_000002_engineering_moderate`: **story points** on tasks
  (0–100, validated; team-local) + `GET .../velocity` (per-sprint done
  counts + point sums from real `done_at` stamps; the served note says
  never to compare across teams/plans); **WIP limits** via
  `PROJECT_PORTFOLIO_MANAGEMENT_WIP_LIMITS` JSON (per-status caps per
  plan board; a move into a full capped column is `422`; unset ⇒ no
  caps, fail-open by design and disclosed); **sprint notes** (retro +
  RAD feedback log: `went_well` / `improve` / `action` / `feedback`,
  with `action`/`feedback` convertible once into a task —
  `sprint_note_converted` audit); **`DevOps` event ingest** (`POST
  /api/devops/events`: deploy requires environment, recovery must
  reference its incident, incidents may declare `caused_by_deploy_pid`)
  + `GET /api/devops/metrics` (DORA-style, **derived only from
  ingested events**: deploys per month/environment, incidents, MTTR
  over linked incident→recovery pairs with unresolved counted never
  timed, change-failure over declared-cause incidents only) + `GET
  /api/devops/releases` (the deploy-event release register).
  **Acceptance:** WIP-limit parse pins; the seeded moderate round-trip
  + the env-gated WIP-limit request test green — full `--ignored`
  suite 29/29 vs Postgres 18; clippy pedantic clean; FE board gains
  points/velocity/retro-notes, `/engineering` gains the `DevOps`
  metrics + releases; svelte-check 0, vitest 45, Playwright 21.

## 14. Implementation status

**Implemented (MVP v0.1.0 + PPM Phases A/B/C; see §13 for the
delivered-task detail).** The crate builds and tests green: one REST
collection (`/api/plans`) over one `plans` table with a nullable `kind`
label and a `parent_pid` containment column — CRUD + Tantivy full-text/
fuzzy/phonetic search (`kind` is a search filter, never a matching gate) +
kind-agnostic matching (embedded matcher, no kind gate) + real-time
create duplicate detection + record merge + payload validation (`422`,
incl. `parent_ref` containment-cycle check) + audit log +
durable-outbox events (Phase 2 outbox + Phase 3 relay/retention,
default-off via `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_TRANSPORT=memory`)
+ offline PASETO v4.public verification with the blanket ABAC guard
(default-off via `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`) + OpenAPI
/ Swagger + Prometheus, plus the three PPM phases (Governance:
proposals / gate reviews / risks / budget lines; Visibility:
dependencies / schedule / milestones / allocations / capacity / reports
/ at-a-glance; Strategy: ideas / scenarios / objectives / benefits) and
the engineering core (tasks / sprints / burndown / standup / DevOps),
plus the executive insight areas (§9.12), the oversight areas — board /
auditor / compliance-register / risk-heatmap / security / regulator
(§9.13) — collaboration / automation / prioritisation (§9.4a), field
masking + audited GDPR export wired to the ABAC `mask` obligation
(§13, 2026-08-02), and row-level integrity verification (§9.15,
2026-07-27/28, default off without a configured MAC key).
Still open (§13): the remaining operational sub-resources (goals /
issues) + the derived timeline view, `deduplicate` + review
queue, cross-service `entity_links`, bulk import/export,
and the collaboration sub-resources, gRPC. The durable event bus's
`FluvioSink` real-broker sink landed 2026-08-03 (BUS-3, following
BUS-1's case-service reference; feature-gated, off by default). The
canonical `Plan` domain model is owned by the
[portfolio entity spec §5](../../spec/index.md); this crate spec
references it.

## 15. Roadmap

`0.1.0` (unreleased) target: the CRUD + matching MVP over the single
plans collection, then `ILIKE` search (since replaced by Tantivy,
2026-08-02) + audit + in-memory streaming, then
the operational sub-resources (goals / tasks / issues) + derived views +
record merge + cross-service links + OpenAPI/Swagger + Prometheus +
offline PASETO v4 public verification + blanket `/api/*` enforcement (auth
source of truth, superseding the RS256-JWT model:
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)),
then field masking + audited GDPR export wired to the ABAC `mask`
obligation (2026-08-02; §13).
Next (deferred, §13):
front-end merge action, bulk import/export, the `posts` / `comments` /
`members` collaboration sub-resources, gRPC. (Done since: the durable
event bus's Phase 2 outbox + Phase 3 relay/retention, then its
`FluvioSink` real-broker sink (BUS-3, following BUS-1's case-service
reference, 2026-08-03); the
paseto-keys-over-HTTP fetch at boot, 2026-07-04 —
`PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL`, fetched key set wins, env fallback.)

## 16. Open questions

- ~~Normalise goals / tasks into a search index once Tantivy lands, or
  keep ILIKE-on-name only?~~ RESOLVED (2026-08-02): landed — goal titles
  are indexed (§13); tasks are not (a plan's tasks are operational detail,
  not identity signal, and are not part of the matcher payload either).
- Should `deduplicate` auto-merge above `auto_merge_threshold`, or always
  route to the review queue?
- Burndown snapshot cadence — on every task write, or a periodic `bg_pg`
  snapshot job?
- Should a plan's child roll-up be served only as `GET /api/plans?parent={pid}`
  (its direct children by `parent_pid`), or additionally as a recursive
  descendant walk (`…/{pid}/descendants`) once traversal-depth limits are
  fixed?
- Key-set refresh: the boot-time paseto-keys fetch is once-only — add a
  rotation-triggered refetch (e.g. on `UnknownKid`) or a periodic
  refresh loop?

## 17. References

- The [portfolio entity spec](../../spec/index.md) (canonical model §5);
  the [project-portfolio-management-matcher spec](../../project-portfolio-management-matcher-rust-crate/spec/index.md);
  loco.rs; [cross-service linking](../../../agents/share/cross-service-linking.md);
  [bulk import/export](../../../agents/share/bulk-import-export.md);
  [event bus](../../../agents/share/event-bus.md).

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`. When
the integration contract changes, also update the
[portfolio entity spec](../../spec/index.md).
