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
validation, OpenAPI, audit, in-memory streaming, record merge, and
offline PASETO v4 public token verification (Ed25519, via the
auth-service's published key). Deferred (§13): privacy, gRPC.
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
3. `GET /api/cases/{pid}` — return the stored `Case`.
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

The uniform async, job-based bulk contract is fixed family-wide in
[bulk import / export](../../../agents/share/bulk-import-export.md)
(execution model on `bg_pg`, the `bulk_jobs` table, the five endpoints,
the JSONL/CSV/Parquet formats, import dedupe semantics, the per-row
error report, and the export privacy/audit posture). This section
declares only what the Case Service differs on (per
[bulk import / export §10](../../../agents/share/bulk-import-export.md)).

**Stable key(s) for upsert (import idempotency).** A row upserts in
place when it carries a key that uniquely identifies an existing case;
otherwise it runs the normal duplicate detection (§6.7) and routes
likely duplicates to the review queue with `provenance = import`. Keys,
in priority order:

1. **`pid`** — the case's public UUID, when present (exact upsert).
2. **A deterministic globally-unique identifier** — any `identifiers`
   entry on a deterministic scheme (`Docket`, `ExternalCaseId`, `Uri`,
   `Uuid`) — the same schemes the case-matcher short-circuits to `1.0`
   on (case-matcher §15). A shared value here is an unambiguous upsert
   target.
3. **Agency-scoped case number** — `case_number` **scoped by
   `agency_id`** (the `AgencyCaseNumber` pairing; case-matcher §11/§16).
   A case number is unique only within its agency, so the upsert key is
   the `(agency_id, case_number)` pair, never `case_number` alone.

`same_as` URL overlap is a matcher short-circuit but is **not** used as
a bulk upsert key (it is a sameness signal, not a stable record
identity); keyless rows fall through to duplicate detection.

**CSV column set + flattening** (per
[bulk import / export §5](../../../agents/share/bulk-import-export.md);
JSONL is the lossless reference — prefer it when fidelity matters):

- **Scalar columns** (one each): `pid`, `title`, `case_number`,
  `agency_id`, `agency_name`, `case_type`, `status`, `priority`,
  `opened_date`, `in_language`.
- **Arrays / arrays-of-objects → a single JSON-encoded cell each**:
  `alternate_titles`, `subjects`, `keywords`, `identifiers`
  (`[{scheme,value}, …]`), `same_as`. Enum-as-`{"Custom":"label"}`
  cells round-trip as their JSON form.
- **Cross-service `entity_links`** (the `subject_of` / `about` edges,
  §8.6) are **not** part of the `Case` payload export; per
  [bulk import / export §9](../../../agents/share/bulk-import-export.md)
  they are an **optional separate** link-import/export job and, being
  the highest-governance kind (§12.1), are never bundled into the
  default case export.

**Export sensitivity — PROMINENT (cases are personal/sensitive
government data; ties to [§12](#12-compliance)).** A bulk extract of
case data is itself a compliance event (HIPAA / NHS / GDPR), so export
is governed more strictly than for non-personal entities:

- **Masked by default.** The `masking_profile` defaults to **masked**
  output; full / unmasked export requires **elevated authorisation** —
  at least the authorisation needed to **read a case**
  (`GET /api/cases/{pid}`), the same boundary as §9 / §12.1. A bulk
  export MUST NEVER reveal more than the caller could read one case at a
  time.
- **`include_soft_deleted` defaults `false` and is gated** — soft-deleted
  cases are exported only with explicit elevated authorisation.
- **Every export is audited** — an `audit_logs` row per export job
  capturing `actor` (from the verified token), the list/search
  **filter**, format, **row count**, and **masking profile**, written
  even for a zero-row export. This mirrors the §12.1 obligation to audit
  every read of sensitive data.
- **Filtered, not all-or-nothing** — exports reuse the existing
  list/search query (title `ILIKE`, agency, status, …), so they are
  scoped. The single-subject special case (filter = one `pid`) is the
  per-case GDPR export.

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

**Not yet adopted** (steps 4–5): the GDPR residency / lawful-basis /
Art. 9 declarations, GDPR Art. 17 erasure by redaction, row-level record
integrity hashing, the FHIR conformance machinery, and the SOUP/SBOM
evidence bundle. The context recorded on an audit row therefore carries
the caller's declaration only — it does **not** claim a residency or
lawful basis this deployment has not declared. §13 T-10 (masking +
Art. 15 export) remains the higher-priority gap for this service.

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
- [x] Title search — `GET /search?q=` Postgres `ILIKE` on the
  denormalised `title` (cap 50, wildcards escaped). Tantivy full-text /
  fuzzy search over the JSONB payload remains deferred.
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
  contract + config-default parsers (DB-free). The remaining follow-up is
  a real **`FluvioSink`** behind a `fluvio` cargo feature +
  `CASE_FLUVIO_ENDPOINT`/`CASE_EVENT_TOPIC` (broker-gated) — another
  `impl EventSink`, no change to the drain/retention loop.
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
- [ ] **CI: actually run the gated request tests.** The CI test job sets
  `DATABASE_URL` but does not pass `--ignored`, so the `#[ignore]`-gated
  request tests (CRUD/audit/event trail, merge audit actions, blanket
  enforcement, OpenAPI/Swagger) never execute in CI — only the DB-free
  unit + `tests/matching.rs` suites do. Add an `--ignored` step (against
  the CI Postgres service) so the request contracts are exercised.
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
- [ ] **Bulk import / export.** See §8.7 and the family contract
  [bulk import / export](../../../agents/share/bulk-import-export.md)
  (uniform across entities; only the §8.7 stable keys, CSV columns, and
  export sensitivity differ for case). Async, job-based on `bg_pg`.
  - [ ] Migration `m20220101_000005_bulk_jobs` creating the `bulk_jobs`
    table ([bulk import / export §3](../../../agents/share/bulk-import-export.md)
    schema, with the `UNIQUE (entity, kind, idempotency_key)` retry key).
  - [ ] Five endpoints
    ([bulk import / export §4](../../../agents/share/bulk-import-export.md)):
    `POST`/`GET /api/cases/import/{id}`,
    `POST`/`GET /api/cases/export/{id}`, `GET /api/cases/bulk-jobs`.
  - [ ] `bg_pg` worker draining `queued → running →
    completed|completed_with_errors|failed`, with progress + count
    updates on `bulk_jobs`.
  - [ ] JSONL (lossless reference) + CSV (flattening per §8.7) + Parquet
    (export-first, feature-gated) codecs.
  - [ ] Per-row import pipeline reusing the single-create validators
    (`src/validation.rs`) + case-matcher + the review queue: stable-key
    (§8.7) upsert in place, else duplicate detection → review queue with
    `provenance = import`; events + audit not bypassed; `dry_run`
    supported.
  - [ ] Per-row error report (CSV or JSONL) with
    `row_number, source_line, field, code, message`; one bad row never
    aborts the load; counts reconcile (`rows_total = created + upserted
    + to_review + errored`).
  - [ ] **Governance ([bulk import / export §8](../../../agents/share/bulk-import-export.md),
    §8.7, §12 / §12.1):** export masked by default; full / unmasked or
    `include_soft_deleted` export requires elevated (case-read)
    authorisation and is never more revealing than reading cases one at
    a time; **every** export writes an `audit_logs` row (`actor`,
    filter, format, row count, masking profile — even for zero rows).
  - **Acceptance:** an idempotent-re-import test (re-submitting the same
    file upserts to the same state, no duplicates); a per-row
    error-report test (one bad row skipped + reported, good rows
    commit); a dedupe-to-review test (keyless likely-duplicate row lands
    in the review queue with `provenance = import`); a **masked-vs-full
    export authz test** (default export is masked; full / unmasked
    export without elevated authz is rejected, and never exposes more
    than single-record reads); and an **export-audit test** (every
    export — including a zero-row export — writes an `audit_logs` row
    with actor, filter, format, row count, and masking profile).
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
(record merge + history) embedding case-matcher; audit log + in-memory
event streaming on every CRUD/merge (`/audit/recent`, `/{pid}/audit`,
`/events/recent`, `/merges/recent`); offline PASETO v4 public token
verification (`AuthUser`/`MaybeAuthUser`, `/whoami`, audit `actor` from
the token) with boot-time key-set fetch over HTTP
(`CASE_PASETO_KEYS_URL`; fetched key set wins, env fallback, always
boots);
OpenAPI 3 doc + Swagger UI (`/api-docs/openapi.json`, `/swagger-ui`);
Prometheus metrics (`/metrics.prom`, root-mounted + public, CRUD counters
+ HTTP request label vec); DB-free tests + gated request-level tests;
green build + clippy.

## 15. Roadmap

v0.1 (here): CRUD + title search + matching + merge + audit + streaming
+ OpenAPI + offline PASETO v4 public token verification per
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
(source of truth; supersedes the RS256-JWT model) + boot-time
paseto-keys-over-HTTP fetch (`CASE_PASETO_KEYS_URL`, fetched key set wins,
env fallback). v0.2: Tantivy full-text/fuzzy/phonetic search + durable
event bus Phases 1–2 — **done**. v0.3: privacy controls, blanket
`/api/*` enforcement, durable bus Phase 3 (Fluvio relay).

## 16. Open questions

- Normalise subjects / identifiers into their own tables once search
  lands?
- Real-time duplicate check on create (409) vs the explicit endpoint?
- Key-set refresh: the boot-time paseto-keys fetch is once-only — add a
  rotation-triggered refetch (e.g. on `UnknownKid`) or a periodic
  refresh loop?

## 17. References

- The case-matcher spec; loco.rs; schema.org case-management vocabulary.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
