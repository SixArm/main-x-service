# Portfolio Service — Specification

> **Single source of truth** for this crate's internals. Code conforms
> to this spec. Behavioural change = spec + code + test in one PR. Live
> work queue is §13.
>
> Entity-wide contract: [portfolio entity spec](../../spec/index.md) (the API
> DTO **is** the matcher's `WorkItem` type; canonical domain model in its §5).
> Sibling matcher: [project-portfolio-management-matcher](../../project-portfolio-management-matcher-rust-crate/spec/index.md).
> Sibling front-end: [project-portfolio-management-front-end-with-svelte](../../project-portfolio-management-front-end-with-svelte/spec/index.md).

## 1. Purpose and vision

A registry of **work-item** records for the Main X Index family — and a
project-management tool. A *work item* is a matchable identity for one
of **four distinct kinds**: a **Portfolio** (the umbrella container), a
**Project**, a **Product**, or a **Program**. The service has **two
faces that share one record**: a deduplicated, matchable identity
registry (the thin `WorkItem` payload, scored by the canonical
project-portfolio-management-matcher) and a project workspace (each work item *owns*
operational sub-resources — goals, tasks, issues — plus derived timeline
/ burndown views). The four kinds are **separate collections and
tables**: a Portfolio is the umbrella, and Projects / Products / Programs
sit **under** a portfolio (they carry a `portfolio_ref` to their parent).
Built on loco.rs.

## 2. Scope

MVP: CRUD + `ILIKE` name search + matching + record merge + audit log +
in-memory event streaming (durable-bus Phase 1) across the **four
matchable collections** (`portfolios`, `projects`, `products`,
`programs`) + the operational sub-resources (goals, tasks, issues) on any
work item + derived timeline / burndown read views + cross-service entity
links (write side) + OpenAPI/Swagger + Prometheus metrics + offline
PASETO v4 public token verification (Ed25519, published key) + blanket
`/api/*` auth enforcement (off by default) + payload validation.
Matching is **within a collection only** — a project never matches a
product (enforced by the matcher's `kind` gate, §5/§9.2).
Deferred (§13): the goals/issues sub-resource tables + the derived
timeline view (tasks + sprints + burndown landed 2026-07-20), Tantivy
full-text/fuzzy search, search-blocked dedup
candidates, the durable event bus's Fluvio broker sink (Phase 2 outbox +
Phase 3 relay/retention landed), privacy,
front-end merge action, bulk import/export,
gRPC, and the deferred `posts` / `comments` / `members` collaboration
sub-resources. (The paseto-keys-over-HTTP fetch at boot landed
2026-07-04 — `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL`, §9.6/§13.) Token **issuance** is out of scope — provided by the central
authentication-service. Auth source of truth (supersedes the RS256-JWT
model): [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).

## 3. Stakeholders and users

Portfolio / programme managers and delivery teams curating work items and
working the boards; peer services (person / worker / organization /
authentication, and the cross-service link aggregator); the portfolio
front-end.

## 4. Glossary

- **work item** — a portfolio / project / product / program; the
  matchable identity record. The canonical Rust type is `WorkItem`, with
  a required `kind` discriminator.
- **kind** — `Portfolio` | `Project` | `Product` | `Program`; the
  collection / table the record lives in, and a hard match gate.
- **portfolio** — the umbrella kind of work item; the container under
  which projects / products / programs sit.
- **pid** — public UUID of a work-item record.
- **data** — the full `WorkItem` payload stored as JSONB.
- **portfolio_ref** — the parent portfolio `pid` carried by a Project /
  Product / Program (the umbrella link); absent for Portfolio kind.
- **sub-resource** — operational data a work item owns (goal, task,
  issue), in its own table keyed by the parent `(kind, pid)`; **not**
  part of the matcher payload (except goal **titles** via `data.goals[]`).
- **EntityRef** — `<entity_type>:<uuid>` URN naming a record in another
  service (lead / assignee / owner org).
- **derived view** — a read-only projection (timeline / burndown)
  computed from goals and tasks; never persisted as its own row.

## 5. Domain model

The API DTO is `project_portfolio_management_matcher::WorkItem` — the canonical model lives
in the [portfolio entity spec §5](../../spec/index.md). Matchable payload
(thin identity): `kind`, `name`, `alternate_names`, `code`,
`owner_org_id`, `owner_org_name`, `lead_ref`, `portfolio_ref`, `status`,
`goals` (titles + optional target dates), `start_date`, `target_date`,
`keywords`, `tags`, `identifiers`, `same_as`, `in_language`,
`relationships`. **The high-volume operational data is deliberately
excluded from the payload** — tasks and issues live in their own tables
(§10) and are never fed to the matcher; goals are the one bridge (their
titles feed matching via `data.goals[]`).

> **Match gate — within-kind only.** Matching is partitioned by `kind`:
> the matcher short-circuits to `0.0` when two work items have different
> `kind` (the R-GATE, [project-portfolio-management-matcher §1–§25](../../project-portfolio-management-matcher-rust-crate/spec/index.md)).
> You never match a project against a product. Each REST collection only
> ever compares records within its own collection.

> **Partition rule — within-payload relationships vs cross-service
> links.** `relationships` inside the `WorkItem` payload are within-entity
> and **are** a matcher signal (Jaccard component). Cross-service links
> (§9.7) live only in `entity_links`, the `linked`/`unlinked` events, and
> the aggregator — they are **never** stored in the payload and **never**
> fed to the matcher. See
> [cross-service linking §7](../../../agents/share/cross-service-linking.md).

## 6. Functional requirements

The **four collections** — `portfolios`, `projects`, `products`,
`programs` — share an **identical controller shape**; `{collection}`
below stands for any one of them, and `{kind}` for its `WorkItemKind`.
Child collections (`projects` / `products` / `programs`) additionally
carry `portfolio_ref` → parent portfolio pid; the `portfolios` collection
does not.

1. `POST /api/{collection}` — create; `kind` is fixed by the
   collection (server-stamped / rejected if mismatched), `name` required,
   `identifiers` structurally checked per scheme (canonical UUID for
   `Uuid`; external PM-tool id shapes for `JiraProjectKey` / `AsanaGid` /
   `TrelloBoardId` / `MsProjectId` / `GitHubProjectId` / `LinearId`; URI
   shape for `Uri`; non-blank for the rest), `goals` titles non-blank, and
   `in_language` checked for BCP-47 syntax; child kinds require a
   well-formed `portfolio_ref`; `422` on any problem, **all reported
   together** — also enforced on update. Real-time duplicate detection on
   create returns `409 Conflict` with candidate matches (within this
   collection only) when duplicates are found. Rules in
   [`src/validation.rs`](../src/validation.rs).
2. `GET /api/{collection}` — list active (cap 100), `{pid, name}`.
   `GET /api/{collection}/search?q=` — case-insensitive name search
   (Postgres `ILIKE`, cap 50; blank `q` → `400`).
3. `GET /api/{collection}/{pid}` — return the stored `WorkItem`.
4. `PUT /api/{collection}/{pid}` — replace the payload (`422` if `name`
   is blank, or any `goals` / `identifiers` / `in_language` /
   `portfolio_ref` entry is malformed).
5. `DELETE /api/{collection}/{pid}` — soft-delete.
6. `POST /api/{collection}/match` — rank an explicit `{query,
   candidates}` set (no persistence). Candidates of a different `kind`
   from the query gate to `0.0` (R-GATE).
7. `POST /api/{collection}/check-duplicates` — match a query against
   stored active records **in this collection**; return those above
   threshold, ranked.
   `POST /api/{collection}/deduplicate` — batch scan of active rows
   in this collection into the review queue (status `Pending`/`Confirmed`/
   `Rejected`/`AutoMerged`).
8. `POST /api/{collection}/merge` — fold a duplicate into a survivor
   **of the same kind** (union payload fields, former-name alias,
   soft-delete the duplicate, `merge_records` history + transferred
   snapshot, `Replaces` link from survivor → duplicate, `Merged` event);
   `422` equal pids or cross-kind merge, `404` unknown.
   `GET /api/{collection}/merges/recent` — merge history.
9. Operational sub-resource CRUD on **any** work item (separate tables
   keyed by the parent `(kind, pid)`):
   - **Goals** — `…/{collection}/{pid}/goals` (also part of the matcher
     payload via `data.goals[]` mutation — the goals bridge).
   - **Tasks** — `…/{collection}/{pid}/tasks` (`status` Todo/InProgress/
     InReview/Done/Blocked; `assignee_ref` EntityRef; `goal_id?`,
     `parent_task_id?`, `estimate`, `remaining`, `due_date`).
   - **Issues** — `…/{collection}/{pid}/issues` (`kind` Bug/Risk/Blocker/
     Question/Improvement; `severity` Low/Med/High/Critical; `status`
     Open/InProgress/Resolved/Closed; `assignee_ref`).
   Full CRUD on each; every write audits + emits a `created`/`updated`/
   `deleted` event scoped to the sub-resource and its parent. (Deferred
   §13: `posts` / `comments` / `members` collaboration sub-resources.)
10. Derived read views — `GET /api/{collection}/{pid}/timeline`
   (goals-with-target-date milestones + task date ranges → Gantt) and
   `GET /api/{collection}/{pid}/burndown` (remaining-vs-estimate over
   time from task estimate/remaining snapshots). Read-only projections;
   never persisted as their own row.
11. Cross-service links — `POST`/`GET`/`DELETE
   /api/{collection}/{pid}/links` (§9.7), emitting `linked` /
   `unlinked`.
12. `GET /api/{collection}/audit/recent` + `/{pid}/audit` — audit-log
   query; `GET /api/{collection}/events/recent` — in-memory event
   stream. Each create/update/delete/merge (work item and sub-resource)
   writes an `audit_logs` row and publishes a `created`/`updated`/
   `deleted`/`merged` (and `linked`/`unlinked`) event.
13. `GET /api/{collection}/whoami` — echo verified bearer-token claims
   (`401` without a valid token); proves offline PASETO verification.
14. `GET /api-docs/openapi.json` + `GET /swagger-ui` — OpenAPI 3 document
   and a Swagger UI page rendering it.
15. `GET /metrics.prom` — Prometheus metrics in text-exposition format
   (`Content-Type: text/plain; version=0.0.4`), mounted at the root (not
   under `/api`) and public under blanket auth enforcement so a scraper
   needs no token. Exposes per-collection CRUD/merge counters
   (`portfolio_{collection}_created_total` / `_updated_total` /
   `_deleted_total` / `_merged_total`) plus `http_requests_total`.
16. Bulk import/export (deferred, §13) — async, job-based, on the loco
   `bg_pg` worker: the five endpoints per collection (§9.8). The uniform
   family contract (execution model, JSONL/CSV/Parquet codecs,
   upsert-by-stable-key + dedupe-to-review, per-row error report, export
   masking + audit) is fixed in
   [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md);
   portfolio-specific bits are in §9.8 / §10.

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic + probabilistic
matching via the embedded `project-portfolio-management-matcher` (`MatchingEngine::new(
MatchConfig::default())` in
[`src/controllers/work_items.rs`](../src/controllers/work_items.rs));
soft-delete with audit-friendly timestamps. The four collections share
one parameterised controller core (one `kind` per route group), so they
never drift. Sub-resource tables are sized for high-volume operational
churn and are kept off the matcher hot path (the matcher only ever sees
the thin JSONB payload). The within-kind match gate keeps each
collection's dedup scan scoped to its own rows.

## 8. Architecture

loco `App` (`src/app.rs`) registers the four work-item collection
controllers (one shared core, parameterised by `kind`) and the
sub-resource controllers. One table per kind (`portfolios`, `projects`,
`products`, `programs`) stores `pid` + denormalised `name` + the full
`WorkItem` JSONB `data` (child tables also denormalise `portfolio_pid`);
the operational sub-resources live in their own tables keyed by the
parent `(kind, pid)` (§10). Matching calls `project-portfolio-management-matcher` directly
on the deserialised payloads — **no adapter**, mirroring care-pathway —
and the matcher's R-GATE makes cross-kind pairs score `0.0`. Cross-service
links use the hybrid topology
([cross-service linking §4](../../../agents/share/cross-service-linking.md)):
the service writes its outbound edges locally and emits events; a
standalone aggregator builds the queryable graph.

## 9. API surface

API URLs are version-free; a client selects the representation version
with the `Accepts-version` request header (default `1.0`) — see
[`agents/share/api-versioning.md`](../../../agents/share/api-versioning.md).

Raw loco JSON under `/api/`. `{collection}` ∈ `{portfolios, projects,
products, programs}`. `404` for unknown `pid`; `422` for a validation
failure (blank `name`, a malformed `goals` / `identifiers` / `in_language`
/ `portfolio_ref` entry — family convention, via `Error::CustomError(
StatusCode::UNPROCESSABLE_ENTITY, …)`, every problem in one body); `400`
for a malformed body; `409 Conflict` for a real-time create duplicate
(within the collection).

### 9.1 Work-item CRUD (per collection)

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/{collection}` | Create (`409` on duplicate) → `{pid, name}` |
| GET | `/api/{collection}` | List active (cap 100) |
| GET | `/api/{collection}/search?q=` | Case-insensitive name search (`ILIKE`, cap 50) |
| GET | `/api/{collection}/{pid}` | Fetch the stored `WorkItem` |
| PUT | `/api/{collection}/{pid}` | Replace payload |
| DELETE | `/api/{collection}/{pid}` | Soft-delete |

Concretely the four collections are `/api/portfolios`,
`/api/projects`, `/api/products`, `/api/programs`.

### 9.2 Match / dedupe / merge (within a collection)

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/{collection}/match` | Rank `{query, candidates}` (no persistence; cross-kind → `0.0`) |
| POST | `/api/{collection}/check-duplicates` | Match a query vs stored active records in this collection |
| POST | `/api/{collection}/deduplicate` | Batch scan this collection's active rows → review queue |
| POST | `/api/{collection}/merge` | Merge a duplicate into a same-kind survivor (`422` equal pids / cross-kind, `404` unknown) |
| GET | `/api/{collection}/merges/recent` | Merge-history records |

The matcher's **R-GATE** (different `kind` ⇒ `0.0`) makes matching
within-collection only; the controller never compares across collections.

### 9.3 Sub-resources (keyed by the parent `(kind, pid)`)

| Resource | Base path | Notable fields |
|---|---|---|
| Goals | `/api/{collection}/{pid}/goals` | title, target_date (also in payload via `data.goals[]`) |
| Tasks | `/api/{collection}/{pid}/tasks` | title, assignee_ref, status, goal_id?, parent_task_id?, estimate, remaining, due_date |
| Issues | `/api/{collection}/{pid}/issues` | title, kind, severity, status, assignee_ref |

Each base path supports `POST` (create), `GET` (list), `GET /{sub_pid}`
(fetch), `PUT /{sub_pid}` (update), `DELETE /{sub_pid}` (soft-delete).
(`posts` / `comments` / `members` are deferred roadmap, §13.)

### 9.4 Derived read views

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/{collection}/{pid}/timeline` | Goals-milestone + task date ranges → Gantt |
| GET | `/api/{collection}/{pid}/burndown` | Remaining-vs-estimate over time |

### 9.5 Audit / events / docs / metrics

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/{collection}/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/{collection}/events/recent` | In-memory event stream |
| GET | `/api/{collection}/whoami` | Verified bearer-token claims (`401` without one) |
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
POST on any of the four collections is `Destructive` rather than `Write`
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
the Portfolio Service originates outbound cross-service edges from a work
item (or a goal / task / issue) to records in **any** index entity (case /
course / place / person / worker / organization / …). Edges are stored in
a dedicated `entity_links` table (§10), **separate** from the
within-payload `relationships` and **never** read by the matcher (the
partition rule, §5).

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/{collection}/{pid}/links` | Create / upsert an outbound edge; emits `linked` |
| GET | `/api/{collection}/{pid}/links` | List this work item's outbound edges |
| DELETE | `/api/{collection}/{pid}/links/{id}` | Soft-delete an edge; emits `unlinked` |

The write is **optimistic** — it stores the assertion and emits an event;
it does **not** call the target service. Verification status is the
aggregator's concern (it sees both ends).

### 9.8 Bulk import/export (deferred, §13)

The five uniform endpoints per collection
([`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md) §4):

```
POST /api/{collection}/import         202 {job_id}
GET  /api/{collection}/import/{id}     status + counts + errors_url + review_url
POST /api/{collection}/export         202 {job_id}
GET  /api/{collection}/export/{id}     status + download_url
GET  /api/{collection}/bulk-jobs       list (filter by kind/status)
```

**Stable upsert key** = a deterministic external PM identifier
(`JiraProjectKey` / `AsanaGid` / `TrelloBoardId` / `MsProjectId` /
`GitHubProjectId` / `LinearId` / `Uri` / `Uuid`) or owner-scoped `code`
(with `owner_org_id`, same-owner only) or `pid`; keyless rows run the
normal duplicate detection → review queue (`provenance = import`), still
within-collection. CSV flattens every repeated/nested field to a
JSON-in-cell. Export is business/ops data (light masking) but **lead /
assignee / person refs are personal data**, so any export carrying them
is audited (even zero-row).

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. `auto_migrate` on in
development. Tables / migrations:

- `m20220101_000001_portfolios` / `…_projects` / `…_products` /
  `…_programs` — one row table per work-item kind: `id` serial PK, `pid`
  UUID unique, `name` (denormalised from `data.name`), `data` JSONB (full
  `WorkItem`), `active` bool, `deleted_at` timestamptz null. The three
  child tables (`projects` / `products` / `programs`) additionally carry a
  denormalised `portfolio_pid` UUID column (from `data.portfolio_ref`) for
  parent roll-up queries; `portfolios` does not.
- `m20220101_000002_audit_logs` — the CRUD audit trail (carries the
  `kind` of the affected work item).
- `m20220101_000003_merge_records` — record-merge history + snapshot.
- `m20220101_000004_tasks` / `…_goals` / `…_issues` — operational
  sub-resource tables, each keyed by the parent `(parent_kind,
  parent_pid)`, with their own `pid`, soft-delete, and created/updated
  timestamps. **Excluded from the matcher payload** (goal titles bridge in
  via `data.goals[]` on the parent row).
- `m20220101_000005_entity_links` — the cross-service link write side
  (§9.7), separate from the within-payload `relationships`, per
  [cross-service linking §4.1](../../../agents/share/cross-service-linking.md).
- `bulk_jobs` (deferred, §13) — the shared bulk-job table
  ([`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md) §3,
  `UNIQUE (entity, kind, idempotency_key)`).
- `review_queue` — duplicate-review items (`Pending` / `Confirmed` /
  `Rejected` / `AutoMerged`, `provenance`), scoped to a collection.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON round-trip +
R-GATE: cross-kind pairs score `0.0`), the `src/validation.rs` unit tests
(UUID / PM-tool-id / URI identifier shapes, blank goal titles, BCP-47
`in_language` syntax, child-kind `portfolio_ref` shape), the `src/auth.rs`
unit tests (mint a real PASETO v4 public token + matching Ed25519 key
in-process, then assert valid → claims and missing / non-bearer / expired /
tampered / empty-verifier → `401`; plus `parse_bool` and `enforce` —
off+no-token → `Ok`, on+public → `Ok`, on+protected+{no/valid/expired/
tampered} → `401`/`Ok`), the `src/merge.rs` unit tests (former-name alias,
scalar fallback, list union, transferred snapshot, `is_self_merge`
equal-pid pin, cross-kind merge rejected), the `escape_like` unit test,
the `src/metrics.rs` unit tests (rendered text carries every per-collection
metric name + `# HELP`/`# TYPE` preamble + content type), and the
derived-view unit tests (timeline ordering, burndown remaining-vs-estimate
from snapshots). Request-level tests (`tests/requests/work_items.rs`, loco
testing harness) cover, for each of the four collections, CRUD + match +
dedupe + merge, the sub-resource CRUD, the timeline/burndown views,
`/links`, unknown-pid `404` on GET / PUT / DELETE (and the merge `404`),
the `409` real-time create duplicate, the cross-collection isolation
(a project query never matches a product), the audit/event trail, `whoami`
(no token → `401`), blanket enforcement (with `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH=1`
in-test: un-authed `GET /api/projects` → `401`, public
`GET /api-docs/openapi.json` → `200`; `#[serial]`), and OpenAPI/Swagger —
these need Postgres, so they are `#[ignore]`-gated; run with
`cargo test -- --ignored` and a `DATABASE_URL`.

## 12. Compliance

Work items are business / operational data (no patient data), so the
default posture is light masking. However, **lead / assignee / member /
person refs are personal data** — leads, assignees, and owners are user
identities — so any read or export that surfaces them is audited, and
GDPR data-subject obligations apply to those people. The `entity_links`
edges follow the [cross-service linking §10](../../../agents/share/cross-service-linking.md)
governance for whatever target entity they reference. Honour the family
HIPAA/NHS/GDPR posture for audit and access controls.

## 13. Tasks (live work queue)

- [ ] loco boot + one table/migration per kind (`portfolios`, `projects`,
  `products`, `programs`) + a shared parameterised CRUD controller core
  with `422` validation (blank `name`; UUID / PM-tool-id / URI
  `identifiers` shapes; blank goal titles; BCP-47 `in_language` syntax;
  child-kind `portfolio_ref` shape — all problems reported together) +
  real-time create duplicate detection (`409`, within-collection).
- [x] **SEC-M1 — input-size caps in `src/validation.rs`** (2026-07-13).
  Bound every scalar text field (`MAX_TEXT_LEN = 1024` chars), every array
  (`MAX_ARRAY_LEN = 256` entries), and every string entry inside an array
  (`MAX_ITEM_LEN = 512` chars, incl. `goals[i].title` /
  `identifiers[i].value` / `relationships[i].work_item_id`) → `422`,
  before store/match, to close the matcher's `O(n·m)` CPU/memory DoS
  vector (amplified by check-duplicates). `kind` is an enum, not capped.
- [ ] Matching — embed `project-portfolio-management-matcher` (`MatchingEngine::new(
  MatchConfig::default())`); `POST /match`, `POST /check-duplicates`
  (scan this collection's active rows), `POST /deduplicate` (batch →
  review queue). **R-GATE**: cross-`kind` pairs score `0.0`, so matching
  is partitioned per collection (never match a project against a product);
  cover it in `tests/matching.rs`.
- [ ] Name search — `GET /search?q=` Postgres `ILIKE` on the denormalised
  `name` (cap 50, wildcards escaped), per collection. Tantivy full-text /
  fuzzy deferred.
- [ ] Operational sub-resources — `tasks` / `goals` / `issues` tables +
  CRUD controllers, each keyed by the parent `(kind, pid)`, audited,
  emitting scoped events. (`posts` / `comments` / `members` deferred.)
- [ ] Derived views — `GET /{pid}/timeline` (Gantt) + `/{pid}/burndown`
  from task estimate/remaining snapshots; pure projection logic + unit
  tests.
- [ ] Record merge — `POST /merge` (union fields, former-name alias,
  soft-delete, `merge_records` history + snapshot, `Replaces` link,
  `Merged` event); reject cross-kind merge; `/merges/recent`; pure
  `src/merge.rs`.
- [ ] Event streaming + audit log on CRUD (work item + sub-resources) —
  `audit_logs` table + best-effort row per write; in-memory event stream;
  `/audit/recent`, `/{pid}/audit`, `/events/recent`. **Phase 1** of the
  durable event bus (canonical versioned `Envelope` + `EventPublisher`
  seam + `InMemoryPublisher` ring buffer; frozen `EventView` projection);
  Phase 2 (transactional outbox) + Phase 3 (relay + retention) are landed
  below; the Fluvio broker sink is the remaining infra-gated follow-up,
  designed in
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md).
- [x] **Durable event bus — Phase 3 (relay + retention).** `src/relay.rs`:
  the `EventSink` trait (the bus seam), a working no-broker **`LoggingSink`**
  default, `drain_once` (`unpublished` → `sink.send` → `mark_published`,
  at-least-once, per-pid order preserved on a send failure), and
  `purge_published` (retention — **enforced**: deletes `published_at <
  now() - INTERVAL '<n> days'`). A background loop (`relay::spawn`, started
  in `App::after_routes`) ticks every `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_RELAY_INTERVAL_SECS`
  (default 5, floored at 1) and purges every N ticks using
  `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_RETENTION_DAYS` (default 7) — **gated by
  `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_TRANSPORT=outbox` AND `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_RELAY`** (truthy
  `1`/`true`/`yes`/`on`), so it is a no-op by default. Tests: DB-free
  `LoggingSink`/capturing-sink send + config defaults; the drain/ack seams
  (`unpublished`/`mark_published`) are DB-gated via the outbox suite.
  **Broker-gated follow-up:** a real **`FluvioSink`** (`impl EventSink`
  behind a `fluvio` cargo feature) — the trait is the seam, so the drain
  loop is unchanged when it lands
  ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §5, §8).
- [ ] Cross-service links — `m20220101_000005_entity_links` migration +
  `POST`/`GET`/`DELETE /api/{collection}/{pid}/links`; emit `linked` /
  `unlinked`; optimistic write (no cross-service call); never a matcher
  signal. Contract:
  [`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md).
- [ ] OpenAPI/Swagger — `src/openapi.rs` (utoipa) served at
  `/api-docs/openapi.json` + `/swagger-ui` by `controllers/docs.rs`
  (covering all four collections).
- [ ] Prometheus metrics — `GET /metrics.prom` (root path,
  `text/plain; version=0.0.4`); process-wide `OnceLock` registry in
  `src/metrics.rs` (per-collection CRUD/merge counters +
  `http_requests_total`); public under blanket auth enforcement.
- [ ] Offline PASETO v4 public verification (`authentication-verifier`);
  `src/auth.rs` per
  [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
  (supersedes the RS256-JWT model). `src/auth.rs` embeds
  `authentication-verifier` (a PASETO v4.public verifier); offline Ed25519
  verification via a process-wide `Verifier` (env-configured
  keys/issuer/audience); `AuthUser`/`MaybeAuthUser` extractors; `/whoami`
  protected; audit `actor` from the token.
  - [ ] Blanket `/api/*` enforcement — `auth::enforce(require_auth, path,
    headers, verifier)` + an `after_routes` layer, gated per-request by
    `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` (off by default). Public paths stay open.
    Family contract: `agents/share/jwt-enforcement.md`.
  - [x] paseto-keys-over-HTTP fetch at boot — done 2026-07-04.
    `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL` set (non-blank) ⇒ `auth::init` (called
    from `App::after_routes`, before serving) fetches the published key
    set once via `Verifier::from_paseto_keys_url` (verifier `fetch`
    feature); success ⇒ fetched key set wins over
    `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS` (`tracing::info!`), failure ⇒
    `tracing::warn!` + fall back to the env path (the service always
    boots); unset/blank ⇒ prior behaviour unchanged. No refresh loop
    (rotation-triggered refetch → §16). Tests: a `#[tokio::test]` local
    ephemeral-port HTTP listener proves the fetch-built verifier accepts
    a token signed by the served key, and a fast-failing URL
    (`http://127.0.0.1:1/`) proves fallback without panic.
- [ ] Privacy — masking of lead / assignee / person refs on read +
  export; GDPR obligations for those people.
- [ ] Bulk import/export — `bulk_jobs` migration + the five endpoints per
  collection (§9.8); `bg_pg` worker draining `queued → running →
  completed | completed_with_errors | failed`; JSONL/CSV/Parquet codecs
  (CSV JSON-in-cell flattening; Parquet export-only, feature-gated);
  per-row pipeline reusing `src/validation.rs` + the matcher + the review
  queue (upsert by external PM id / owner-scoped `code` / `pid`; keyless →
  dedupe → review, `provenance = import`, within-collection); per-row
  error report; export masking + audit (lead/person refs personal data).
  Uniform contract:
  [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md).
- [ ] Collaboration sub-resources (deferred) — `posts` / `comments` /
  `members` tables + CRUD, mirroring the plan template's collaboration
  tier, if/when prioritised.
- [ ] Tantivy full-text/fuzzy search over the JSONB payload.

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
  `budget_lines`, + operational `work_items.stage`); pure rules in
  `src/governance.rs` (proposal pipeline state machine, strictly
  ordered g0–g5 gates, 1–5×1–5 risk exposure, ISO-4217 shape,
  overflow-safe minor-unit money — all DB-free unit-tested);
  `controllers/governance.rs` (intake pipeline
  draft→submitted→in_review→approved/rejected→promoted with the
  promote step minting the work item via `create_and_emit` and
  `provenance=intake` audit; matcher-backed duplicate-demand check
  over live work items + sibling proposals; gate reviews advancing
  `stage`; risks with exposure-ranked list + escalate; budget lines
  with per-currency planned/actual/variance and accumulate-actual;
  the per-item `/governance` summary). Record-level ABAC:
  `auth::work_item_resource_attrs` exposes `resource.stage` and
  `auth::authorize_record` gates work-item `PUT` + gate-review
  `POST`, so gate-locking is policy. OpenAPI `governance` tag; every
  mutation audited. Tests: 4 pure-rule unit tests + 5 DB-gated
  request tests (`tests/requests/governance.rs`), all green vs
  Postgres 18. Events for governance mutations ride the audit trail
  only for now — envelope kinds for them arrive with the Phase-B
  dashboard work.

- [x] **T-PPM-B — Visibility (PPM-6/7/8/9; spec
  `../spec/15-roadmap.md`), delivered 2026-07-18.** Migration
  `…_000006_visibility` (`work_item_dependencies`, `milestones`,
  `allocations`, `report_definitions`); pure rules in
  `src/visibility.rs` (flexible-date parsing `YYYY[-MM[-DD]]`,
  DFS cycle detection, finish-start violation checks with lag, a
  memoised longest-duration critical path, the documented RAG
  heuristic, window-overlap capacity sums, RFC-4180 CSV escaping —
  7 DB-free unit tests); `controllers/visibility.rs` (dependencies
  with self/duplicate/cycle refusal; `GET
  /portfolios/{pid}/schedule` with violations + critical path +
  undated members; milestones with overdue flags; allocations +
  `GET /capacity` per-person rollup flagging > 100 %; saved reports
  run synchronously as JSON or CSV, row cap 1000; the
  ETag-conditional `GET /at-a-glance` dashboard with per-collection
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
  weighted per-pair-upserting item mappings and per-collection
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
  variance by collection / category / portfolio, one row per currency,
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
  `released_at`; a gated line is HELD (actuals `422`) until the item's
  stage reaches the gate and `POST
  /{collection}/{pid}/budget-lines/{line_pid}/release` succeeds
  (`rules::gate_reached`, fail-closed; audit `budget_line_released`;
  `financials/exposure` reports `held_minor` per currency);
  **technical-debt register** — `risks.category`
  (`delivery`/`tech_debt`/`compliance`/`security`/`other`, validated;
  absent reads `delivery`) + `GET /api/technology/debt`
  (exposure-sorted register); **delivery-flow metrics** —
  `milestones.done_at` stamped on complete + `GET /api/technology/flow`
  (throughput/month, median lead days; pre-stamp completions counted
  but never timed); **strategic-alignment coverage** — `GET
  /api/executive/alignment` (per-collection aligned/unaligned via
  `objective_links`, unaligned spend per currency, items ranked by
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

## 14. Implementation status

**Implemented (MVP v0.1.0 + PPM Phases A/B/C; see §13 for the
delivered-task detail).** The crate builds and tests green: four REST
collections (portfolios / projects / products / programs) over one
`kind`-keyed `work_items` table — CRUD + `ILIKE` search + within-kind
matching (embedded matcher, R-GATE) + real-time create duplicate
detection + record merge + payload validation (`422`) + audit log +
durable-outbox events (Phase 2 outbox + Phase 3 relay/retention,
default-off via `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_TRANSPORT=memory`)
+ offline PASETO v4.public verification with the blanket ABAC guard
(default-off via `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH`) + OpenAPI
/ Swagger + Prometheus, plus the three PPM phases (Governance:
proposals / gate reviews / risks / budget lines; Visibility:
dependencies / schedule / milestones / allocations / capacity / reports
/ at-a-glance; Strategy: ideas / scenarios / objectives / benefits).
Still open (§13): the operational sub-resources (goals / tasks /
issues) + derived timeline / burndown views, `deduplicate` + review
queue, cross-service `entity_links`, bulk import/export, Tantivy,
privacy, the collaboration sub-resources, gRPC, and the Fluvio broker
sink. The canonical `WorkItem` domain model is owned by the
[portfolio entity spec §5](../../spec/index.md); this crate spec
references it.

## 15. Roadmap

`0.1.0` (unreleased) target: the CRUD + matching MVP across the four
collections, then `ILIKE` search + audit + in-memory streaming, then the
operational sub-resources (goals / tasks / issues) + derived views +
record merge + cross-service links + OpenAPI/Swagger + Prometheus +
offline PASETO v4 public verification + blanket `/api/*` enforcement (auth
source of truth, superseding the RS256-JWT model:
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
Next (deferred, §13): Tantivy full-text/fuzzy search, the durable event
bus's Fluvio broker sink (Phase 2 outbox + Phase 3 relay/retention
landed), privacy,
front-end merge action, bulk import/export, the `posts` / `comments` /
`members` collaboration sub-resources, gRPC. (Done since: the
paseto-keys-over-HTTP fetch at boot, 2026-07-04 —
`PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL`, fetched key set wins, env fallback.)

## 16. Open questions

- Normalise goals / tasks into a search index once Tantivy lands, or keep
  ILIKE-on-name only?
- Should `deduplicate` auto-merge above `auto_merge_threshold`, or always
  route to the review queue?
- Burndown snapshot cadence — on every task write, or a periodic `bg_pg`
  snapshot job?
- Should a portfolio's child roll-up (its projects / products / programs)
  be a derived read view on the portfolio (`…/{pid}/children`), or driven
  purely by the front-end querying each child collection by `portfolio_ref`?
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
