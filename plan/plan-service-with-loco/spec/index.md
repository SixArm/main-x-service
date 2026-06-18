# Plan Service — Specification

> **Single source of truth** for this crate's internals. Code conforms
> to this spec. Behavioural change = spec + code + test in one PR. Live
> work queue is §13.
>
> Entity-wide contract: [plan entity spec](../../spec/index.md) (the API
> DTO **is** the matcher's `Plan` type; canonical domain model in its §5).
> Sibling matcher: [plan-matcher](../../plan-matcher-rust-crate/spec/index.md).
> Sibling front-end: [plan-front-end-with-svelte](../../plan-front-end-with-svelte/spec/index.md).

## 1. Purpose and vision

A registry of **plan** records for the Main X Index family — and a
project-management tool. A *plan* is a matchable identity for a
project, product, programme, initiative, portfolio, or epic. The
service has **two faces that share one record**: a deduplicated,
matchable identity registry (the thin `Plan` payload, scored by the
canonical plan-matcher) and a project workspace (each `Plan` *owns*
operational sub-resources — goals, tasks, issues, posts, comments,
members — plus derived timeline / burndown views). Built on loco.rs.

## 2. Scope

MVP: CRUD + `ILIKE` name search + matching + record merge + audit log +
in-memory event streaming (durable-bus Phase 1) + the operational
sub-resources (goals, tasks, issues, posts, comments, members) + derived
timeline / burndown read views + cross-service entity links (write side)
+ OpenAPI/Swagger + Prometheus metrics + offline PASETO v4 public token
verification (Ed25519, published key) + blanket `/api/*` auth enforcement
(off by default) + payload validation.
Deferred (§13): Tantivy full-text/fuzzy search, search-blocked dedup
candidates, durable event bus Phases 2–3 (outbox → Fluvio), privacy,
front-end merge action, paseto-keys-over-HTTP fetch at boot, bulk import/export,
gRPC. Token **issuance** is out of scope — provided by the central
authentication-service. Auth source of truth (supersedes the RS256-JWT
model): [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).

## 3. Stakeholders and users

Portfolio / programme managers and delivery teams curating plans and
working the boards; peer services (person / worker / organization /
authentication, and the cross-service link aggregator); the plan
front-end.

## 4. Glossary

- **plan** — a project / product / programme / initiative / portfolio /
  epic; the matchable identity record.
- **pid** — public UUID of a plan record.
- **data** — the full `Plan` payload stored as JSONB.
- **sub-resource** — operational data a plan owns (goal, task, issue,
  post, comment, member), in its own table keyed by the plan `pid`; **not**
  part of the matcher payload.
- **EntityRef** — `<entity_type>:<uuid>` URN naming a record in another
  service (lead / assignee / author / member / owner org).
- **derived view** — a read-only projection (timeline / burndown)
  computed from goals and tasks; never persisted as its own row.

## 5. Domain model

The API DTO is `plan_matcher::Plan` — the canonical model lives in the
[plan entity spec §5](../../spec/index.md). Matchable payload (thin
identity): `name`, `alternate_names`, `plan_code`, `owner_org_id`,
`owner_org_name`, `plan_type`, `goals` (titles + optional target dates),
`timeframe`, `keywords`, `relationships`, `identifiers`, `same_as`,
`in_language`. **The high-volume operational data is deliberately
excluded from the payload** — tasks, issues, posts, comments, and
members live in their own tables (§10) and are never fed to the matcher.

> **Partition rule — within-payload relationships vs cross-service
> links.** `relationships` inside the `Plan` payload are within-entity
> and **are** a matcher signal (Jaccard component). Cross-service links
> (§9.7) live only in `entity_links`, the `linked`/`unlinked` events, and
> the aggregator — they are **never** stored in the payload and **never**
> fed to the matcher. See
> [cross-service linking §7](../../../agents/share/cross-service-linking.md).

## 6. Functional requirements

1. `POST /api/v1/plans` — create; `name` required, `identifiers`
   structurally checked per scheme (canonical UUID for `Uuid`; external
   PM-tool id shapes for `JiraProjectKey` / `AsanaGid` / `TrelloBoardId` /
   `MsProjectId` / `GitHubProjectId` / `LinearId`; URI shape for `Uri`;
   non-blank for the rest), `goals` titles non-blank, and `in_language`
   checked for BCP-47 syntax; `422` on any problem, **all reported
   together** — also enforced on update. Real-time duplicate detection on
   create returns `409 Conflict` with candidate matches when duplicates are
   found. Rules in [`src/validation.rs`](../src/validation.rs).
2. `GET /api/v1/plans` — list active (cap 100), `{pid, name}`.
   `GET /api/v1/plans/search?q=` — case-insensitive name search (Postgres
   `ILIKE`, cap 50; blank `q` → `400`).
3. `GET /api/v1/plans/{pid}` — return the stored `Plan`.
4. `PUT /api/v1/plans/{pid}` — replace the payload (`422` if `name` is
   blank, or any `goals` / `identifiers` / `in_language` entry is malformed).
5. `DELETE /api/v1/plans/{pid}` — soft-delete.
6. `POST /api/v1/plans/match` — rank an explicit `{query, candidates}`
   set (no persistence).
7. `POST /api/v1/plans/check-duplicates` — match a query against stored
   active plans; return those above threshold, ranked.
   `POST /api/v1/plans/deduplicate` — batch scan of active rows into the
   review queue (status `Pending`/`Confirmed`/`Rejected`/`AutoMerged`).
8. `POST /api/v1/plans/merge` — fold a duplicate into a survivor (union
   payload fields, former-name alias, soft-delete the duplicate,
   `merge_records` history + transferred snapshot, `Replaces` link from
   survivor → duplicate, `Merged` event); `422` equal pids, `404` unknown.
   `GET /api/v1/plans/merges/recent` — merge history.
9. Operational sub-resource CRUD (separate tables keyed by plan `pid`):
   - **Goals** — `…/{pid}/goals` (also part of the matcher payload).
   - **Tasks** — `…/{pid}/tasks` (`status` Todo/InProgress/InReview/
     Done/Blocked; `assignee_ref` EntityRef; `goal_id?`, `parent_task_id?`,
     `estimate`, `remaining`, `due_date`).
   - **Issues** — `…/{pid}/issues` (`kind` Bug/Risk/Blocker/Question/
     Improvement; `severity` Low/Med/High/Critical; `status` Open/
     InProgress/Resolved/Closed; `assignee_ref`).
   - **Posts** — `…/{pid}/posts` (`author_ref`, `title`, `body_markdown`).
   - **Comments** — `…/{pid}/comments` (`target` = post|task|issue + id;
     `author_ref`, `body_markdown`).
   - **Members** — `…/{pid}/members` (`user_ref` into authentication /
     person; `role` Owner/Lead/Member/Viewer).
   Full CRUD on each; every write audits + emits a `created`/`updated`/
   `deleted` event scoped to the sub-resource.
10. Derived read views — `GET /api/v1/plans/{pid}/timeline` (goals-with-
   target-date milestones + task date ranges → Gantt) and
   `GET /api/v1/plans/{pid}/burndown` (remaining-vs-estimate over time from
   task estimate/remaining snapshots). Read-only projections; never
   persisted as their own row.
11. Cross-service links — `POST`/`GET`/`DELETE /api/v1/plans/{pid}/links`
   (§9.7), emitting `linked` / `unlinked`.
12. `GET /api/v1/plans/audit/recent` + `/{pid}/audit` — audit-log query;
   `GET /api/v1/plans/events/recent` — in-memory event stream. Each
   create/update/delete/merge (plan and sub-resource) writes an
   `audit_logs` row and publishes a `created`/`updated`/`deleted`/`merged`
   (and `linked`/`unlinked`) event.
13. `GET /api/v1/plans/whoami` — echo verified bearer-token claims (`401`
   without a valid token); proves offline PASETO verification.
14. `GET /api-docs/openapi.json` + `GET /swagger-ui` — OpenAPI 3 document
   and a Swagger UI page rendering it.
15. `GET /metrics.prom` — Prometheus metrics in text-exposition format
   (`Content-Type: text/plain; version=0.0.4`), mounted at the root (not
   under `/api`) and public under blanket auth enforcement so a scraper
   needs no token. Exposes plan CRUD/merge counters
   (`plan_created_total` / `_updated_total` / `_deleted_total` /
   `_merged_total`) plus `http_requests_total`.
16. Bulk import/export (deferred, §13) — async, job-based, on the loco
   `bg_pg` worker: the five endpoints (§9.8). The uniform family contract
   (execution model, JSONL/CSV/Parquet codecs, upsert-by-stable-key +
   dedupe-to-review, per-row error report, export masking + audit) is fixed
   in [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md);
   plan-specific bits are in §9.8 / §10.

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic + probabilistic
matching via the embedded `plan-matcher` (`MatchingEngine::new(
MatchConfig::default())` in [`src/controllers/plans.rs`](../src/controllers/plans.rs));
soft-delete with audit-friendly timestamps. Sub-resource tables are
sized for high-volume operational churn and are kept off the matcher
hot path (the matcher only ever sees the thin JSONB payload).

## 8. Architecture

loco `App` (`src/app.rs`) registers the plans controller and the
sub-resource controllers. One `plans` table stores `pid` + denormalised
`name` + the full `Plan` JSONB `data`; the operational sub-resources live
in their own tables keyed by the plan `pid` (§10). Matching calls
`plan-matcher` directly on the deserialised payloads — **no adapter**,
mirroring care-pathway. Cross-service links use the hybrid topology
([cross-service linking §4](../../../agents/share/cross-service-linking.md)):
the service writes its outbound edges locally and emits events; a
standalone aggregator builds the queryable graph.

## 9. API surface

Raw loco JSON under `/api/v1/`. `404` for unknown `pid`; `422` for a
validation failure (blank `name`, a malformed `goals` / `identifiers` /
`in_language` entry — family convention, via `Error::CustomError(
StatusCode::UNPROCESSABLE_ENTITY, …)`, every problem in one body); `400`
for a malformed body; `409 Conflict` for a real-time create duplicate.

### 9.1 Plan CRUD

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/v1/plans` | Create (`409` on duplicate) → `{pid, name}` |
| GET | `/api/v1/plans` | List active (cap 100) |
| GET | `/api/v1/plans/search?q=` | Case-insensitive name search (`ILIKE`, cap 50) |
| GET | `/api/v1/plans/{pid}` | Fetch the stored `Plan` |
| PUT | `/api/v1/plans/{pid}` | Replace payload |
| DELETE | `/api/v1/plans/{pid}` | Soft-delete |

### 9.2 Match / dedupe / merge

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/v1/plans/match` | Rank `{query, candidates}` (no persistence) |
| POST | `/api/v1/plans/check-duplicates` | Match a query vs stored active plans |
| POST | `/api/v1/plans/deduplicate` | Batch scan active rows → review queue |
| POST | `/api/v1/plans/merge` | Merge a duplicate into a survivor (`422` equal pids, `404` unknown) |
| GET | `/api/v1/plans/merges/recent` | Merge-history records |

### 9.3 Sub-resources (keyed by plan `pid`)

| Resource | Base path | Notable fields |
|---|---|---|
| Goals | `/api/v1/plans/{pid}/goals` | title, target_date (also in payload) |
| Tasks | `/api/v1/plans/{pid}/tasks` | title, assignee_ref, status, goal_id?, parent_task_id?, estimate, remaining, due_date |
| Issues | `/api/v1/plans/{pid}/issues` | title, kind, severity, status, assignee_ref |
| Posts | `/api/v1/plans/{pid}/posts` | author_ref, title, body_markdown |
| Comments | `/api/v1/plans/{pid}/comments` | target (post\|task\|issue + id), author_ref, body_markdown |
| Members | `/api/v1/plans/{pid}/members` | user_ref, role (Owner/Lead/Member/Viewer) |

Each base path supports `POST` (create), `GET` (list), `GET /{sub_pid}`
(fetch), `PUT /{sub_pid}` (update), `DELETE /{sub_pid}` (soft-delete).

### 9.4 Derived read views

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/v1/plans/{pid}/timeline` | Goals-milestone + task date ranges → Gantt |
| GET | `/api/v1/plans/{pid}/burndown` | Remaining-vs-estimate over time |

### 9.5 Audit / events / docs / metrics

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/v1/plans/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/v1/plans/events/recent` | In-memory event stream |
| GET | `/api/v1/plans/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (root path, public under auth enforcement) |

### 9.6 Auth

Every route may carry `Authorization: Bearer <token>` (a PASETO v4 public
token, verified offline against the auth-service's published Ed25519 key);
handlers take `MaybeAuthUser`
to stamp the audit `actor`. Members, assignees, and authors are user
identities from the authentication-service. Blanket `/api/*` enforcement
is wired (an `after_routes` middleware calling `auth::enforce`) but **off
by default** — gated by `PLAN_REQUIRE_AUTH` (`1`/`true`/`yes`/`on` ⇒ on).
When on, any `/api/*` route without a valid token is `401`; the public
paths `/_health`, `/_ping`, `/api-docs/openapi.json`, `/swagger-ui*`, and
`/metrics.prom` stay open (via `src/auth.rs::is_public_path`).
Keys/issuer/audience come from `PLAN_PASETO_KEYS` (the auth-service's
published Ed25519 public-key set; absent ⇒ empty key set, all tokens
rejected) / `PLAN_TOKEN_ISSUER` (default
`authentication-service`) / `PLAN_TOKEN_AUDIENCE` (default `main-x-service`).
Auth source of truth (supersedes the RS256-JWT model):
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md);
blanket-enforcement contract: [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md).

### 9.7 Cross-service entity links (write side)

Per [cross-service linking](../../../agents/share/cross-service-linking.md),
the Plan Service originates outbound cross-service edges from a plan (or a
goal / task / issue) to records in **any** index entity (case / course /
place / person / worker / organization / …). Edges are stored in a
dedicated `entity_links` table (§10), **separate** from the within-payload
`relationships` and **never** read by the matcher (the partition rule, §5).

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/v1/plans/{pid}/links` | Create / upsert an outbound edge; emits `linked` |
| GET | `/api/v1/plans/{pid}/links` | List this plan's outbound edges |
| DELETE | `/api/v1/plans/{pid}/links/{id}` | Soft-delete an edge; emits `unlinked` |

The write is **optimistic** — it stores the assertion and emits an event;
it does **not** call the target service. Verification status is the
aggregator's concern (it sees both ends).

### 9.8 Bulk import/export (deferred, §13)

The five uniform endpoints
([`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md) §4):

```
POST /api/v1/plans/import         202 {job_id}
GET  /api/v1/plans/import/{id}     status + counts + errors_url + review_url
POST /api/v1/plans/export         202 {job_id}
GET  /api/v1/plans/export/{id}     status + download_url
GET  /api/v1/plans/bulk-jobs       list (filter by kind/status)
```

**Stable upsert key** = a deterministic external PM identifier
(`JiraProjectKey` / `AsanaGid` / `TrelloBoardId` / `MsProjectId` /
`GitHubProjectId` / `LinearId` / `Uri` / `Uuid`) or owner-scoped
`plan_code` (with `owner_org_id`, same-owner only) or `pid`; keyless rows
run the normal duplicate detection → review queue (`provenance = import`).
CSV flattens every repeated/nested field to a JSON-in-cell. Export is
business/ops data (light masking) but **member / person refs are personal
data**, so any export carrying them is audited (even zero-row).

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. `auto_migrate` on in
development. Tables / migrations:

- `m20220101_000001_plans` — the `plans` row: `id` serial PK, `pid` UUID
  unique, `name` (denormalised), `data` JSONB (full `Plan`), `active`
  bool, `deleted_at` timestamptz null.
- `m20220101_000002_audit_logs` — the CRUD audit trail.
- `m20220101_000003_merge_records` — record-merge history + snapshot.
- `m20220101_000004_tasks` / `…_issues` / `…_posts` / `…_comments` /
  `…_members` / `…_goals` — operational sub-resource tables, each keyed by
  the plan `pid` (FK-local), with their own `pid`, soft-delete, and
  created/updated timestamps. **Excluded from the matcher payload.**
- `m20220101_000005_entity_links` — the cross-service link write side
  (§9.7), separate from the within-payload `relationships`, per
  [cross-service linking §4.1](../../../agents/share/cross-service-linking.md).
- `bulk_jobs` (deferred, §13) — the shared bulk-job table
  ([`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md) §3,
  `UNIQUE (entity, kind, idempotency_key)`).
- `review_queue` — duplicate-review items (`Pending` / `Confirmed` /
  `Rejected` / `AutoMerged`, `provenance`).

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON round-trip),
the `src/validation.rs` unit tests (UUID / PM-tool-id / URI identifier
shapes, blank goal titles, BCP-47 `in_language` syntax), the `src/auth.rs`
unit tests (mint a real PASETO v4 public token + matching Ed25519 key
in-process, then
assert valid → claims and missing / non-bearer / expired / tampered /
empty-verifier → `401`; plus `parse_bool` and `enforce` — off+no-token →
`Ok`, on+public → `Ok`, on+protected+{no/valid/expired/tampered} →
`401`/`Ok`), the `src/merge.rs` unit tests (former-name alias, scalar
fallback, list union, transferred snapshot, `is_self_merge` equal-pid
pin), the `escape_like` unit test, the `src/metrics.rs` unit tests
(rendered text carries every metric name + `# HELP`/`# TYPE` preamble +
content type), and the derived-view unit tests (timeline ordering,
burndown remaining-vs-estimate from snapshots). Request-level tests
(`tests/requests/plans.rs`, loco testing harness) cover Plan CRUD + match
+ dedupe + merge, the sub-resource CRUD, the timeline/burndown views,
`/links`, unknown-pid `404` on GET / PUT / DELETE (and the merge `404`),
the `409` real-time create duplicate, the audit/event trail, `whoami` (no
token → `401`), blanket enforcement (with `PLAN_REQUIRE_AUTH=1` in-test:
un-authed `GET /api/v1/plans` → `401`, public `GET /api-docs/openapi.json`
→ `200`; `#[serial]`), and OpenAPI/Swagger — these need Postgres, so they
are `#[ignore]`-gated; run with `cargo test -- --ignored` and a
`DATABASE_URL`.

## 12. Compliance

Plans are business / operational data (no patient data), so the default
posture is light masking. However, **member / person refs are personal
data** — assignees, authors, leads, and members are user identities —
so any read or export that surfaces them is audited, and GDPR
data-subject obligations apply to those people. The `entity_links`
edges follow the [cross-service linking §10](../../../agents/share/cross-service-linking.md)
governance for whatever target entity they reference. Honour the family
HIPAA/NHS/GDPR posture for audit and access controls.

## 13. Tasks (live work queue)

- [ ] loco boot + `plans` table/migration + CRUD with `422` validation
  (blank `name`; UUID / PM-tool-id / URI `identifiers` shapes; blank goal
  titles; BCP-47 `in_language` syntax — all problems reported together)
  + real-time create duplicate detection (`409`).
- [ ] Matching — embed `plan-matcher` (`MatchingEngine::new(
  MatchConfig::default())`); `POST /match`, `POST /check-duplicates`
  (scan active rows), `POST /deduplicate` (batch → review queue).
- [ ] Name search — `GET /search?q=` Postgres `ILIKE` on the denormalised
  `name` (cap 50, wildcards escaped). Tantivy full-text / fuzzy deferred.
- [ ] Operational sub-resources — `tasks` / `issues` / `posts` /
  `comments` / `members` / `goals` tables + CRUD controllers, each keyed
  by the plan `pid`, audited, emitting scoped events.
- [ ] Derived views — `GET /{pid}/timeline` (Gantt) + `/{pid}/burndown`
  from task estimate/remaining snapshots; pure projection logic + unit
  tests.
- [ ] Record merge — `POST /merge` (union fields, former-name alias,
  soft-delete, `merge_records` history + snapshot, `Replaces` link,
  `Merged` event); `/merges/recent`; pure `src/merge.rs`.
- [ ] Event streaming + audit log on CRUD (plan + sub-resources) —
  `audit_logs` table + best-effort row per write; in-memory event stream;
  `/audit/recent`, `/{pid}/audit`, `/events/recent`. **Phase 1** of the
  durable event bus (canonical versioned `Envelope` + `EventPublisher`
  seam + `InMemoryPublisher` ring buffer; frozen `EventView` projection);
  Phases 2–3 (outbox → Fluvio) remain infra-gated roadmap, designed in
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md).
- [ ] Cross-service links — `m20220101_000005_entity_links` migration +
  `POST`/`GET`/`DELETE /api/v1/plans/{pid}/links`; emit `linked` /
  `unlinked`; optimistic write (no cross-service call); never a matcher
  signal. Contract:
  [`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md).
- [ ] OpenAPI/Swagger — `src/openapi.rs` (utoipa) served at
  `/api-docs/openapi.json` + `/swagger-ui` by `controllers/docs.rs`.
- [ ] Prometheus metrics — `GET /metrics.prom` (root path,
  `text/plain; version=0.0.4`); process-wide `OnceLock` registry in
  `src/metrics.rs` (CRUD/merge counters + `http_requests_total`); public
  under blanket auth enforcement.
- [ ] Offline PASETO v4 public verification (`authentication-verifier`);
  switch `src/auth.rs` per
  [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
  (supersedes the RS256-JWT model). `src/auth.rs`
  embeds `authentication-verifier` (now a PASETO v4.public verifier);
  offline Ed25519 verification via a process-wide
  `Verifier` (env-configured keys/issuer/audience); `AuthUser`/
  `MaybeAuthUser` extractors; `/whoami` protected; audit `actor` from the
  token.
  - [ ] Blanket `/api/*` enforcement — `auth::enforce(require_auth, path,
    headers, verifier)` + an `after_routes` layer, gated per-request by
    `PLAN_REQUIRE_AUTH` (off by default). Public paths stay open. Family
    contract: `agents/share/jwt-enforcement.md`.
  - [ ] paseto-keys-over-HTTP fetch from the auth service at boot
    (env-injected today).
- [ ] Privacy — masking of member / person refs on read + export; GDPR
  obligations for those people.
- [ ] Bulk import/export — `bulk_jobs` migration + the five endpoints
  (§9.8); `bg_pg` worker draining `queued → running → completed |
  completed_with_errors | failed`; JSONL/CSV/Parquet codecs (CSV
  JSON-in-cell flattening; Parquet export-only, feature-gated); per-row
  pipeline reusing `src/validation.rs` + the matcher + the review queue
  (upsert by external PM id / owner-scoped `plan_code` / `pid`; keyless →
  dedupe → review, `provenance = import`); per-row error report; export
  masking + audit (member/person refs personal data). Uniform contract:
  [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md).
- [ ] Tantivy full-text/fuzzy search over the JSONB payload.

## 14. Implementation status

**Spec-only; no code yet.** This document and the doc-set (`README.md`,
`AGENTS.md`, `CHANGELOG.md`, `index.md`) are the inaugural scaffold. No
Rust / Cargo crate has been generated; every §13 task is unchecked. The
canonical `Plan` domain model is owned by the
[plan entity spec §5](../../spec/index.md); this crate spec references it.

## 15. Roadmap

`0.1.0` (unreleased) target: the CRUD + matching MVP, then `ILIKE`
search + audit + in-memory streaming, then the operational sub-resources +
derived views + record merge + cross-service links + OpenAPI/Swagger +
Prometheus + offline PASETO v4 public verification + blanket `/api/*`
enforcement (auth source of truth, superseding the RS256-JWT model:
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
Next (deferred, §13): Tantivy full-text/fuzzy search, durable event bus
Phases 2–3 (outbox → Fluvio), paseto-keys-over-HTTP fetch at boot, privacy,
front-end merge action, bulk import/export, gRPC.

## 16. Open questions

- Normalise goals / tasks / members into a search index once Tantivy
  lands, or keep ILIKE-on-name only?
- Should `deduplicate` auto-merge above `auto_merge_threshold`, or always
  route to the review queue?
- Burndown snapshot cadence — on every task write, or a periodic `bg_pg`
  snapshot job?

## 17. References

- The [plan entity spec](../../spec/index.md) (canonical model §5); the
  [plan-matcher spec](../../plan-matcher-rust-crate/spec/index.md);
  loco.rs; [cross-service linking](../../../agents/share/cross-service-linking.md);
  [bulk import/export](../../../agents/share/bulk-import-export.md);
  [event bus](../../../agents/share/event-bus.md).

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`. When
the integration contract changes, also update the
[plan entity spec](../../spec/index.md).
