# Case Service — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test in one PR. Live work queue is §13.
>
> Sibling matcher: [case-matcher](../../case-matcher-rust-crate/spec/index.md).
> Sibling front-end: [case-front-end-with-svelte](../../case-front-end-with-svelte/spec/index.md).

## 1. Purpose and vision

A registry of governmental case records for the Main X Index family:
create/read/update/delete and detect duplicates with the canonical
case-matcher. Built on loco.rs.

## 2. Scope

MVP: CRUD + Tantivy full-text/fuzzy/phonetic search + matching, with
validation, OpenAPI, audit, in-memory streaming, record merge,
field masking + audited GDPR export wired to the ABAC `mask` obligation
(§13 2026-08-02), and
offline PASETO v4 public token verification (Ed25519, via the
auth-service's published key). Deferred (§13): gRPC.
Authentication issuance is out of scope here — provided by the
central authentication-service; this service only verifies. Auth model
source of truth: [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
(supersedes the prior RS256-JWT + JWKS model).

## 3. Stakeholders and users

Agency case-workers and data stewards curating cases; peer services; the
case front-end.

## 4. Glossary

- **case** — an open or historical matter handled by a public agency on
  behalf of one or more subjects (benefit claim, legal action,
  social-services referral, licensing application, complaint, appeal …).
- **pid** — public UUID of a case record.
- **data** — the full `Case` payload stored as JSONB.
- **subject** — an opaque involved-party identifier (e.g. a person pid).

## 5. Domain model

The API DTO is `case_matcher::Case`: `title`, `alternate_titles`,
`case_number`, `agency_id`, `agency_name`, `case_type`, `status`,
`priority`, `opened_date`, `subjects`, `keywords`, `identifiers`,
`same_as`, `in_language`. Enum unit variants serialise as bare
PascalCase strings; `Custom` as `{"Custom":"label"}`.

> **Partition rule — within-entity fields vs cross-service links.** The
> `Case` payload's own fields (`subjects`, `identifiers`, `same_as`, …)
> are within-entity and ARE matcher signals. Cross-service
> `entity_links` (§8.6 — the `subject_of` / `about` edge from a case to a
> person) are **entirely separate**: they are NOT stored in the `Case`
> payload, NOT routed to the matcher, and NOT a match signal. The
> matcher scores two cases' *sameness*; "case is about this person" is
> not sameness evidence. Any future matching adapter MUST NEVER project
> `entity_links` into the matcher input. See
> [cross-service linking §7](../../../agents/share/cross-service-linking.md).

## 6. Functional requirements

1. `POST /api/cases` — create; `title` required, `opened_date` (if
   present) ISO-8601 `YYYY` / `YYYY-MM-DD`, identifier values non-blank,
   `subjects` / `keywords` entries non-blank; `422` on any problem, all
   reported together — also enforced on update. Rules in
   [`src/validation.rs`](../src/validation.rs).
2. `GET /api/cases` — list active (cap 100), `{pid, title}`.
   `GET /api/cases/search?q=` — Tantivy full-text search over title,
   alternate titles, agency name, identifiers, keywords, and subjects
   (`?fuzzy=true` for typo tolerance, `?phonetic=true` for Soundex; blank
   `q` → `400`; an unavailable index → `503`).
3. `GET /api/cases/{pid}` — return the stored `Case`, unless the
   record-level ABAC decision carries the **`mask` obligation**
   (`mask_case`, §9), in which case the masked view (§6.13) is
   returned instead.
4. `PUT /api/cases/{pid}` — replace the payload (`422` on any validation
   problem).
5. `DELETE /api/cases/{pid}` — soft-delete.
6. `POST /api/cases/match` — rank an explicit `{query, candidates}` set
   (no persistence).
7. `POST /api/cases/check-duplicates` — match a query against stored
   cases; return those above threshold, ranked.
8. `POST /api/cases/merge` — fold a duplicate into a survivor (union
   fields, former-title alias, soft-delete the duplicate, `merge_records`
   history, `Merged` event); `422` equal pids, `404` unknown.
   `GET /api/cases/merges/recent` — merge history. The merge writes
   **two** audit rows: a `merged` action against the survivor pid (with
   the merged payload as its `new` value) and a `merged_into` action
   against the duplicate pid (recording that it was folded away); it
   publishes a `Merged` event for the survivor and a `Deleted` event for
   the duplicate.
9. `GET /api/cases/audit/recent` + `/{pid}/audit` — audit-log query;
   `GET /api/cases/events/recent` — in-memory event stream. Each
   create/update/delete writes one `audit_logs` row and publishes a
   `created`/`updated`/`deleted` event; a merge writes two audit rows
   (`merged` on the survivor, `merged_into` on the duplicate — see §6.8)
   and publishes a `merged` event for the survivor plus a `deleted`
   event for the folded-away duplicate. Audit actions across the surface:
   `created`, `updated`, `deleted`, `merged`, `merged_into`.
10. `GET /api/cases/whoami` — echo verified bearer-token claims (`401`
   without a valid token); proves offline PASETO verification.
11. `GET /api-docs/openapi.json` + `GET /swagger-ui` — OpenAPI 3
   document and a Swagger UI page rendering it.
12. `GET /metrics.prom` — Prometheus metrics in text-exposition format
   (`text/plain; version=0.0.4`), mounted at the **root** (not under
   `/api`) and public even under blanket enforcement. Exposes four CRUD
   counters (`case_created_total`, `case_updated_total`,
   `case_deleted_total`, `case_merged_total`) incremented on each
   create / update / delete / merge success, plus an `http_requests_total`
   counter vec (`method`/`path`/`status`). Registry + render live in
   [`src/metrics.rs`](../src/metrics.rs); the handler is
   [`controllers/metrics.rs`](../src/controllers/metrics.rs).
13. `GET /api/cases/{pid}/masked` — the **masked view**: `subjects`,
    `identifiers`, `same_as`, and `case_number` redacted (`mask_case`,
    §9); the descriptive shell (`title`, `case_type`, `status`, …)
    untouched. Regardless of the caller's policy — distinct from the
    `mask` obligation on `GET /{pid}` (§6.3), which is the deployment
    deciding what a caller may see; this is a caller *asking* for the
    redacted form. `404` for an unknown `pid`.
14. `GET /api/cases/{pid}/export` — **GDPR right-of-access** export: an
    envelope of `{entity, pid, exported_at, masked, record, note}`.
    **Every export is audited** via the existing
    `disclosure::action::EXPORT` (HIPAA §164.528), the same accounting
    `GET /{pid}` feeds — extracting a case is itself a compliance event
    whether or not it is masked. A caller whose record-level ABAC
    decision carries the `mask` obligation gets the redacted record and
    `masked: true` — an access request answered with redactions must
    never look like a complete answer. `404` for an unknown `pid`; `503`
    when the export could not be recorded on the audit trail
    (`CASE_AUDIT_FAIL_CLOSED`).

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic matching via the
embedded library; soft-delete with audit-friendly timestamps.

**Configuration (environment).** PASETO keys / verification:
`CASE_PASETO_KEYS_URL` (optional URL of the auth-service's published key
set, e.g. `https://auth…/.well-known/paseto-keys`; set ⇒ fetched over
HTTP **once at boot** in `App::after_routes` via `auth::init` /
`Verifier::from_paseto_keys_url` — on success the fetched key set wins
over `CASE_PASETO_KEYS`, on failure the service logs a warning and falls
back to the env path, so it always boots; no refresh loop — a
rotation-triggered refetch is a future item, §16), `CASE_PASETO_KEYS`
(the auth-service's published Ed25519 public-key set;
absent ⇒ empty key set, all tokens rejected), `CASE_TOKEN_ISSUER`
(default `authentication-service`), `CASE_TOKEN_AUDIENCE` (default
`main-x-service`). Access control:
`CASE_REQUIRE_AUTH` — blanket-enforcement flag, parsed leniently
(`1`/`true`/`yes`/`on`, case-insensitive ⇒ on; unset/blank/other ⇒ off),
**off by default** (see §9); `CASE_ABAC_POLICY` (inline JSON) /
`CASE_ABAC_POLICY_FILE` (path) — the ABAC authorization policy
evaluated inside the guard (see §9; unset or unparsable ⇒ warn-log +
the built-in default policy — read allow / mutation deny — so the
service always boots; read once per process, restart to change).
Durable event bus ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md)
§7): `CASE_EVENT_TRANSPORT` (default `memory`) — `memory` ⇒ the
process-wide ring buffer (Phase 1; no DB, no tx — today's behaviour);
`outbox` ⇒ the transactional outbox (Phase 2): every CRUD/merge handler
writes one `event_outbox` row **on the same transaction** as the entity
mutation, so the change and its event commit or roll back together;
unrecognised value ⇒ `memory` (fail-safe), read once at boot and cached.
`CASE_EVENT_RELAY` (default off; `1`/`true`/`yes`/`on`) — enable the
Phase-3 outbox **relay** background loop (drains unpublished
`event_outbox` rows to a sink, stamps `published_at`, purges old rows);
a no-op unless `CASE_EVENT_TRANSPORT=outbox` **and** this flag is on, so
the default `memory` transport never starts it.
`CASE_EVENT_RELAY_INTERVAL_SECS` (default `5`, floored at `1`) — the
relay poll interval. `CASE_EVENT_RETENTION_DAYS` (default `7`) — outbox
row TTL, **enforced by the relay**: it periodically deletes rows with
`published_at < now() - INTERVAL '<n>'` (§3). Plus loco's own
`DATABASE_URL` etc.

## 8. Architecture

loco `App` (`src/app.rs`) registers the cases controller. One `cases`
table stores `pid` + denormalised `title` + the full `Case` JSONB
`data`. Matching calls `case-matcher` directly on the deserialised
payloads — no adapter.

### 8.6 Cross-service entity links (write side)

Per [cross-service linking](../../../agents/share/cross-service-linking.md),
the Case Service originates outbound cross-service edges to records in
sibling services without calling the target service. The full topology —
shared `EntityRef` URN format, the read-model aggregator, integrity
lifecycle, governance, and the edge-kind registry — is fixed in that
shared doc; this section documents only the **write side that the Case
Service owns**.

Case owns this outbound edge kind in v1
([cross-service linking §9](../../../agents/share/cross-service-linking.md)):

| Kind | From → To | Direction | Card. | Temporal | Sensitivity |
|---|---|---|---|---|---|
| `subject_of` / `about` | case → person | directed | M:N | sometimes (`valid_from`/`valid_to`) | **high** — the edge asserts a person is the subject of a government case (§12) |

Outbound edges are stored in a dedicated `entity_links` table
(§10, migration `m20220101_000004_entity_links`), **separate** from the
within-entity `Case` payload (the partition rule, §5). Per
[cross-service linking §4.1](../../../agents/share/cross-service-linking.md),
each row carries `from_pid` (the local case), `kind`, `to_ref` (the
target `EntityRef` URN, e.g. `person:0c4f…`), optional `role`,
`confidence`, `provenance`, and `valid_from`/`valid_to`, with a soft
`deleted_at` and the idempotent `UNIQUE (from_pid, kind, to_ref,
valid_from)` upsert key.

REST surface (three endpoints under the existing case resource,
mirroring the controller style above):

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/api/cases/{pid}/links` | Create / upsert an outbound edge; emits `linked` |
| `GET` | `/api/cases/{pid}/links` | List this case's outbound edges |
| `DELETE` | `/api/cases/{pid}/links/{id}` | Soft-delete an edge; emits `unlinked` |

The write path is **optimistic**:

**Link:** HTTP POST `/api/cases/{pid}/links` → authorise (§9) →
validate edge kind + `to_ref` → upsert into `entity_links` → publish
`linked` event → audit → Response. No cross-service call, so latency and
availability are unaffected by the target service's state.

**Unlink:** HTTP DELETE `/api/cases/{pid}/links/{id}` → authorise →
soft-delete the row (`deleted_at`) → publish `unlinked` event → audit →
Response.

The `linked` / `unlinked` events are two new `kind` values on the
**existing** event envelope and reuse the same `EventPublisher` / outbox
path — no new transport
([cross-service linking §4.2](../../../agents/share/cross-service-linking.md)).
The envelope's `entity`/`pid` are the **from** (case) side; the edge
detail (`edge_id`, `from_ref`, `to_ref`, `edge_kind`, `role`,
`confidence`, `provenance`, `valid_from`/`valid_to`) rides in `data`.
Verification status (`unverified` / `verified` / `dangling`) is not
returned here — it is the aggregator's read-model concern.

Cross-service links are never read by the matcher (the partition rule,
§5; [cross-service linking §7](../../../agents/share/cross-service-linking.md)).

### 8.7 Bulk import / export

**Status: landed (BLK-5, 2026-08-03).** The uniform async, job-based
bulk contract is fixed family-wide in
[bulk import / export](../../../agents/share/bulk-import-export.md)
(execution model on the loco `worker` feature, the `bulk_jobs` table,
the five endpoints, import dedupe semantics, the per-row error report,
and the export privacy/audit posture). This section declares what the
Case Service differs on (per
[bulk import / export §10](../../../agents/share/bulk-import-export.md))
and records the scope this rollout deliberately did not cover.

**Formats: JSONL + CSV only.** No Parquet and no S3 artifact store in
this rollout — those were person-specific extras (its BLK-3/BLK-4)
built *after* CSV/review-routing, and this task's dependency was only on
the CSV/review-routing steps (BLK-1/BLK-2). `src/bulk/store.rs`'s
`ArtifactStore` trait is nonetheless **async** (care-pathway's shape,
not person's original synchronous one) specifically so a future S3
rollout is additive rather than a breaking signature change; asking for
`s3` today (`CASE_BULK_ARTIFACT_BACKEND=s3`) is a clear, named error,
never a silent fallback to local disk.

**Stable key(s) for upsert (import idempotency).** A row upserts in
place when it carries a key that uniquely identifies an existing case;
otherwise it runs duplicate detection (the same search-blocked
`case-matcher` path `check-duplicates` uses) and routes a likely
duplicate to the review queue with `provenance = import`. Keys, in
priority order:

1. **Agency-scoped case number** — the `(agency_id, case_number)` pair,
   both present and non-blank. A case number is unique only *within* its
   agency (case-matcher's own deterministic short-circuit), so the pair
   is the key — never `case_number` alone, and never `agency_id` alone.
2. **`pid`** — the case's public UUID, when the bulk row names one. This
   is what makes re-importing an *export* (which always carries pids)
   idempotent even for a case with no case number recorded.

Unlike the person reference implementation's stable key, there is
**no third, deterministic-identifier tier** (an earlier planning draft
of this section proposed one, keyed on `identifiers` schemes
`Docket`/`ExternalCaseId`/`Uri`/`Uuid`) — it was dropped as out of this
task's bound rather than built partially; a future rollout can add it
as a policy decision, not a technical blocker.

`case_matcher::Case` carries **no `pid` field of its own** — the bulk
wire type (`src/bulk/row.rs::BulkCaseRow`) wraps it with an out-of-band,
genuinely optional `pid: Option<Uuid>` (`#[serde(flatten)]`, no
fabricated default), so a JSONL/CSV row reads as `{"pid": "...",
"title": "...", …}`. This is a deliberate simplification versus
person's own stable-key module: because `pid` has no fabricated serde
default, "no pid given" and "pid present" are never ambiguous, so there
is no need for person's raw-line `row_has_explicit_id` byte-sniff.

`same_as` URL overlap is a matcher short-circuit but is **not** used as
a bulk upsert key (it is a sameness signal, not a stable record
identity); keyless rows fall through to duplicate detection.

**CSV column set + flattening** (per
[bulk import / export §5](../../../agents/share/bulk-import-export.md);
JSONL is the lossless reference — prefer it when fidelity matters):

- **Scalar columns** (one each): `pid` (the wire envelope's field, not
  part of `Case`), `title`, `case_number`, `agency_id`, `agency_name`,
  `opened_date`.
- **`case_type` / `status` / `priority` are JSON-encoded, not
  scalar** — each is an enum carrying a data-bearing `Custom(String)`
  variant, which `serde` externally tags as a JSON *object*
  (`{"Custom":"foo"}`) rather than a bare string; a plain scalar cell
  could not distinguish a unit variant's name from a `Custom` payload,
  or round-trip the object shape at all. This is a deliberate departure
  from an earlier planning draft of this section (and from person's
  `gender`, whose enum has no data-bearing variant) that listed them as
  plain scalar columns.
- **Arrays / arrays-of-objects → a single JSON-encoded cell each**:
  `alternate_titles`, `subjects`, `keywords`, `identifiers`
  (`[{scheme,value}, …]`), `same_as`, `in_language`.
- `case_matcher::Case` has **no single nested object** needing dotted
  columns the way person's `name` or organization's `address` do.
- **Cross-service `entity_links`** (the `subject_of` / `about` edges,
  §8.6) are **not** part of the `Case` payload export; per
  [bulk import / export §9](../../../agents/share/bulk-import-export.md)
  they are an **optional separate** link-import/export job (not built
  here) and, being the highest-governance kind (§12.1), are never
  bundled into the default case export.

**Export sensitivity — PROMINENT (cases are personal/sensitive
government data; ties to [§12](#12-compliance)).** A bulk extract of
case data is itself a compliance event (HIPAA / NHS / GDPR), so export
is governed more strictly than for non-personal entities:

- **Masked by default, reusing the existing masking, not a new
  rule.** The overview capability matrix's "Privacy masking module
  (`src/privacy`)" column marks case **absent**, and that is accurate
  for a *dedicated module* — but masking logic already existed, inline
  as `controllers::cases::mask_case` (the function behind
  `GET /{pid}/masked` and the masked branch of `GET /{pid}/export`).
  Bulk export's default `masked` profile calls this exact function on
  every exported record (redacting `subjects` / `identifiers` /
  `same_as` / `case_number`) rather than inventing a second redaction
  rule as a side effect of this task.
- **Full / unmasked export requires elevated authorisation** — the
  caller must clear `authorize_record(…, Action::Destructive, …)`
  (mirroring the person reference implementation's export-elevation
  gate), checked **synchronously at submission time** against the live
  caller. `include_soft_deleted` defaults `false` and, like person's
  reference implementation, is rejected as not-yet-supported rather
  than silently leaking or ignoring the flag.
- **Known, documented gap: no per-row record-level ABAC inside the
  async export worker.** `GET /api/cases` (list) and
  `GET /api/cases/search` apply per-row SEC-G3 concealment
  (`resource.case_type`/`status`/`priority`-gated policy) against the
  *live* caller's verified claims. The bulk export **worker** does not
  re-apply this, because it runs asynchronously with no live HTTP
  request or bearer token to evaluate a record-level decision against —
  synthesising `Claims` from data stored at submission time, rather
  than an actually-verified token, would itself be an unverified
  privilege-check bypass path, which is a worse defect than not
  building the feature. This matches the person reference
  implementation, which has the same limitation despite having
  record-level ABAC as a general capability — so it is a family-wide
  gap, not a case-specific shortcut. **Follow-up:** closing it needs
  either a way to carry a verified caller identity into an async job
  (a short-lived, job-scoped credential the worker can present back to
  the ABAC engine) or a synchronous-only privileged-export path.
- **Every export is audited, unconditionally, and the audit write
  gates delivery (SEC-B8).** Unlike the opt-in, default-off
  `CASE_AUDIT_READS` read-auditing this crate already had (§12.0's
  `compliance::disclosure` module), a bulk export's audit row is always
  written via `models::audit_logs::Model::record` — actor, filter,
  format, row count, masking profile, timestamp — even for a zero-row
  export, and the write is recorded **before** the job is marked
  `completed`: a failed audit write fails the whole job (no
  `download_url` is ever surfaced for an unaccounted-for export). An
  **import**'s job-level audit row, by contrast, stays best-effort
  (logged on failure, never blocks the job) — every row it writes
  already carries its own per-row audit from
  `streaming::create_and_emit`/`update_and_emit`, so a failure to write
  the *summary* row does not by itself leave anything unaccounted for.
- **Filtered, not all-or-nothing** — exports reuse the existing title
  search / list query, so they are scoped. The single-subject special
  case (filter = one `pid`) is the per-case GDPR export
  (`GET /{pid}/export`), unchanged by this work.

**Concurrency — documented, not half-built.** The family contract's
SEC-B3 concern (two concurrent importers of the same stable key must
produce exactly one record) is **not** implemented here as a true
cross-request lock: `streaming::create_and_emit`/`update_and_emit` each
own their transaction internally (for the `outbox` event transport) and
offer no hook to nest an externally-held Postgres advisory lock around
that write without duplicating their event/audit/index logic in
`src/bulk/`. A lock that only wrapped the read-then-decide step would
not actually close the create-vs-update race it exists to prevent, so
rather than ship that (worse than no lock — it *looks* safe), this
rollout implements and tests **sequential idempotency only**:
re-running the same file twice upserts in place, which is what
`import_is_idempotent_on_the_agency_case_number_key` (§11) pins. True
concurrent-importer race safety is a follow-up needing either a
`ConnectionTrait`-generic `create_and_emit`/`update_and_emit` variant or
a dedicated transaction-scoped bulk write path.

**New tables.** `bulk_jobs` (loco-idiomatic `sea-orm-migration`, one
`SeaORM` entity — modelled on care-pathway's own `bulk_jobs` migration,
which is closer to this crate's layout than person's) and — new to this
crate — `review_queue` (raw-SQL migration, modelled on
organization's, with `provenance` in the initial schema rather than a
follow-up migration, since there was no pre-existing table to keep
backward-compatible).

## 9. API surface

See §6. Raw loco JSON. `404` for unknown `pid`; `422` for a validation
failure (blank `title`, malformed `opened_date`, blank identifier value,
or blank `subjects` / `keywords` entry — family convention, via
`Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`, with every
problem reported in one body); `400` for a malformed body.

**Authentication / blanket enforcement.** Offline PASETO v4 public token
verification (Ed25519; `src/auth.rs`, embedding `authentication-verifier`)
underpins the `AuthUser` / `MaybeAuthUser` extractors. When
`CASE_REQUIRE_AUTH` is on,
an Axum `from_fn` middleware wired in `App::after_routes` (delegating to
the pure `auth::enforce(require_auth, method, path, headers, verifier,
policy)`) rejects
every non-public request lacking a valid bearer token with `401`;
`/_health`, `/_ping`, `/api-docs/openapi.json`, `/swagger-ui*` and
`/metrics.prom` stay public. The flag is read once per process and the layer is always wired,
so it is a near-noop when off. Enforcement is **off by default**;
because case data is personal data, this blanket gate is the
access-control boundary in front of the case API once activated (an
operations decision taken with the family SSO rollout). The contract is
the family-wide [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md);
the credential is now a PASETO token per
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
(source of truth; supersedes the RS256-JWT model).

**Authorization (ABAC).** Inside the same guard — so only when
`CASE_REQUIRE_AUTH` is on — a verified token is authorized by
**attribute-based access control** per
[`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md):
the request's action is derived from the HTTP method plus this crate's
destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`,
`/deduplicate`, `/import`; the latter two ahead of the dedup-scan and
bulk-import features), and the shared engine in
`authentication-verifier` 0.3 evaluates the policy over the token's
`attrs` claim, first-match-wins. Configure with `CASE_ABAC_POLICY`
(inline JSON) or `CASE_ABAC_POLICY_FILE` (path); unset or unparsable ⇒
warn-log + the built-in default policy (any authenticated subject
reads; `access=write` writes; `access=admin` adds DELETE/merge;
`svc=true` does everything). `401` = missing/bad credential; `403` =
valid credential, policy denied (the body names the deciding rule).
Because case data is personal data, deployments can express e.g.
department or **purpose-of-use** scoping as configured policy rules
over the same `attrs` claim — configuration, not code.

**Record-level authorization (delivered).** Beyond the coarse blanket
guard, the single-case handlers `GET`/`PUT`/`DELETE /api/cases/{pid}`
run a **second, finer** ABAC pass after loading the record, per
[`authorization-attributes.md`](../../../agents/share/authorization-attributes.md)
§9. `auth::case_resource_attrs` derives the case's classification fields
into resource attributes — `resource.case_type`, `resource.status`,
`resource.priority` (lowercase tokens, e.g. `investigation`, `closed`,
`high`) — and `auth::authorize_record` calls the verifier 0.4
`Policy::evaluate_with_resource` (gated on `CASE_REQUIRE_AUTH`, so it is
a no-op when enforcement is off). A deployment can then write, as
policy, e.g. "deny `write` when `resource.status=closed` unless
`access=admin`" or "deny `read` when `resource.case_type=investigation`
unless `dept=investigations`". `PUT`/`DELETE` evaluate the **stored**
case's attributes (the record being modified), not the incoming payload.
No schema change — these are the case's existing fields; a dedicated
per-case **sensitivity tier** column remains an optional roadmap add.

The same pass supplies **environment attributes** (verifier 0.5
`Policy::evaluate_with_context`, §10): `auth::request_env_attrs` derives
`env.hour` / `env.after_hours` (UTC) at the service edge so a deployment
can add e.g. "deny write when `env.after_hours=true` unless
`access=admin`". Value templates (`$sub` / `$email`) additionally let a
rule express **ownership** (`resource.owner: ["$sub"]`) once a record
carries an owner attribute.

**Mask-on-allow (obligations, verifier 0.6, §11).** An allow rule may
attach `"obligations": ["mask"]`; `authorize_record` returns the
decision's obligations and `GET /api/cases/{pid}` honours `mask` by
returning a **redacted** case (`mask_case` drops `subjects` /
`identifiers` / `same_as` / case number, keeps the descriptive shell) —
so a policy can grant a *masked* read (e.g. cross-department) without a
separate endpoint. This makes ABAC the driver for the case service's
per-record masking (previously a deferred item).

**Cross-service link authorisation (governance).** The `subject_of` /
`about` edge (§8.6) is **sensitive data**: the edge itself asserts a
person is the subject of a government case. Per
[cross-service linking §10](../../../agents/share/cross-service-linking.md),
both **creating** and **reading** such an edge require at least the
authorisation needed to **read the case** — the link endpoints
(`POST`/`GET`/`DELETE /api/cases/{pid}/links`) are never more
permissive than `GET /api/cases/{pid}`. An unauthorised caller must not
even learn that the edge exists (responses do not distinguish "no such
edge" from "not authorised"). This authorisation requirement also
constrains the aggregator: its `single-view` / `neighbors` responses
MUST honour the same authz/masking before surfacing a case→person edge
(see §12). See [§12](#12-compliance) for the audit + masking obligations.

**Bulk import/export endpoints (§8.7, BLK-5).** `POST`/`GET
/api/cases/import[/{id}]`, `POST`/`GET /api/cases/export[/{id}]`, `GET
/api/cases/bulk-jobs` — mounted as a separate route table
(`bulk::handlers::routes()`) alongside `controllers::cases::routes()`.
`POST /import` falls under the same `DESTRUCTIVE_POST_SUFFIXES` /import
entry named above; the privileged export paths (`masking_profile=full`
or `include_soft_deleted=true`) are checked explicitly in the handler
against `Action::Destructive`, not via the coarse method-derived action.

**FHIR R5 endpoints (§13, family contract [`agents/share/fhir.md`](../../../agents/share/fhir.md)).**
`GET /fhir/metadata`, and read/create/update/delete/search at
`/fhir/Task{,/{id}}` (`controllers::fhir::routes()`) — a best-effort
mapping of the stored `Case` to a FHIR `Task` (§13 lists the documented
lossy fields). Sits behind the same blanket auth+ABAC guard as `/api/*`
(not on the public allow-list except `/fhir/metadata`'s discovery role).
Subject-reference masking on the `for`/`focus` element is a documented
open gap (§13), since the crate has no dedicated field-masking module —
`mask_case` lives beside the native controller instead.

**Compliance endpoints (§12.0/§12.0.1).** `GET /api/cases/audit/verify`,
`GET /api/cases/records/verify`, `GET /api/cases/checkpoint`,
`POST /api/cases/checkpoint/verify`, `POST /api/cases/{pid}/erase`,
`GET /api/cases/{pid}/audit/disclosures` (all under the `cases`
controller, alongside the data they verify), plus the service-level
`GET /api/compliance` and `GET /api/compliance/sbom`
(`controllers::compliance::routes()`). `POST /{pid}/erase` is a
declared destructive action (`access=admin`); the rest are reads,
gated only by the coarse blanket guard when it is on.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migrations
`m20220101_000001_cases` (the `cases` table),
`m20220101_000002_audit_logs` (the CRUD `audit_logs` trail),
`m20220101_000003_merge_records` (record-merge history), and
`m20220101_000004_entity_links` (the cross-service link write side, §8.6).
`auto_migrate` on in development.

The `entity_links` table is **separate** from the within-entity `Case`
payload (the partition rule, §5) and is the **outbound** edges only — the
inverse is the far endpoint's concern and the aggregator stores both
directions. Schema per
[cross-service linking §4.1](../../../agents/share/cross-service-linking.md):

```sql
CREATE TABLE entity_links (
    id           UUID PRIMARY KEY,
    from_pid     UUID NOT NULL,          -- local case (FK to cases)
    kind         TEXT NOT NULL,          -- subject_of | about
    to_ref       TEXT NOT NULL,          -- EntityRef URN of the far record (e.g. person:0c4f…)
    role         TEXT,
    confidence   DOUBLE PRECISION,       -- 1.0 operator-asserted; <1 suggested
    provenance   TEXT NOT NULL,          -- operator | import | matcher_suggested
    valid_from   DATE,                   -- subject-of start (nullable)
    valid_to     DATE,                   -- subject-of end (nullable)
    created_at   TIMESTAMPTZ NOT NULL,
    deleted_at   TIMESTAMPTZ,            -- soft-delete (withdrawn edge)
    UNIQUE (from_pid, kind, to_ref, valid_from)   -- idempotent upsert key
);
```

This table is never read by the matcher (the partition rule, §5).

**Bulk import/export tables (§8.7, BLK-5).** `m20260803_000012_review_queue`
(raw SQL; `id`, `record_id_a`/`record_id_b` UUID pair normalized and
UNIQUE, `match_score`, `match_quality`, `detection_method`,
`score_breakdown` JSONB, `status` default `pending`, `provenance`
default `operator`, `reviewed_by`, `created_at`, `reviewed_at`) and
`m20260803_000013_bulk_jobs` (loco-idiomatic `sea-orm-migration`; `id`,
`kind`, `entity`, `format`, `status`, `params` JSONB, the `rows_*`
counters, `actor`, `idempotency_key` with a `UNIQUE (entity, kind,
idempotency_key)` index, `input_url`/`result_url`/`error_report_url`,
`created_at`/`updated_at`/`expires_at`). Neither table is read by the
matcher or by `entity_links`; `review_queue` rows reference `cases.pid`
values but carry no foreign key (mirroring the family's other
review-queue tables), since a queued pair can outlive either side being
merged or soft-deleted.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON
round-trip), the `src/validation.rs` unit tests (title, `opened_date`
formats, blank identifier / subject / keyword), the `src/auth.rs` unit
tests (mint a real PASETO v4 public token + matching Ed25519 key
in-process, then assert
valid → claims and missing / non-bearer / expired / tampered /
empty-verifier → `401`; plus the blanket-enforcement decision —
`parse_bool` truthy/falsey cases and `enforce` off-no-token → `Ok`,
on-public → `Ok` (incl. `/metrics.prom`),
on-protected-no-token / expired / tampered → `401`,
on-protected-valid → `Ok`), the `src/merge.rs` unit tests (former-title
alias, scalar fallback, list union, transferred snapshot), the
`escape_like` unit test (search wildcard neutralisation), the
`src/openapi.rs` unit tests (well-formed doc; core + merge + whoami +
search + metrics endpoints), the `src/metrics.rs` unit tests (`render`
yields valid Prometheus text + the content-type constant), the
`src/streaming.rs` unit test (publish/read-back),
and controller validation unit tests (blank-title and malformed-date →
`422` pins; the `check-duplicates` ranking — `score_desc` ordering
incl. NaN, and a DB-free reproduction of the in-memory scan that pins a
deterministic docket twin scoring `1.0` ahead of an unrelated case).
Request-level tests (`tests/requests/cases.rs`, loco testing
harness) cover the CRUD + match endpoints, the audit/event trail
(including the merge's two audit actions — `merged` on the survivor and
`merged_into` on the duplicate — and the duplicate's `deleted` event),
`whoami` (no token → `401`), blanket enforcement (with
`CASE_REQUIRE_AUTH=1` set in-test: un-authed `GET /api/cases` → `401`,
public `GET /api-docs/openapi.json` → `200`; `#[serial]`), and
OpenAPI/Swagger but require Postgres, so they are `#[ignore]`-gated —
run with `cargo test -- --ignored` and a `DATABASE_URL`.

## 12. Compliance

Cases can hold government and personal data; honour the family
compliance posture (HIPAA/NHS/GDPR) for any audit and access controls
added later. Subjects are stored as opaque identifiers, not embedded PII.

### 12.0 Tamper-evident audit history and read/disclosure auditing

Adopted 2026-07-25 from the family reference implementation in the
care-pathway service, per
[`spec/compliance` §8.5](../../../spec/compliance/index.md) step 3 — the
personal-data services take the audit chain first, because **case data is
personal data** and a case record is *about* someone.

- **Hash chain** ([`src/compliance/audit_chain.rs`](../src/compliance/audit_chain.rs)).
  Migration `m20260726_000006_compliance` adds `prev_hash` / `hash` /
  `context` / `disclosure` / `redacted_at` to `audit_logs`. Each row binds
  its own content **and its predecessor's hash**, so inserting, deleting,
  reordering or editing a row breaks verification there and after
  (HIPAA §164.312(c)). `GET /api/cases/audit/verify` reports the counts,
  every break with its row id and kind, and the chain head. Appends take
  `pg_advisory_xact_lock`; under `CASE_EVENT_TRANSPORT=outbox` they are
  fully serialised, and under `memory` a concurrent-append fork is
  possible and is reported rather than hidden.
- **Read/disclosure auditing** ([`src/compliance/disclosure.rs`](../src/compliance/disclosure.rs)).
  `CASE_AUDIT_READS` (**default off**) audits `get` / `list` / `search`.
  The caller declares context in `X-Purpose-Of-Use` and
  `X-Disclosure-Recipient` (normalised against a closed vocabulary, never
  echoed). On `GET /api/cases/{pid}` the audit row is written **after**
  the record-level authorization decision, so a denied request — which
  disclosed nothing — never enters the accounting. Collection reads are
  recorded against the nil `pid` so they cannot corrupt any single case's
  accounting.
- **Accounting of disclosures** (§164.528).
  `GET /api/cases/{pid}/audit/disclosures` is gated behind the **same
  record-level authorization as reading the case**: learning who a case
  was disclosed to reveals that the case exists, so the accounting cannot
  be more open than the record it describes (linking doc §10). It states
  whether it is complete or `INCOMPLETE` because read-auditing is off.
- **Fail-open vs fail-closed.** `CASE_AUDIT_FAIL_CLOSED` (**default
  off**) decides what happens when an audit write fails: off logs and
  serves the read; on refuses it with `503`, disclosing nothing the
  service cannot account for. A deployment holding real case data should
  set it, along with `CASE_AUDIT_READS` and `CASE_REQUIRE_AUTH`.

### 12.0.1 Row-level integrity, GDPR Art. 17 erasure, external-witness
checkpoints, and the SBOM/identification surface

Landed 2026-07-26/27, the same rollout round as §12.0 (ported from the
care-pathway reference, then propagated to person and worker). All four
close gaps §12.0's original text left open; they are documented here
rather than duplicated per module — see each module's own doc comment
for the full rationale.

- **Row-level record integrity** ([`src/compliance/record_integrity.rs`](../src/compliance/record_integrity.rs),
  migration `m20260727_000007_case_content_hash`). Every `cases` row
  carries a `content_hash` (SHA-256 over its own content and lifecycle
  state), recomputed on every write. This is the audit chain's
  complement, not a duplicate: the chain proves the *trail* was not
  rewritten; this proves the *record* was not edited out of band (an
  attacker with SQL access who edits a stored case and writes no audit
  row defeats the chain but is caught here). `GET /api/cases/records/verify?limit=`
  (capped at 10 000) recomputes and reports `intact` / `unhashed` (rows
  predating the column — neither verified nor a mismatch) / `mismatched`.
  Row deletion leaves no trace here (see checkpoints, below).
- **GDPR Art. 17 erasure** ([`src/compliance/erasure.rs`](../src/compliance/erasure.rs)).
  `POST /api/cases/{pid}/erase` — a **destructive**, `access=admin`-gated
  action (`auth::DESTRUCTIVE_POST_SUFFIXES`), distinct from the soft
  `DELETE /{pid}` (which keeps the data). Erasure replaces the payload
  with a tombstone, soft-deletes the record, withdraws its `subject_of`
  cross-service links (§8.6) — leaving the accusation standing while
  erasing the detail would defeat the point — destroys the `snapshot` of
  every audit row about the case while leaving `hash`/`prev_hash` intact
  (so the chain keeps verifying across the redaction), and appends a
  final chained `erased` accountability row. Irreversible; idempotent on
  an already-erased or already soft-deleted `pid` (a right does not lapse
  because the record was already retired).
- **External-witness checkpoints** ([`src/compliance/checkpoint.rs`](../src/compliance/checkpoint.rs)).
  The hash chain cannot detect **tail** deletion (delete the last *N* rows
  and there is no successor left to break). `GET /api/cases/checkpoint`
  takes a signed statement of the chain's current head/position/depth
  (also emitted as an `INFO` log line, `target: "audit_checkpoint"`, so a
  deployment shipping logs off-host has a witness for free) — it must be
  **stored outside this service's database** to be worth anything.
  `POST /api/cases/checkpoint/verify` checks a previously-taken checkpoint
  against the current chain and reports whether it is still honoured.
  `204` when the chain is empty.
- **Keyed integrity MAC** ([`src/compliance/mac.rs`](../src/compliance/mac.rs),
  embedding the shared [`integrity-mac`](../../../integrity/integrity-mac-rust-crate)
  crate; migration `m20260727_000011_integrity_mac`, superseding an
  interim BLAKE3 digest added and dropped the same day —
  `m20260727_000008`/`_000010`). An HMAC-SHA256 tag over the audit-chain
  and record-integrity pre-images, keyed by `CASE_INTEGRITY_MAC_KEY` /
  `_KEY_FILE` / `_KEY_ID` / `_KEYS_RETIRED`, HKDF-subkeyed per
  (service, domain) so a tag cannot transfer between purposes or
  services. **Default off**: no key configured ⇒ no MAC written, and
  existing rows report *unverifiable* rather than *mismatched* — the
  control an attacker with database access but not the key cannot forge,
  distinct from what a hash alone defends against.
- **SBOM / build-provenance / service-identification** ([`src/compliance/soup.rs`](../src/compliance/soup.rs),
  [`src/compliance/mod.rs`](../src/compliance/mod.rs) `Build`,
  [`controllers/compliance.rs`](../src/controllers/compliance.rs)).
  `GET /api/compliance` reports version, source commit, `SOURCE_DATE_EPOCH`
  presence, and whether the running binary qualifies as a reproducible
  release, plus an explicit `not_claimed` list (this is a governmental
  case registry, not IEC 62304 health software; not an FD&C §524B cyber
  device). `GET /api/compliance/sbom` serves a CycloneDX 1.5 SBOM + the
  SOUP register (`compliance/soup.tsv`), derived from `Cargo.lock` at
  compile time so it cannot drift from the running binary. Both sit
  behind the same blanket guard as `/api/*` (deliberately not public —
  the SBOM names exact dependency versions, which is exactly what an
  attacker needs to match a deployment against published advisories).

**Still not adopted**: the GDPR residency / lawful-basis / Art. 9
declarations (`disclosure.rs` explicitly does not record `residency` /
`lawful_basis` / `art9_condition` / `transfer_safeguard` — see its own
tests), and the FHIR **ONC/HTI conformance machinery** specifically —
US Core profile validation, terminology/value-set binding, `$validate`,
SMART discovery, and Bulk Data `$export`
([`agents/share/fhir.md`](../../../agents/share/fhir.md) §2.3 /
[`compliance-for-healthcare.md`](../../../agents/share/compliance-for-healthcare.md)
§2.3). This is narrower than "no FHIR" — the base FHIR R5 `Task` CRUD +
search surface is landed (§13, §9) — it is the certification-grade
conformance layer on top that remains open. `compliance/lifecycle.md`
and `compliance/traceability.tsv` (which care-pathway, the reference,
already carries) are also not yet ported here.

### 12.1 Cross-service `case ↔ person` link governance

The `subject_of` / `about` edge (§8.6) is the **highest-governance** v1
cross-service kind
([cross-service linking §10](../../../agents/share/cross-service-linking.md)),
because the edge *itself* is sensitive data: it asserts a named person is
the subject of a government case. It therefore carries the case service's
full compliance posture, not the lighter affiliation posture of the other
edge kinds. The implementation MUST enforce:

- **Access control on create AND read.** Both writing and reading a
  `subject_of` edge require at least the authorisation needed to read the
  case (§9). The link endpoints are never more permissive than
  `GET /api/cases/{pid}`. An unauthorised caller MUST NOT learn the edge
  exists — a denied read is indistinguishable from "no such edge".
- **Audit every read and write of these edges.** Each `POST`/`GET`/
  `DELETE` on `/api/cases/{pid}/links` — and any `single-view` that
  surfaces such an edge — writes an `audit_logs` row, consistent with the
  case service's existing CRUD audit trail (§6.9), with the `actor`
  stamped from the verified token.
- **Privacy masking.** The edge is sensitive data subject to masking. The
  aggregator's `single-view` / `neighbors` responses MUST honour the same
  masking and authorisation as the case service before surfacing a
  case→person edge; the edge is suppressed entirely for unauthorised
  callers rather than masked-but-present.

These obligations are why `case ↔ person` is governed more strictly than
the other v1 edge kinds even though it shares the same edge shape.

## 13. Tasks (live work queue)

- [x] **2026-07-25..27 — Compliance: tamper-evident audit chain,
  read/disclosure auditing, row-level record integrity, GDPR Art. 17
  erasure, external-witness checkpoints, keyed integrity MAC, and the
  SBOM/identification surface.** Ported from the family reference
  implementation in care-pathway, per `spec/compliance` §8.5 step 3 (the
  personal-data services first, since case data is personal data). Full
  detail lives in §12.0/§12.0.1 rather than duplicated here; six new
  endpoints landed (`GET /audit/verify`, `GET /records/verify`,
  `GET /checkpoint`, `POST /checkpoint/verify`, `POST /{pid}/erase`,
  `GET /{pid}/audit/disclosures`, plus service-level `GET /api/compliance`
  and `GET /api/compliance/sbom`) and six migrations
  (`m20260726_000006_compliance` through `m20260727_000011_integrity_mac`,
  §10). This entry exists because none of the above had a §13 task
  record despite being fully landed, tested (DB-gated
  `tests/requests/cases.rs` erasure + record-integrity suites, plus
  DB-free unit tests in every `src/compliance/*.rs` module), and reachable
  — the same "shipped feature with zero spec/13 presence" gap DOC-2 found
  in sibling crates.
- [x] **2026-08-02 — Privacy: masked view + GDPR export (repo tasks.md
  P-3).** Case already honoured the `mask` **obligation** on
  `GET /{pid}` (`mask_case`, landed with the ABAC work — §9); this task
  was narrower than P-1/P-2 by design, adding only what was missing:
  `GET /{pid}/masked` (§6.13, always-redacted, no policy needed) and
  `GET /{pid}/export` (§6.14, the GDPR right-of-access envelope). The
  export reuses the existing `disclosure::action::EXPORT` — already
  declared in `disclosure.rs`'s action vocabulary, unused until now — for
  its §164.528 accounting, the same machinery `GET /{pid}` already used
  for reads. `export_case` lives beside `mask_case` in
  `controllers/cases.rs` rather than a new `src/privacy.rs` module,
  matching how masking was already organised in this crate (unlike
  organization/care-pathway, which each own a dedicated module).
  The end-to-end obligation proof needed its **own test binary**
  (`tests/export_masking.rs`), separate from the pre-existing
  `tests/masking.rs` (the SEC-G2/G3 concealment proof) — both set
  process-wide `policy()`/`require_auth()`/`compliance::audit_reads()`
  `OnceLock`s, and sharing one binary let the second test's boot silently
  win the race and starve the first of its own policy, so both files
  must stay separate to keep either one meaningful.
  *Verified:* 34 DB-gated request-suite + 2 dedicated masking/export
  binaries (1 each) + 2 enforcement + 1 outbox-audit green vs Postgres
  18; 193 lib tests; fmt + clippy clean.
- [x] **T-6 (Tantivy full-text/fuzzy/phonetic search — S-3).** Transfers
  the care-pathway/organization Tantivy pattern (repo tasks.md S-1/S-2):
  `src/search/index.rs` (`CaseIndexSchema`/`CaseIndex` — `pid` STORED;
  `title`/`alternate_titles`/`title_phonetic`/`identifiers`/`keywords`/
  `subjects`/`agency_name` TEXT; `case_number`/`agency_id`/`case_type`/
  `status`/`active` STRING exact-match) and `src/search/mod.rs`
  (`SearchEngine`: `search`/`fuzzy_search`/`phonetic_search`/
  `search_page`/`candidates`). Replaces the Postgres `ILIKE` title search
  (`GET /search?q=`, now `?fuzzy=`/`?phonetic=` too, with the true
  `X-Total-Count` from Tantivy's `Count` collector) and the capped
  1000-row `check-duplicates` scan (now blocked on up to 200
  fuzzy-title/phonetic/exact-identifier candidates from the index —
  subjects, the involved-party field, is the defining attribute made
  searchable). Indexing is wired into
  `streaming.rs`'s `*_and_emit` seam (best-effort, after the write
  commits) so no write path can skip it; `tasks/search.rs` adds the
  `search_reindex` CLI task plus a boot-time rebuild-if-empty. An
  unavailable index is a `503` on both endpoints, never silently "no
  results". Every returned `pid` still passes through the existing
  record-level ABAC `read_visibility` concealment (§10/§12) before it
  reaches a caller — the index is a candidate generator, not an
  authorization boundary. (Repo tasks.md S-3.)
- [x] **SEC-G8 (security): default-off exposure pin.** A named unit test
  pins that with `CASE_REQUIRE_AUTH` off (the shipped default) the sensitive
  reads — a case's PII, the audit trail, and the governed `subject_of` links
  (§10) — are open without a token, so activation is a **tracked release
  gate** (see `agents/share/security.md` §4) and the default can't be flipped
  to "secure" silently. (Repo tasks.md Phase 5 SEC-G8.)

- [x] **SEC-G2/G3 (security): record-level authz + masking on every read.**
  `list` / `search` / `check-duplicates` now omit cases a caller may not
  read (concealment, §10/§12), and FHIR `read` / `search` apply the same
  record-level ABAC + `mask` obligation as the native `GET /{pid}` (they
  previously took no caller). Shared `auth::read_visibility`; `mask_case`
  now `pub(crate)`; DB-gated `tests/masking.rs` proves the concealment on
  list / native GET / FHIR read. (Repo tasks.md Phase 5 SEC-G2/G3.)
- [x] **SEC-M1 (security): input-size caps close the O(n·m) matcher DoS.**
  `validation::problems` now `422`s oversized payloads *before* store/match:
  scalar text fields capped at `MAX_TEXT_LEN` = 1024 chars, arrays at
  `MAX_ARRAY_LEN` = 256 entries, per-array string entries at
  `MAX_ITEM_LEN` = 512 chars (report-everything, `src/validation.rs`).
- [x] **SEC-B5 (security): lock merge participants against a concurrent
  race.** The merge handler already `422`s `main == duplicate`; the
  `outbox` merge path (`streaming::merge_and_emit`) now also locks both
  participant rows `FOR UPDATE` (pid-ordered, deadlock-free) and re-checks
  the duplicate is still active before writing, so two concurrent merges of
  the same duplicate cannot both apply (the loser fails closed). (Repo
  tasks.md Phase 5 SEC-B5.)
- [x] **SEC-G1 (security): authorise + audit the governed bulk-links read.**
  `GET /api/cases/links` dumped every `subject_of` (case → person) edge
  with only the coarse blanket-read gate and no audit — a cross-case
  enumeration of the highest-governance §12 edge. It now authorises the
  bulk dump as a privileged governed read
  (`authorize_record(Action::Destructive, …)`; default policy admits only
  `svc`/`admin`) and writes a `links_bulk_read` audit row. DB-gated
  `bulk_links_requires_elevated_authority` (401/403/200). (Repo tasks.md
  Phase 5 SEC-G1.)
- [x] Title search — `GET /search?q=` Tantivy full-text/fuzzy/phonetic
  search (§13 T-6, 2026-08-02), replacing the earlier Postgres `ILIKE`.
- [x] Event streaming + audit log on CRUD — `audit_logs` table +
  best-effort row per create/update/delete (`models/audit_logs.rs`);
  in-memory event stream (`streaming.rs`); read at `/audit/recent`,
  `/{pid}/audit`, `/events/recent`. **Durable event bus Phase 1
  implemented** (in-memory canonical `Envelope` + `EventPublisher` seam
  per [`agents/share/event-bus.md`](../../../agents/share/event-bus.md)
  §4–§5): the publish path builds a versioned `Envelope` (`event_id`,
  `schema_version` 1, `entity` `"case"`, `kind`, `pid`, `seq`, `actor`,
  `name`) behind an `EventPublisher` trait with an `InMemoryPublisher`
  ring buffer; `/events/recent` returns the flat `EventView` projection
  (`{kind, pid, name, seq}`).
- [x] **Durable event bus — Phase 2 (transactional outbox).** Copy-adapted
  from the organization **reference**
  ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §3–§8).
  New `event_outbox` table (`migration/…_000004_event_outbox`: `PkAuto id`,
  `event_id UUID UNIQUE`, `entity`, `entity_pid`, `kind`, `occurred_at`,
  `actor`, `schema_version`, `payload JSONB`, `published_at`, partial index
  on unpublished rows); SeaORM entity `models/_entities/event_outbox.rs`;
  `models/event_outbox.rs` with the **pure** DB-free
  `OutboxInsert::from_envelope` mapping (unit-tested),
  `insert_on(&impl ConnectionTrait)`, `recent(db, limit) → Vec<EventView>`,
  and the relay poll/ack (`unpublished`/`mark_published`, unused until
  Phase 3). New `EventTransport`/`transport()` selector + `OutboxPublisher`
  in `src/streaming.rs`, plus transport-aware
  `create_and_emit`/`update_and_emit`/`delete_and_emit`/`merge_and_emit`
  used by **both** the native (`controllers/cases.rs`) and FHIR
  (`controllers/fhir.rs`) controllers. The model write helpers
  (`create`/`update_data`/`soft_delete`) are now generic over
  `sea_orm::ConnectionTrait`, so the `outbox` path runs the entity write
  **and** the `event_outbox` insert on one `db.begin()` transaction (crash
  can't persist one without the other); `memory` keeps the ring buffer, no
  tx. Gated by `CASE_EVENT_TRANSPORT` (default `memory` ⇒ behaviour and
  existing tests unchanged). Case data is personal data, but the outbox
  stores the same envelope the ring buffer already carries — no new
  exposure, just durability. Tests: DB-free envelope→row mapping
  (create/update/delete/merge fields, non-UUID pid rejected),
  transport-string parse; DB-gated (`tests/requests/event_outbox.rs`,
  `#[ignore]`) atomicity — one tx writes case + exactly one outbox row, a
  rollback drops both.
- [x] **Durable event bus — Phase 3 (relay + retention).** Copy-adapted
  from the organization **reference** (`src/relay.rs`). A background loop
  (`crate::relay::spawn`, started from `App::after_routes`) drains
  `event_outbox` (`unpublished` → `EventSink::send` → `mark_published`,
  at-least-once, per-pid order preserved) and periodically purges rows
  with `published_at < now() - CASE_EVENT_RETENTION_DAYS`. The default
  `LoggingSink` is the no-broker dev/CI sink; the `EventSink` trait is the
  seam. Gated by transport=`outbox` **and** `CASE_EVENT_RELAY` (both off
  by default ⇒ no loop, behaviour unchanged); interval via
  `CASE_EVENT_RELAY_INTERVAL_SECS`. Tests: logging/capturing sink send
  contract + config-default parsers (DB-free).
- [x] **Durable event bus — Phase 3, `FluvioSink` (BUS-1).**
  *(done 2026-08-03)* The real-broker `impl EventSink`, behind this
  crate's own `fluvio` Cargo feature (off by default — the dependency
  tree and boot behaviour of a default build are unchanged). One
  producer per topic (`fluvio::Fluvio::connect_with_config` +
  `topic_producer`, held for the sink's lifetime), partitioned by record
  `pid` per §7. Config: `CASE_FLUVIO_ENDPOINT` (the broker's SC address;
  unset ⇒ `LoggingSink`, unchanged default behaviour) and
  `CASE_EVENT_TOPIC` (default `mxi.case.events`). **No silent fallback**:
  an endpoint configured **without** the `fluvio` feature refuses to
  start the relay at all (logged at `error`), rather than a
  `LoggingSink` masquerade that would mark outbox rows `published_at`
  without ever reaching the broker the operator asked for — the same
  shape as the family's artifact-store "no fallback on an explicit
  backend choice" rule (`agents/share/bulk-import-export.md` §12). The
  initial connection retries indefinitely rather than falling back, for
  the same reason. `compose.fluvio.yaml` + `Dockerfile.fluvio-cli`
  provision a local SC+SPU broker (Fluvio's own documented Docker
  Compose layout, translated to this repo's Podman conventions) for
  opt-in manual runs; **not** wired into any automated CI stage. Tests:
  `cargo build`/`clippy --all-targets -D warnings`/`fmt --check` clean
  under both default features and `--features fluvio` (the real `fluvio`
  0.50 API compiling is the actual verification of correct usage — no
  web-search guess went unverified); `tests/fluvio_relay.rs` is a
  `#![cfg(feature = "fluvio")]`-gated, `#[ignore]`d round-trip
  (enqueue → `FluvioSink` → `drain_once` → assert `published_at`) with
  its run command documented inline — it needs a live broker, which no
  automated run in this repo stands up, so it is verified by compiling
  under the feature, not by an actual execution (same posture as
  person's `s3_round_trip_against_a_live_endpoint`, BLK-4). SOUP register
  updated. **BUS-2** (link-graph Fluvio consumer) and **BUS-3** (roll
  `FluvioSink` to the other nine services) have both since landed
  2026-08-03 — see the family capability matrix
  ([`agents/share/overview.md`](../../../agents/share/overview.md)); no
  deployment yet points `CASE_FLUVIO_ENDPOINT` at a live broker, which is
  the one thing genuinely still open here.
- [x] Prometheus metrics — `GET /metrics.prom` (root-mounted, public
  under enforcement) renders a process-wide registry
  (`src/metrics.rs`, `controllers/metrics.rs`) in text-exposition format:
  four CRUD counters (`case_created`/`updated`/`deleted`/`merged_total`)
  incremented in the cases controller, plus an `http_requests_total`
  label vec. Documented in OpenAPI under `observability`. Parity with the
  older Axum services.
- [ ] Privacy controls if any restricted fields appear.
- [x] Record merge — `POST /merge` folds a duplicate into a survivor
  (union fields, former-title alias, soft-delete, `merge_records`
  history + snapshot, `Merged` event); pure `src/merge.rs`;
  `/merges/recent`. Front-end merge action is a follow-up.
- [x] OpenAPI/Swagger — hand-written `src/openapi.rs` (matcher DTO is
  dependency-light, so no utoipa, matching the organization service)
  served at `/api-docs/openapi.json` + `/swagger-ui` by
  `controllers/docs.rs`.
- [x] Payload validation — `src/validation.rs` checks `title`,
  `opened_date` (ISO-8601 `YYYY` / `YYYY-MM-DD` with calendar-range
  checks), non-blank identifier values, and non-blank `subjects` /
  `keywords`; `422` with all problems reported together.
- [x] Request-level integration tests (Postgres) — landed
  `#[ignore]`-gated; wiring a DB-backed run into CI remains.
- [x] **CI: actually run the gated request tests.** *(resolved
  2026-08-01, repo-wide.)* This crate is now enrolled in
  [`ci/db-suites.txt`](../../../ci/db-suites.txt); the `test-db` CI stage
  runs `scripts/ci-check.sh test-db case/case-service-with-loco` — which
  is `cargo test -- --ignored` against the CI Postgres service — so the
  `#[ignore]`-gated request/erasure/record-integrity/outbox suites
  execute on every push, not just locally.
- [x] Token verification consuming the auth-service's published key —
  `src/auth.rs` embeds `authentication-verifier`; offline verification via
  a process-wide `Verifier` (env-configured `CASE_PASETO_KEYS` /
  `CASE_TOKEN_ISSUER` / `CASE_TOKEN_AUDIENCE`); `AuthUser`/`MaybeAuthUser`
  extractors; `/whoami` protected; audit `actor` stamped from the token.
  - [x] Switch the credential to PASETO v4 public (Ed25519) per
    [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
    (source of truth; supersedes the RS256-JWT + JWKS model): verifier
    consumes the auth-service's published Ed25519 key(s)
    (`Verifier::from_paseto_keys_value` / `from_paseto_keys_url`), same
    `Claims` shape (`kid`/`iss`/`aud`/`exp`; footer carries `kid`).
  - [x] Blanket `/api/*` enforcement — `CASE_REQUIRE_AUTH` flag +
    `auth::enforce` middleware wired in `App::after_routes` (off by
    default; public paths exempt; un-gated `enforce`/`parse_bool` unit
    tests + DB-gated request test). Family contract
    [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md).
    Case data is personal data, so this is the access-control gate.
  - [x] paseto-keys-over-HTTP fetch at boot — done 2026-07-04.
    `CASE_PASETO_KEYS_URL` set (non-blank) ⇒ `auth::init` (called from
    `App::after_routes`, before serving) fetches the published key set
    once via `Verifier::from_paseto_keys_url` (verifier `fetch` feature);
    success ⇒ fetched key set wins over `CASE_PASETO_KEYS`
    (`tracing::info!`), failure ⇒ `tracing::warn!` + fall back to the env
    path (the service always boots); unset/blank ⇒ prior behaviour
    unchanged. No refresh loop (rotation-triggered refetch → §16). Tests:
    a `#[tokio::test]` local ephemeral-port HTTP listener proves the
    fetch-built verifier accepts a token signed by the served key, and a
    fast-failing URL (`http://127.0.0.1:1/`) proves fallback without
    panic. Activating the enforcement flag remains an operations
    decision.
- [x] **Cross-service entity links (write side) — LANDED (case is the
  reference; `cross-service-linking.md` rollout step 2 deviation, §11).**
  See §5, §8.6, §9, §10 (§12.1) and
  [cross-service linking](../../../agents/share/cross-service-linking.md).
  Case owns the `subject_of` (case → person) edge — the
  highest-governance v1 kind. (The design nominally names person + worker
  `same_identity` for step 2, but those are older axum-style services with
  no event bus; case is the first loco service that both *originates* a v1
  edge AND has the durable-bus outbox to emit `linked`/`unlinked` — so it
  ships the write side first. person/worker `same_identity` awaits their
  own event infrastructure.)
  - [x] Migration `m20220101_000005_entity_links` creating the
    `entity_links` table (§4.1 schema: `id UUID` pk, `from_pid`, `kind`,
    `to_ref`, `role`, `confidence`, `provenance`, `valid_from`,
    `valid_to`, `deleted_at`) with the
    `UNIQUE (from_pid, kind, to_ref, valid_from)` upsert key — declared
    `NULLS NOT DISTINCT` so a null `valid_from` still collides on
    re-assert (Postgres 15+), making the upsert idempotent. (`…_000004`
    was already the `event_outbox` migration.)
  - [x] `EntityRef` / `EdgeKind` contract — **depended on** the shared
    `entity-ref` crate (`entity_ref::{EntityRef, EntityType, EdgeKind}`)
    rather than copied per project. The `permits(from, to)` /
    `is_symmetric` / `is_temporal` registry is reused as the validator.
  - [x] `SeaORM` entity `models/_entities/entity_links.rs` + model
    `models/entity_links.rs` (`NewEdge`, idempotent `upsert` on the
    unique key with revive-on-reassert, `list_active`, case-scoped
    `find_active`, `soft_delete`), generic over `ConnectionTrait` so the
    outbox path shares the handler transaction.
  - [x] Link endpoints `POST` / `GET` / `DELETE`
    `/api/cases/{pid}/links` (`controllers/links.rs`); create/upsert is
    optimistic (no cross-service call) and admits exactly `subject_of`
    (case → person) — every other kind/endpoint pair is `422`
    (DB-free-tested).
  - [x] Emit `linked` / `unlinked` events on the existing event envelope
    via the transactional `link_and_emit` / `unlink_and_emit` seam (edge
    detail in the new additive `Envelope::data`; no new transport, no
    `SCHEMA_VERSION` bump; `EventView` projection byte-identical for
    existing kinds).
  - [x] **Governance (§12.1, [cross-service linking §10](../../../agents/share/cross-service-linking.md)):**
    both create AND read authorise at the "read the case" level
    (`auth::authorize_record` on the loaded case, so never more
    permissive than `GET /api/cases/{pid}`); every create/withdraw
    writes an audit row (`linked` / `unlinked` action, `actor` from
    token). Per-record **masking** of the edge and a "denied read is
    indistinguishable from no-such-edge" refinement remain follow-ups —
    v1 leans on the blanket write/destructive guard + the case-level
    record check.
  - [x] Partition guard: `entity_links` live only in their own table +
    events; they are never projected into the matcher input (§5).
  - **Acceptance:** DB-gated (`tests/requests/entity_links.rs`,
    `#[ignore]`) create → list → delete round-trip asserts a `linked`
    then `unlinked` event on `/events/recent`, plus idempotent re-assert
    and the `same_identity` → `422` reject; DB-free unit tests
    (`controllers/links.rs`, `streaming.rs`) pin the accept/reject
    validation matrix and the `data`-carrying envelope with the frozen
    projection.
- [x] **Bulk import / export (BLK-5, landed 2026-08-03).** See §8.7 and
  the family contract
  [bulk import / export](../../../agents/share/bulk-import-export.md).
  Async, job-based on the loco `worker` feature. JSONL + CSV only (no
  Parquet, no S3 — §8.7 "Formats").
  - [x] Migrations `m20260803_000012_review_queue` (new to this crate;
    `provenance` from the start) and `m20260803_000013_bulk_jobs`
    (`src/models/bulk_jobs.rs`, `src/models/review_queue.rs`).
  - [x] Five endpoints (`src/bulk/handlers.rs`):
    `POST`/`GET /api/cases/import[/{id}]`,
    `POST`/`GET /api/cases/export[/{id}]`, `GET /api/cases/bulk-jobs`.
    `POST /import` is a declared destructive named POST
    (`auth::DESTRUCTIVE_POST_SUFFIXES` already listed `/import` ahead of
    this rollout); the privileged export paths are gated explicitly in
    the handler (§8.7).
  - [x] `BulkJobWorker` (`src/bulk/worker.rs`, registered in
    `app.rs::connect_workers`) draining `queued → running →
    completed|completed_with_errors|failed`.
  - [x] JSONL (`src/bulk/jsonl.rs`, lossless reference) + CSV
    (`src/bulk/csv.rs` + `columns.rs`, flattening per §8.7).
  - [x] Per-row import pipeline (`src/bulk/pipeline.rs`) reusing the
    single-create validators (`src/validation.rs`) + case-matcher + the
    (new) review queue: stable-key (§8.7) upsert in place, else
    duplicate detection → review queue with `provenance = import`;
    events + audit not bypassed (`streaming::create_and_emit`/
    `update_and_emit`); `dry_run` supported. Sequential idempotency only
    — see §8.7 "Concurrency" for the documented SEC-B3 follow-up.
  - [x] Per-row error report (`src/bulk/error_report.rs`, CSV) with
    `row_number, field, code, message`; one bad row never aborts the
    load; counts reconcile (`rows_total = created + upserted + errored`,
    with `rows_to_review <= rows_created`).
  - [x] **Governance ([bulk import / export §8](../../../agents/share/bulk-import-export.md),
    §8.7, §12 / §12.1):** export masked by default (reusing
    `controllers::cases::mask_case`); full / unmasked or
    `include_soft_deleted` export requires elevated (Destructive)
    authorisation checked at submission; **every** export writes an
    `audit_logs` row (`actor`, filter, format, row count, masking
    profile — even for zero rows) and the write **gates delivery**
    (SEC-B8). Known gap, documented in §8.7: no per-row record-level ABAC
    inside the async export worker.
  - **Acceptance (all DB-gated, `src/bulk/pipeline.rs::db_tests` +
    `src/models/bulk_jobs.rs::db_tests`):**
    `import_is_idempotent_on_the_agency_case_number_key` (re-submitting
    the same file upserts to the same state, no duplicates);
    `invalid_row_is_reported_not_fatal` (a per-row error-report test —
    one bad row skipped + reported, good rows commit);
    `keyless_row_with_a_likely_duplicate_creates_and_queues_for_review`
    (a keyless likely-duplicate row is created *and* lands in the review
    queue with `provenance = import`); `csv_import_creates_a_keyed_row` +
    `export_round_trips_through_{jsonl,csv}` (both formats round-trip);
    `export_masks_by_default_and_full_is_unmasked` (masked-vs-full
    export); `export_rejects_include_soft_deleted`; and
    `idempotent_resubmit_returns_the_same_job` /
    `keyless_submit_is_never_deduped` (SEC-B9 idempotency-key dedupe).
- [x] **FHIR R5 API** (`Task` default; `CarePlan` roadmap) — adopt the
  family contract ([`agents/share/fhir.md`](../../../agents/share/fhir.md)).
  **Done:** `src/fhir/{mod,resources,search}.rs` + mounted
  `src/controllers/fhir.rs` (`routes()` in `app.rs`) implement read /
  create / update / delete / search at `/fhir/Task{,/{id}}` +
  `GET /fhir/metadata`, copy-adapted from the organization reference.
  `case_matcher::Case` → `Task`: `title`→`description`, `status`→`status`,
  `priority`→`priority`, `case_type`→`code`, agency-scoped `case_number`
  (with `agency_id`/`agency_name` in `assigner`)→`identifier`,
  `identifiers`→`identifier` (scheme↔system round-trip), first `subjects`
  entry→`for`. Documented lossy gaps: `alternate_titles`, `opened_date`,
  `keywords`, `same_as`, `in_language`, and 2nd+ subjects are not carried;
  `Closed`/`Resolved` status collide on `completed`; `Low` priority
  resolves to `Normal`; `Custom` status label is dropped. Writes reuse the
  native model helpers + audit + event/metrics; `/fhir/*` sits behind the
  blanket auth+ABAC guard. DB-free unit tests: scheme round-trip, DTO↔Task
  round-trip, missing-`description` rejected, search predicates,
  `CapabilityStatement` param drift-guard. **Subject-reference masking is
  DEFERRED** (see below) — the crate has no field-masking layer today.
  **Best-effort mapping** (§3, `low` fidelity — a governmental case has
  no exact FHIR analog): map the stored `case_matcher::Case` DTO (§5) to
  a FHIR **`Task`**: `title` → `description`, agency-scoped `case_number`
  (with `agency_id`/`agency_name`) → `identifier`, `status` → `status`,
  `priority` → `priority`, `case_type` → `code`, subject person (the
  `subject_of` cross-service link, §8.6) → `for`/`focus`. New `src/fhir/`
  module (resource structs, `to_fhir_task`/`from_fhir_task`,
  `FhirOperationOutcome`, searchset `Bundle`, search-param parsing) + a
  mounted `src/controllers/fhir.rs` (`routes()` in `app.rs`): read /
  create / update / delete / search at `/fhir/Task{,/{id}}` +
  `GET /fhir/metadata` `CapabilityStatement`. Reuses native model helpers
  (`models/cases.rs`), validators (`src/validation.rs`), event/audit
  (`src/streaming.rs`, `models/audit_logs.rs`), and the blanket
  auth+ABAC guard (§12.1; `/fhir/*` guarded, not on the public
  allow-list, action derived from HTTP method).
  - [ ] **Elevated governance ([fhir §8](../../../agents/share/fhir.md),
    [cross-service linking §10](../../../agents/share/cross-service-linking.md)):**
    the `for`/subject person reference inherits the `case ↔ person`
    (`subject_of`) sensitivity — access control + audit on read AND
    write, and the subject reference is masked for unauthorised callers
    (who must not even learn the edge exists).
  - [ ] Supported search params (§6, reflected in the
    `CapabilityStatement`): `_id`, `_lastUpdated`, `_count`,
    `identifier`, `status`, `priority`.
  - **Acceptance:** DTO↔`Task` round-trip; each interaction (read /
    create / update / delete / search); search → searchset `Bundle`;
    `OperationOutcome` on 404 / 400 / 422; `CapabilityStatement` matches
    the mounted routes; and a subject-reference masking test (an
    unauthorised caller neither sees the subject reference nor learns the
    edge exists).

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

## 14. Implementation status

Done: loco boot; cases table + migration; CRUD with `422` validation on
create/update (blank `title`, `opened_date` format, non-blank
identifier / subject / keyword, all problems reported together);
Tantivy full-text/fuzzy/phonetic search; `/match`, `/check-duplicates`, and `/merge`
(record merge + history) embedding case-matcher; field masking
(`mask_case`) + the masked view + audited GDPR export, wired to the ABAC
`mask` obligation; audit log + in-memory
event streaming on every CRUD/merge (`/audit/recent`, `/{pid}/audit`,
`/events/recent`, `/merges/recent`); offline PASETO v4 public token
verification (`AuthUser`/`MaybeAuthUser`, `/whoami`, audit `actor` from
the token) with boot-time key-set fetch over HTTP
(`CASE_PASETO_KEYS_URL`; fetched key set wins, env fallback, always
boots);
OpenAPI 3 doc + Swagger UI (`/api-docs/openapi.json`, `/swagger-ui`);
Prometheus metrics (`/metrics.prom`, root-mounted + public, CRUD counters
+ HTTP request label vec); bulk import/export (BLK-5, JSONL + CSV,
`src/bulk/`, §8.7); cross-service `subject_of` entity links (write side,
§8.6, the family reference implementation); durable event bus Phases
1–3 including the `FluvioSink` real-broker sink behind the `fluvio`
feature (BUS-1); FHIR R5 `Task` CRUD + search (§9, §13); the compliance
suite — tamper-evident audit chain, read/disclosure auditing, row-level
record integrity, GDPR Art. 17 erasure, external-witness checkpoints,
keyed integrity MAC, SBOM/build-provenance (§12.0/§12.0.1); DB-free tests
+ gated request-level tests + gated bulk-pipeline tests; green build +
clippy; enrolled in CI's `test-db` stage (`ci/db-suites.txt`).

## 15. Roadmap

v0.1 (here): CRUD + title search + matching + merge + audit + streaming
+ OpenAPI + offline PASETO v4 public token verification per
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
(source of truth; supersedes the RS256-JWT model) + boot-time
paseto-keys-over-HTTP fetch (`CASE_PASETO_KEYS_URL`, fetched key set wins,
env fallback). v0.2: Tantivy full-text/fuzzy/phonetic search + durable
event bus Phases 1–2 — **done**. v0.3: privacy controls (masking + GDPR
export — done 2026-08-02) + blanket
`/api/*` enforcement — **done**; durable bus Phase 3 (relay loop + the
`FluvioSink` real-broker sink, BUS-1) — **done 2026-08-03**. Also
landed, not originally scoped into a version line: cross-service
`subject_of` links (§8.6, 2026-07-10, the family reference), the
compliance suite — audit chain, erasure, row-level integrity,
checkpoints, keyed MAC, SBOM (§12.0/§12.0.1, 2026-07-25..27), FHIR R5
`Task` CRUD/search (§9, §13), and bulk import/export (§8.7, BLK-5,
2026-08-03).

## 16. Open questions

- Normalise subjects / identifiers into their own tables once search
  lands?
- Real-time duplicate check on create (409) vs the explicit endpoint?
- Key-set refresh: the boot-time paseto-keys fetch is once-only — add a
  rotation-triggered refetch (e.g. on `UnknownKid`) or a periodic
  refresh loop?
- **Bulk import (§8.7): true concurrent-importer race safety
  (SEC-B3).** Today's `src/bulk/pipeline.rs` gives sequential
  idempotency only (re-running a file is safe; two importers of the same
  stable key racing each other is not). Closing this needs either a
  `ConnectionTrait`-generic `streaming::create_and_emit`/
  `update_and_emit` variant (so a caller-held Postgres advisory lock can
  span find-then-write) or a dedicated transaction-scoped bulk write
  path — a decision that likely belongs at the family level
  (`agents/share/bulk-import-export.md`) rather than being invented
  per-crate.
- **Bulk export (§8.7): per-row record-level ABAC inside the async
  worker.** The worker has no live bearer token to evaluate a
  record-level (`resource.case_type`/`status`/`priority`) decision
  against. A fix needs either a short-lived, job-scoped credential the
  worker can present back to the ABAC engine, or restricting
  record-level-sensitive exports to a synchronous-only path — also
  likely a family-level design question, since the person reference
  implementation shares the same gap.
- **Bulk artifact storage: S3 backend.** `src/bulk/store.rs`'s
  `ArtifactStore` trait is async specifically to make this additive
  later (care-pathway's `S3ArtifactStore` is the template); not built in
  this rollout (out of BLK-5's scope).

## 17. References

- The case-matcher spec; loco.rs; schema.org case-management vocabulary.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
