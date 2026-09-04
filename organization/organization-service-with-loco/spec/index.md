# Organization Service — Specification

> **Single source of truth (crate internals).** Code conforms to this
> spec. Behavioural change = spec + code + test in one PR. Live work
> queue is §13.
>
> **Entity umbrella spec:** the fuller cross-subproject contract lives
> at [`organization/spec/index.md`](../../spec/index.md) (numbered files
> `01-…`–`18-…`). It carries the entity-wide **requirement / task IDs**
> (`R-DUP`, `T-2`, `T-7`, `T-9`, `T-12`, `OQ-1`) that the source-code
> comments cite (e.g. `src/controllers/organizations.rs` → "spec §6
> R-DUP, task T-7"; `src/auth.rs` → "spec §13 T-9"; `src/app.rs` →
> "entity spec §13 T-12"). Those citations resolve against the umbrella
> spec's §6 / §13. When the umbrella spec and this crate spec disagree
> about crate internals, this crate spec wins; about the integration
> contract, the umbrella spec wins (see its header).
>
> Sibling matcher: [organization-matcher](../../organization-matcher-rust-crate/spec/index.md).
> Sibling front-end: [organization-front-end-with-svelte](../../organization-front-end-with-svelte/spec/index.md).

## 1. Purpose and vision

A registry of organization identities (schema.org/Organization) for the
Main X Index family: create/read/update/delete records and detect
duplicates with the canonical organization-matcher. Built on loco.rs.

## 2. Scope

MVP: CRUD + matching. Since delivered beyond the MVP (§13): full-text
search (Tantivy), streaming (incl. the durable outbox + relay + a
`FluvioSink`), audit, OpenAPI, record merge + a stored review queue,
field masking + the GDPR right-of-access export, async bulk
import/export (BLK-5, JSONL/CSV/TSV), a **FHIR R5 API** (the family
reference implementation), header-based API versioning, and PASETO key
rotation + ABAC policy hot-reload without a restart (AU-2). Still out
of scope (deferred, §13): gRPC, richer validation (URL/country-code
format checks — identifier check-digits landed via SEC-M5), real-time
duplicate check on create. Consent is out of scope **by decision**, not
by deferral (§13): an organization is not a data subject.
Authentication is out of scope here — provided by the central
authentication-service.

## 3. Stakeholders and users

Operators curating an organization registry; peer services resolving
organization identity; the organization front-end.

## 4. Glossary

- **pid** — public UUID of an organization record.
- **data** — the full `Organization` payload stored as JSONB.
- **deterministic identifier** — LEI/DUNS/etc. that pins a match to 1.0
  (see the matcher spec).

## 5. Domain model

The API DTO is `organization_matcher::Organization`: `name`,
`legal_name`, `alternate_names`, `identifiers`, `url`, `same_as`,
`address`, `jurisdiction`, `founding_date`, `telephone`, `email`,
`keywords`. The service does not fork this type.

**Partition rule.** The within-entity `relationships[]` (org → org,
inside this registry) are a matcher signal. **Cross-service links**
(e.g. `person works_at organization`) are separate: they live only in
the aggregator + the originating service, never in `relationships`, and
are never fed to the matcher. See §8 and
[`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md) §7.

## 6. Functional requirements

1. `POST /api/organizations` — create; `name` required (422 if blank).
2. `GET /api/organizations[?limit=&offset=]` — list active `{pid, name}`,
   newest first, one page at a time. Reports `X-Total-Count` /
   `X-Limit` / `X-Offset` per
   [`agents/share/restful.md`](../../../agents/share/restful.md);
   omitting both parameters returns the first 100, exactly as before
   pagination. `limit` clamps to 500; `offset` beyond 10 000 is `400`
   (an unbounded offset makes the database materialise and discard
   arbitrarily many rows). Search (§6.11) paginates identically, and its
   `X-Total-Count` is the **index's** match count, not the page length.
3. `GET /api/organizations/{pid}` — return the stored `Organization`.
   Runs the **record-level** ABAC pass after loading (shared
   `authorization-attributes.md` §9): resource attributes are
   `resource.jurisdiction` and `resource.has_fiscal_id`, and an allow
   carrying the **`mask` obligation** returns the masked view (§6.12)
   from this same URL. A no-op while `ORGANIZATION_REQUIRE_AUTH` is off.
4. `PUT /api/organizations/{pid}` — replace the payload; `name`
   required (422 if blank).
5. `DELETE /api/organizations/{pid}` — soft-delete (`active=false`,
   `deleted_at` stamped).
6. `POST /api/organizations/match` — rank an explicit `{query,
   candidates}` set (no persistence).
7. `POST /api/organizations/check-duplicates` — match a query against
   stored organizations; return the ones above threshold, ranked.
   Candidates are **blocked via the search index** (fuzzy name + exact
   identifier + phonetic routes, capped at 200 candidates), not scanned
   from the table: a duplicate's reachability depends on its similarity,
   not on how recently it was inserted. `503` when the index is
   unavailable — answering "no duplicates" from a broken index would let
   a caller create a duplicate believing it had been checked.
8. `POST /api/organizations/deduplicate` — batch-scan the stored
   records pairwise (up to the §6 R-DUP scan cap) and **persist** likely
   duplicates in the stored `review_queue` (normalized-pair upsert:
   re-scans refresh scores, decided rows keep their decision, item ids
   stay stable); the response reports the stored rows
   (`organizations_scanned` / `duplicates_found` / `auto_merged` (always
   0) / `queued_for_review` / `review_items[]`). Destructive-classed
   under ABAC, like merge.
9. `GET /api/organizations/review-queue[?status=&limit=]` — list the
   stored review queue, newest first (limit cap 500; unknown status
   token → `422`).
10. `POST /api/organizations/review-queue/{id}/decision` — decide a
    `pending` item (`{"status": "confirmed" | "rejected"}`);
    first-writer-wins in SQL (`404` unknown id, `422` already decided).
    The reviewer identity is the verified bearer `sub` when present, and
    each decision writes a `review_decision` audit row.
11. `GET /api/organizations/search?q=…[&fuzzy=true][&phonetic=true]` —
    **Tantivy full-text search**, ranked by relevance, capped at 50.
    Indexed fields: `name`, `legal_name`, `alternate_names`, identifier
    values, `keywords`, the flattened postal address, and `url`;
    `jurisdiction` is indexed for exact filtering (unused today).
    `fuzzy` gives typo tolerance (Levenshtein ≤ 2 per token) and
    `phonetic` matches Soundex codes; `phonetic` takes precedence when
    both are set. Blank/missing `q` → `400`; index unavailable → `503`.
    A query Tantivy's parser rejects falls back to an OR over its
    tokens rather than erroring.
12. `GET /api/organizations/{pid}/masked` — the **masked view**:
    `telephone` and `email` masked to their tail, the address's
    `street_address` dropped, and `TaxId` / `Vat` identifier values
    masked. Public registry identifiers (LEI, DUNS, ROR, ISNI,
    Wikidata, …), the names, `url`, and `jurisdiction` are untouched —
    masking them would break the lookups a registry exists for.
13. `GET /api/organizations/{pid}/export` — **GDPR right-of-access**
    export: an envelope of `{entity, pid, exported_at, masked, record,
    note}`. **Audited** on every call (`exported`, with whether it was
    masked), because a disclosure of personal data is itself a
    recordable event. A caller whose record-level policy carries the
    `mask` obligation gets the redacted record and `masked: true` — an
    access request answered with redactions must not look complete.
14. `GET /metrics.prom` — Prometheus metrics in text-exposition format
   (`text/plain; version=0.0.4`). Mounted at the application **root**
   (not under `/api`), public even under blanket auth enforcement.

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic matching via the
embedded library; soft-delete with audit-friendly timestamps.

### Configuration environment variables

| Variable | Default | Purpose |
|---|---|---|
| `ORGANIZATION_PASETO_KEYS` | empty key set | Published Ed25519 public-key set (`paseto-keys` JSON) for offline PASETO v4.public token verification (`src/auth.rs`). |
| `ORGANIZATION_PASETO_KEYS_URL` | unset ⇒ no fetch | When set, fetch the key set over HTTP **at boot** (`Verifier::from_paseto_keys_url`, typically the auth-service `/.well-known/paseto-keys`; seeded from `App::after_routes` via `auth::init_from_env`). Success ⇒ the fetched set wins over `ORGANIZATION_PASETO_KEYS`; failure ⇒ warn + fall back to the env key set — the service always boots. Refreshed periodically thereafter — see `ORGANIZATION_PASETO_KEYS_REFRESH_SECS` below (AU-2, §13). |
| `ORGANIZATION_PASETO_KEYS_REFRESH_SECS` | `3600` | AU-2: how often `auth::spawn_key_refresh` re-fetches `ORGANIZATION_PASETO_KEYS_URL` in the background, so a rotated key reaches a running process without a restart. `0` disables the refresh loop; a no-op when `ORGANIZATION_PASETO_KEYS_URL` is unset. A failed refresh **keeps the current key set** (a transient auth-service outage must not lock every caller out). The verifier and the ABAC policy are both hot-reloadable holders (`ReloadableVerifier` / `ReloadablePolicy`) read per request, proven end-to-end by `tests/enforcement.rs`. |
| `ORGANIZATION_TOKEN_ISSUER` | `authentication-service` | Expected `iss` (see [`authentication-sessions.md`](../../../agents/share/authentication-sessions.md) §5 claims). |
| `ORGANIZATION_TOKEN_AUDIENCE` | `main-x-service` | Expected `aud` (see [`authentication-sessions.md`](../../../agents/share/authentication-sessions.md) §5 claims). |
| `ORGANIZATION_REQUIRE_AUTH` | unset ⇒ **off** | Blanket `/api/*` enforcement (credential is now a PASETO v4.public token or BFF cookie session). Lenient bool: `1`/`true`/`yes`/`on` ⇒ on; else off. See [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md) (credential superseded by [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)). |
| `ORGANIZATION_ABAC_POLICY` | unset ⇒ built-in default policy | ABAC authorization policy as inline JSON (evaluated only when enforcement is on). Unparsable ⇒ warn-log + built-in default. See [`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md). |
| `ORGANIZATION_ABAC_POLICY_FILE` | unset | Path to the ABAC policy JSON file (used when `ORGANIZATION_ABAC_POLICY` is unset). Unreadable/unparsable ⇒ warn-log + built-in default. Polled every 15s for changes by `auth::spawn_policy_watcher` (AU-2) and hot-reloaded; a malformed edit falls back to the built-in default rather than leaving the service unprotected. |
| `ORGANIZATION_EVENT_TRANSPORT` | `memory` | Durable event-bus transport ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §7). `memory` ⇒ the process-wide ring buffer (Phase 1; no DB, no tx — today's behaviour). `outbox` ⇒ the transactional outbox (Phase 2): every CRUD/merge handler writes one `event_outbox` row **on the same transaction** as the entity mutation, so the change and its event commit or roll back together. Unrecognised value ⇒ `memory` (fail-safe). Read once at boot and cached. |
| `ORGANIZATION_EVENT_RELAY` | off | Phase-3 relay switch. Truthy (`1`/`true`/`yes`/`on`) **and** `EVENT_TRANSPORT=outbox` ⇒ `App::after_routes` spawns the background relay loop (`src/relay.rs`: drain `event_outbox` → `EventSink` → `mark_published`, + periodic retention purge). Off by default ⇒ no loop. |
| `ORGANIZATION_EVENT_RELAY_INTERVAL_SECS` | `5` | Relay drain-loop tick interval (floored at 1). |
| `ORGANIZATION_SEARCH_INDEX_PATH` | `data/search-index` | Directory holding the Tantivy full-text index (`src/search/`), created if absent. The index is a **derived artifact**: Postgres remains the source of truth and every hit is resolved against it, so a stale index degrades (a missing hit) rather than corrupts (it can never resurrect a deleted record). If the directory cannot be opened, search and `check-duplicates` return `503` rather than an empty result. An **empty index over a non-empty table triggers a background rebuild at boot**, so an upgrade or a lost volume self-heals; `cargo loco task search_reindex` rebuilds on demand. |
| `ORGANIZATION_SEARCH_BOOT_REINDEX` | unset ⇒ **on** | Boot-time self-heal: when the index is empty **and** the table is not, rebuild in the background (`App::after_routes`). Falsy (`0`/`false`/`no`/`off`) ⇒ skip, for a deployment large enough to want the rebuild scheduled by hand. Anything else (including an unrecognised value) leaves it on, because the failure it prevents is silent. |
| `ORGANIZATION_EVENT_RETENTION_DAYS` | `7` | Outbox row TTL. **Enforced** by the Phase-3 relay's periodic `purge_published` (deletes `published_at < now() - INTERVAL '<n> days'`) when the relay runs ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §3). |
| `ORGANIZATION_BULK_ARTIFACT_DIR` | system temp dir + `organization-bulk-artifacts` | Base directory for BLK-5 bulk import/export artifacts (uploaded input, export output, error report) — `src/bulk/store.rs`, local-filesystem only this rollout step (§10.7). |
| `OTLP_ENDPOINT` | `http://localhost:4317` | OTLP/gRPC collector endpoint (`src/observability.rs`). Set to the empty string to disable export. **Deliberately not `ORGANIZATION_`-prefixed** — one family-wide variable name, per [`agents/share/rust-tracing-opentelemetry-stack.md`](../../../agents/share/rust-tracing-opentelemetry-stack.md). |
| `OTLP_SERVICE_NAME` | `organization-service` | `service.name` OTel resource attribute (`src/observability.rs`). Also deliberately unprefixed. |

## 8. Architecture

loco `App` (`src/app.rs`) registers the organizations controller. One
`organizations` table stores `pid` + denormalised `name` + the full
`Organization` JSONB `data`. Matching calls `organization-matcher`
directly on the deserialised payloads — no adapter.

**Cross-service linking — target only (v1).** Per
[`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md),
organization is a v1 link **target**: inbound edges point at it
(`person → organization` `works_at`/`member_of`; `worker → organization`
`employed_by`). It therefore has **no `entity_links` write-side table and
no `/links` surface**. It participates by (a) emitting its existing
`created`/`deleted`/`merged` events, which feed the aggregator's
`entity_presence` verification oracle and merge-repointing, and (b) being
addressable as an `EntityRef` URN `organization:<pid>` so inbound edges
resolve. The inverse edges (`has_member`, `employs`) are materialised in
the aggregator read-model, **not** stored here. Origination from the org
side is a roadmap item (umbrella spec §15).

**Bulk import / export (BLK-5, delivered — §10.7, §13).** The family-wide
contract — async loco-worker jobs, the `bulk_jobs` table, the five
`/api/organizations/{import,export,bulk-jobs}` endpoints, idempotent
upsert-by-key, the per-row error report, and the export privacy/audit
posture — is fixed in
[`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md).
This rollout step is deliberately bounded to what BLK-1/BLK-2 require:
**JSONL + CSV only** (no Parquet — a later, person-specific extra) and a
**local-filesystem-only** artifact store (no S3 backend). §10.7 declares
this crate's per-entity adoption in full: the stable key (`Lei` → `Duns` →
explicit `pid` — narrower than the matcher's full deterministic-scheme
list, by design; see §10.7), the CSV column set, and export sensitivity.

## 9. API surface

See §6. Responses are raw loco JSON. `404` for unknown `pid`; `422
Unprocessable Entity` for validation failures (blank `name` on create
or replace — family convention); `400` for malformed requests (blank
search `q`, invalid audit pid).

**API versioning.** `/api/*` URLs carry no version segment; a client
selects the representation version with the `Accepts-version` request
header (default `1.0` when omitted), per
[`agents/share/api-versioning.md`](../../../agents/share/api-versioning.md).
`src/version.rs` (`resolve_version`, pure/unit-tested) is layered as
`require_version_mw` in `App::after_routes` alongside the auth guard: an
explicit but unsupported version is `406 Not Acceptable`; a supported (or
omitted) one is stamped back onto the response as `Accepts-version`. A
bare major (`1`) aliases its current minor (`1.0`). Copied from the
event-service reference implementation (§13).

**FHIR R5 (`Organization`) — family reference implementation.**
`src/fhir/{mod,resources,search}.rs` + mounted `src/controllers/fhir.rs`
serve `GET`/`POST`/`PUT`/`DELETE /fhir/Organization{,/{id}}`, `GET
/fhir/Organization?<params>` (a searchset `Bundle`; supported params:
`_id`, `_lastUpdated`, `_count`, `identifier`, `name`, `address`,
`address-city`, `address-postalcode`), and `GET /fhir/metadata` (the
`CapabilityStatement`), per
[`agents/share/fhir.md`](../../../agents/share/fhir.md). Every non-2xx
response is a FHIR `OperationOutcome`; every response carries
`application/fhir+json`. These routes reuse the native model helpers,
validators, event/audit path, and sit behind the same blanket
auth+ABAC guard as `/api/*` (`/fhir/*` is not on the public allow-list).
Organization was built first and is the copy source for the other
in-scope services (§13).

**Bulk import/export (BLK-5, §10.7).** `POST /api/organizations/import`
(multipart: `file` + `format` + optional `dry_run`) → `202 {job_id}`;
`GET /api/organizations/import/{id}` → job status + counts +
`errors_url`; `POST /api/organizations/export` (JSON: `format`, `q`,
`limit`, `offset`, `masking_profile`, `include_soft_deleted`) → `202
{job_id}`; `GET /api/organizations/export/{id}` → job status +
`download_url`; `GET /api/organizations/bulk-jobs[?limit=]` → recent
jobs, newest first. `import` is a declared destructive POST
(`auth::DESTRUCTIVE_POST_SUFFIXES`); an elevated (`full` masking or
soft-deleted) export additionally requires `Action::Destructive` via
`authorize_record`. `format` accepts `jsonl`/`csv`/`tsv` (`400`
otherwise; TSV shares the CSV codec via a declared, never-sniffed
delimiter — §13 2026-08-21); `include_soft_deleted=true` is `400` (not
yet supported).

**Auth.** The credential is a short-lived **PASETO v4.public** token
(Ed25519, riding in `Authorization: Bearer v4.public.…`), verified
offline against the auth-service's published Ed25519 key — see
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
(source of truth; supersedes the prior RS256-JWT + JWKS model).
`GET /api/organizations/whoami` always requires a valid bearer
token (the `AuthUser` extractor; `401` otherwise); other handlers take
`MaybeAuthUser` to stamp the audit/merge `actor` when a token is present.
When `ORGANIZATION_REQUIRE_AUTH` is on (see §7), an `axum` middleware
layer (`App::after_routes` → `auth::enforce`) requires a valid bearer
token on **every** route except the public health/ping, OpenAPI/Swagger,
and `/metrics.prom` paths, returning `401` otherwise. The flag is read
per request and is **off by default**, so default behaviour is unchanged.

**Authorization (ABAC).** Inside the same guard — so only when
`ORGANIZATION_REQUIRE_AUTH` is on — a verified token is authorized by
**attribute-based access control** per
[`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md):
the request's action is derived from the HTTP method plus this crate's
destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`,
`/deduplicate`, `/import`; the dedup scan is live as of 2026-07-19,
`/import` stays ahead of the bulk feature), and the shared engine in
`authentication-verifier` 0.3 evaluates the policy over the token's
`attrs` claim, first-match-wins. Configure with `ORGANIZATION_ABAC_POLICY`
(inline JSON) or `ORGANIZATION_ABAC_POLICY_FILE` (path); unset or
unparsable ⇒ warn-log + the built-in default policy (any authenticated
subject reads; `access=write` writes; `access=admin` adds DELETE/merge;
`svc=true` does everything). `401` = missing/bad credential; `403` =
valid credential, policy denied (the body names the deciding rule). This
supersedes the earlier per-crate roles/RBAC sketch.

**Observability.** `GET /metrics.prom` (root path, public) serves the
process-wide Prometheus registry (`src/metrics.rs`) in text-exposition
format. The metric set: `organization_created_total`,
`organization_updated_total`, `organization_deleted_total`,
`organization_merged_total` (counters incremented one per success path
in the CRUD/merge handlers), plus a labelled `http_requests_total`
(`path`/`status`) reserved for a future request middleware. Configure
the scraper with `metrics_path: /metrics.prom`.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migration
`m20220101_000001_organizations`. `auto_migrate` on in development.

### 10.7 Bulk import/export (BLK-5) — per-entity adoption

Per `agents/share/bulk-import-export.md` §10, this section is
organization's declaration of what differs from the family-wide
contract. `bulk_jobs` (migration `m20260803_000002_bulk_jobs`) is the
uniform job table (`id UUID PRIMARY KEY`, `kind`, `entity`, `format`,
`status`, `params JSONB`, the five `rows_*` counters, `actor`,
`idempotency_key`, `input_url`/`result_url`/`error_report_url`,
`created_at`/`updated_at`/`expires_at`), plus a `provenance` column
added to the existing `review_queue` table
(`m20260803_000001_review_queue_provenance`: `operator` | `import` |
`matcher_suggested`, backfilled `operator`, set once on insert and never
touched by a re-scan upsert).

**Stable key** (priority order, `src/bulk/stable_key.rs`):

1. An **LEI** identifier (`IdentifierScheme::Lei`) with a non-blank value.
2. Failing that, a **DUNS** identifier (`IdentifierScheme::Duns`) with a
   non-blank value.
3. Failing that, the row's own **explicit `pid`**.

This is narrower than the matcher's full deterministic-scheme list
(`Lei`/`Duns`/`Iso6523`/`Gln`/`Wikidata`/`Ror`/`Isni`/`Vat`) — LEI and
DUNS are the two schemes an operator's source system most plausibly
carries as a primary key for a bulk load; the others remain valid
*matching* short-circuits but are not (yet) bulk stable keys. A row
satisfying none of the three is **keyless** and routes through
duplicate detection (§6 of the family doc) instead of a blind create.

`organization_matcher::Organization` carries **no id field of its
own** (unlike person's `Person::id`) — `pid` is server-assigned on
create. The bulk **wire row** is therefore the organization's own
fields plus a top-level, optional `pid` (`src/bulk/columns.rs`
`to_row_value`/`from_row_value`); re-importing an export (which always
carries the real `pid`) is what makes priority 3 idempotent. An
explicit `pid` that does not resolve to an existing row (e.g. it names
a soft-deleted record, which `find_by_pid` excludes) is **not**
preserved on create — a fresh `pid` is minted, since
`models::organizations::Model::create` has no caller-supplied-id entry
point. This is a deliberate, narrow scope decision (documented in
`src/bulk/pipeline.rs`), not an oversight.

**CSV column set** (`src/bulk/columns.rs`, export order): `pid`,
`name`, `legal_name`, `url`, `jurisdiction`, `founding_date`,
`telephone`, `email` (scalars); `address.street_address`,
`address.locality`, `address.region`, `address.postal_code`,
`address.country` (the single nested `address` object, dotted);
`identifiers`, `alternate_names`, `same_as`, `keywords` (arrays, one
JSON-encoded cell each). JSONL is the lossless reference format
(one wire row per line); CSV round-trips losslessly against it
(pinned by `bulk::csv::tests::round_trips_a_fully_populated_organization_losslessly`).
**TSV** (landed 2026-08-21, §13) reuses this same column set and codec
unchanged — it differs from CSV in exactly one byte, the delimiter,
declared by `format` and never sniffed from the file content.

**Export sensitivity.** Reuses the existing masking exactly
(`crate::privacy::mask_organization` — §7 masking task): the default
`masked` profile redacts `telephone`, `email`, the street address line,
and fiscal (`TaxId`/`Vat`) identifier values; the privileged `full`
profile is gated behind `authorize_record(.., Action::Destructive, ..)`
(a no-op unless `ORGANIZATION_REQUIRE_AUTH` is on). `include_soft_deleted
= true` is rejected at the handler with `400` (before a job is even
created) as not-yet-supported, per the family doc's "note it as
deferred, don't half-build it" guidance — `models::organizations` has
no soft-deleted listing query today. Every export writes an audit row
(`AuditModel::record(.., "bulk_exported", ..)`, keyed by the job id) and
the write **gates delivery** (SEC-B8): a failed audit write fails the
whole export job before the artifact is stored or `result_url` is set.
Import audit (`"bulk_imported"`) is best-effort (the rows are already
individually audited via `streaming::create_and_emit`/`update_and_emit`).

**Concurrency (known limitation).** Unlike the family's SEC-B3
reference pattern (an advisory-lock guard transaction wrapping
find-then-write), this crate's per-row upsert (`bulk::pipeline::import_upsert`)
is a **plain** find-then-write with no lock. `streaming::create_and_emit`
/ `update_and_emit` are hard-coded to `&DatabaseConnection` (they open
their own transaction internally under the `outbox` transport), so a
lock held on a separate guard transaction would occupy one pooled
connection while these need a second — under a small pool (this
crate's own `config/test.yaml` runs `max_connections: 1`) that
deadlocked every single import, not only a concurrent one, when first
tried. Two importers racing the *same* stable key in the same instant
can therefore both create a row. Closing this properly needs a
`ConnectionTrait`-generic `streaming::create_and_emit`/`update_and_emit`
— a `src/streaming.rs`-wide change, out of BLK-5's scope; tracked as a
follow-up in §13.

**Artifact store** (`src/bulk/store.rs`) — **local-filesystem only**
this rollout step (`ORGANIZATION_BULK_ARTIFACT_DIR`, default a system-temp
subdirectory); no S3 backend. The `ArtifactStore` trait is still async
(a future S3 addition needs no signature change, the lesson learned
from person's/care-pathway's own sync→async S3 rollouts).

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON
round-trip) and unit tests in `src/` (validation → `422` pin, OpenAPI
shape incl. the `/metrics.prom` path, the Prometheus registry render +
counter increment in `metrics::tests`, streaming, and `auth::tests` —
`bearer_claims` plus the pure
`enforce`/`parse_bool` decision: off+no-token ⇒ ok, on+public ⇒ ok,
on+protected without/expired/tampered ⇒ `401`, on+protected+valid ⇒
ok). Request-level tests (`tests/requests/organizations.rs`): boot the
real app via loco's `testing` harness and cover create round-trip,
blank-name `422` (create + update), unknown-pid `404`, search
(keyword hit; index follows update + delete; fuzzy / phonetic modes),
check-duplicates (including identifier-only blocking), merge, `whoami` `401`, the blanket-enforcement
gate (with `ORGANIZATION_REQUIRE_AUTH=1` set in-test, un-authed `GET
/api/organizations` ⇒ `401` while `GET /api-docs/openapi.json` ⇒
`200`; `#[serial]` for env-var ordering), the audit endpoints
(`/audit/recent` + `/{pid}/audit` record CRUD actions; invalid pid ⇒
`400`), and the plain-CRUD `created`/`updated`/`deleted` events on
`/events/recent`. These require Postgres
(`config/test.yaml`) and are `#[ignore]`d so the default `cargo test`
stays green — run with `cargo test -- --ignored`.

**BLK-5 bulk import/export.** DB-free unit tests throughout `src/bulk/`:
the wire-row shape round-trip (`columns::tests`), the CSV codec against
a fully-populated organization (`csv::tests`, incl. reordered/extra
columns, a bad-JSON-cell per-row error, `pid`-column explicitness), the
JSONL codec, the stable-key precedence (LEI → DUNS → explicit `pid` →
keyless; `stable_key::tests`), the local artifact store (SEC-B4
confinement, unsafe-key rejection; `store::tests`), the pure export
helpers (masking application, elevation-required gate, export-limit
clamp; `pipeline::tests`), and the audit-summary builders
(`worker::tests`). Request-level tests
(`tests/requests/bulk.rs`, Postgres-gated, `#[ignore]`d): JSONL
import-then-reimport upserts idempotently by LEI (no duplicate row);
JSONL export round-trips a created organization; a CSV import/export
round trip; a keyless row with a likely duplicate is created **and**
queued in the review queue with `provenance = "import"`; export masks
by default and the `full` profile is unmasked and audited;
`include_soft_deleted=true` and an unsupported format are both `400`
before a job is created; `GET /bulk-jobs` lists a submitted job. These
rely on `config/test.yaml`'s `workers.mode: ForegroundBlocking`, under
which `BulkJobWorker::perform_later` runs synchronously, so a job's
terminal status is observable on the very next request — no polling.

**Dedicated Postgres-gated test binaries** (each its own process because
`require_auth`/`policy`/`verifier`/`transport` are process-wide
`OnceLock`s that would otherwise leak between suites; each `#[ignore]`d,
run with `cargo test --test <name> -- --ignored`):

- `tests/enforcement.rs` — the AU-2 "activation proof" against the real
  router with `ORGANIZATION_REQUIRE_AUTH=1`: public paths stay open, a
  protected read/write without a token is `401`, a malformed bearer is
  `401` (not `500`), a valid token with no attributes reads `200`/writes
  `403`, and `access=write` creates.
- `tests/masking.rs` — the privacy layer end to end: a policy allow
  carrying the `mask` obligation redacts the ordinary `GET`, carries into
  the export, and both are audited. Mutation-checked (dropping the
  obligation branch fails the suite).
- `tests/outbox_audit.rs` — under the `outbox` transport, `create_and_emit`
  writes the entity, its `event_outbox` row, and its `audit_logs` row in
  one transaction.
- `tests/seed_examples_db.rs` — the `seed_examples` task (EX-4): a first
  run seeds all 20 fixture rows, a second run is a no-op.
- `tests/fluvio_relay.rs` — `#![cfg(feature = "fluvio")]`-gated (compiles
  to zero tests on a default build) round-trip against a real Fluvio
  broker (BUS-3); needs `compose.fluvio.yaml`, so no automated run in
  this repo exercises it — verified by compiling under the feature.

## 12. Compliance

Organization data is largely public, but contact fields may be personal
data. Per-field masking, the masked view, and the audited GDPR
right-of-access export have landed (§13, §6.12–13) and honour the ABAC
`mask` obligation; there is deliberately no consent model (§2) — an
organization is not a data subject.

## 13. Tasks (live work queue)


- [x] **2026-08-21 — TSV bulk format + fuzzed row decoders.**
  `BulkFormat::Tsv` is accepted for import and export alongside `jsonl`
  and `csv`. TSV shares the CSV codec rather than forking it — the codec
  takes a `delimiter: u8` and `BulkFormat::delimiter()` is the one place
  that maps format → byte. The delimiter is declared by the caller, never
  inferred: reading CSV as TSV resolves no column and reconstructs the
  row from nothing (a per-row error here, since a required field is
  missing; a silently empty record for an entity whose fields were all
  optional), and a test pins that the data is not recovered either way.
  Tab-containing fields are quoted by the `csv` crate, so free text is
  safe. A new `bulk_decoders` fuzz target drives both delimiters and the
  JSONL path over arbitrary bytes, pinning never-panic, determinism, and
  the §7 per-row error contract.


- [x] **2026-08-21 — FUZZ-2: cargo-fuzz harness for the request-path
  logic.** Three coverage-guided targets over the pure, total code that
  faces the network: `validate_json` (bytes → `serde_json` → `Organization` →
  `validation::problems`), `validate_built` (the validator driven from
  raw bytes, so the fuzzer controls array cardinality directly rather
  than having to learn JSON first), and `merge_orgs` (the merge fold over
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
  response. Worse here than a blank check: each entry also ran the SEC-M5
  check-digit validation for its declared scheme. Every per-entry loop now walks the new `inspected()`
  helper (at most `MAX_ARRAY_LEN` entries, index preserved): the
  cardinality problem already rejects the payload, so inspecting the tail
  decides nothing, and bounding the **report** is the same input-bounding
  rule as bounding the work (SEC-M1). Named rather than inlined so a loop
  added later without it reads as different. Pinned by a test, which was
  confirmed to fail without the cap. Case was the reference
  (repo `tasks.md` SEC-M8/SEC-M8b).

- [x] **2026-08-20 — Search writer held for the process; Criterion
  benchmarks; declared MSRV.** `SearchEngine::index_organization` (and the
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

- [x] **BLK-5: async bulk import/export (organization half).** Delivers
  the family contract in
  [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md)
  for this crate, scoped to what BLK-1/BLK-2 need: **JSONL + CSV only**
  (no Parquet) and a **local-filesystem-only** artifact store (no S3).
  New `src/bulk/` (`mod`, `columns`, `csv`, `jsonl`, `stable_key`,
  `error_report`, `pipeline`, `store`, `worker`, `handlers`), a
  `bulk_jobs` migration/entity/model (`m20260803_000002_bulk_jobs`), and
  a `review_queue.provenance` column
  (`m20260803_000001_review_queue_provenance`, mirroring person's
  `m20260802_000001`). Full details — stable key (LEI → DUNS → explicit
  `pid`), the CSV column set, export sensitivity, and the documented
  non-concurrent-safe limitation — are in §10.7; the five endpoints are
  in §9; the test suite is in §11. Every written row goes through the
  existing `streaming::create_and_emit`/`update_and_emit`, so a
  bulk-imported organization gets the same event/audit/search-index
  side effects as one created interactively — no bypass of the audit
  trail. **Follow-up (not this task):** a `ConnectionTrait`-generic
  `streaming::create_and_emit`/`update_and_emit` so the per-row upsert
  can be wrapped in a SEC-B3 stable-key advisory lock without a pool
  deadlock (§10.7 "Concurrency"); an S3 `ArtifactStore` backend behind a
  cargo feature (the trait is already async, so this is additive).

- [ ] **SEC-B3: advisory-lock-protect the BLK-5 per-row bulk upsert.**
  `bulk::pipeline::import_upsert` (§10.7 "Concurrency") is a **plain**
  find-then-write with no lock — unlike the family's SEC-B3 reference
  pattern (person; an advisory-lock guard transaction wrapping
  find-then-write). Two importers racing the identical stable key
  (LEI → DUNS → explicit `pid`) in the same instant can both pass the
  "does it exist" check and both `INSERT`, producing a duplicate row.
  **Why it isn't already fixed:** `streaming::create_and_emit` /
  `update_and_emit` are hard-coded to `&DatabaseConnection` and open
  their *own* internal transaction under the `outbox` transport, so they
  are not generic over `ConnectionTrait`. A stable-key advisory lock
  taken on a separate guard transaction would hold one pooled connection
  while these helpers need a second — under a small pool (this crate's
  own `config/test.yaml` runs `max_connections: 1`) that was tried and
  deadlocked **every** import, not only a concurrent one. **Scope of the
  real fix** (not part of this task item — this item only tracks it):
  make `streaming::create_and_emit`/`update_and_emit` generic over
  `ConnectionTrait` (a `src/streaming.rs`-wide change, since every
  caller passes `&DatabaseConnection` today), then have
  `bulk::pipeline::import_upsert` take a stable-key `pg_advisory_xact_lock`
  inside the *same* transaction it upserts in, matching the SEC-B3
  reference shape with no separate connection needed. See §10.7 and the
  repo [`tasks.md`](../../../tasks.md) SEC-B3 entry (closed there for
  person; open here).

- [x] **SEC-M5 (security): check-digit / format validation of deterministic
  identifiers.** `validation::problems` now validates LEI (ISO 17442 + ISO
  7064 MOD 97-10), GLN (13 digits + GS1 mod-10), DUNS (9 digits), and VAT
  (country-prefix format) before store, since they drive the matcher's
  deterministic short-circuit — a malformed one could produce a false
  deterministic match. A bad value is a field-scoped `422`; non-deterministic
  schemes are unconstrained. Pure check-digit helpers unit-tested. (Repo
  tasks.md Phase 5 SEC-M5.)

- [x] **SEC-M1 (security): input-size caps on the `Organization` payload.**
  New `src/validation.rs` (`problems`) bounds every scalar text field
  (`MAX_TEXT_LEN = 1024`, incl. nested `address.*`), array cardinality
  (`MAX_ARRAY_LEN = 256`), and per-entry length (`MAX_ITEM_LEN = 512`),
  keeping the blank-`name` / non-blank-`identifiers[i].value` rules — all
  collected into one `422` before the record is stored or matched, closing
  the O(n·m) matcher `DoS`. Controller `validate` delegates to it. Unit
  tested. (Repo tasks.md Phase 5 SEC-M1.)

- [x] Event streaming + audit log on CRUD. **Phase 1 (in-memory
  envelope + `EventPublisher` seam) implemented** per
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md):
  `src/streaming.rs` carries the canonical versioned `Envelope`
  (`event_id`, `schema_version` = 1, `entity`, `kind`, `pid`, `seq`,
  `actor`, `name`; `occurred_at`/`data` deferred to the outbox stage),
  an `EventPublisher` trait, and an `InMemoryPublisher` ring buffer
  (process-wide `OnceLock`). The operator endpoint
  `/api/organizations/events/recent` returns the frozen flat
  `EventView { kind, pid, name, seq }` projection (wire shape unchanged
  — front-end safe). CRUD/merge call sites stamp the bearer `actor`.
- [x] **Durable event bus — Phase 2 (transactional outbox).** This is the
  family **reference** implementation
  ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §3–§8).
  New `event_outbox` table (`migration/…_000004_event_outbox`: `BIGSERIAL
  id`, `event_id UUID UNIQUE`, `entity`, `entity_pid`, `kind`,
  `occurred_at`, `actor`, `schema_version`, `payload JSONB`,
  `published_at`, partial index on unpublished rows); SeaORM entity
  `models/_entities/event_outbox.rs`; `models/event_outbox.rs` with the
  **pure** DB-free `OutboxInsert::from_envelope` mapping (unit-tested),
  `insert_on(&impl ConnectionTrait)`, `recent(db, limit) → Vec<EventView>`,
  and the relay poll/ack (`unpublished`/`mark_published`, unused until
  Phase 3). New `EventTransport`/`transport()` selector +
  `OutboxPublisher` in `src/streaming.rs`, plus transport-aware
  `create_and_emit`/`update_and_emit`/`delete_and_emit`/`merge_and_emit`
  used by **both** the native and FHIR controllers. The model write
  helpers (`create`/`update_data`/`soft_delete`) are now generic over
  `sea_orm::ConnectionTrait`, so the `outbox` path runs the entity write
  **and** the `event_outbox` insert on one `db.begin()` transaction (crash
  can't persist one without the other); `memory` keeps the ring buffer,
  no tx. Gated by `ORGANIZATION_EVENT_TRANSPORT` (default `memory` ⇒
  behaviour and existing tests unchanged). Tests: DB-free envelope→row
  mapping (create/update/delete/merge fields, non-UUID pid rejected),
  transport-string parse, `EventView` projection frozen; DB-gated
  (`tests/requests/event_outbox.rs`, `#[ignore]`) atomicity — one tx
  writes org + exactly one outbox row, a rollback drops both.
- [x] **Durable event bus — Phase 3 (relay + retention).** `src/relay.rs`:
  the `EventSink` trait (the bus seam), a working no-broker **`LoggingSink`**
  default, `drain_once` (`unpublished` → `sink.send` → `mark_published`,
  at-least-once, per-pid order preserved on a send failure), and
  `purge_published` (retention). A background loop (`relay::spawn`, started
  in `App::after_routes`) ticks every `ORGANIZATION_EVENT_RELAY_INTERVAL_SECS`
  and purges every N ticks — **gated by `ORGANIZATION_EVENT_TRANSPORT=outbox`
  AND `ORGANIZATION_EVENT_RELAY`**, so it is a no-op by default. Tests:
  DB-free `LoggingSink`/capturing-sink send + config defaults; the drain/ack
  seams (`unpublished`/`mark_published`) are DB-gated-tested via the outbox
  suite.
- [x] **Durable event bus — Phase 3, `FluvioSink` (BUS-3).**
  *(done 2026-08-03, ported from case-service's BUS-1 reference)* The
  real-broker `impl EventSink`, behind this crate's own `fluvio` Cargo
  feature (off by default — the dependency tree and boot behaviour of a
  default build are unchanged). One producer per topic
  (`fluvio::Fluvio::connect_with_config` + `topic_producer`, held for the
  sink's lifetime), partitioned by record `pid` per §7. Config:
  `ORGANIZATION_FLUVIO_ENDPOINT` (the broker's SC address; unset ⇒
  `LoggingSink`, unchanged default behaviour) and `ORGANIZATION_EVENT_TOPIC`
  (default `mxi.organization.events`). **No silent fallback**: an endpoint
  configured **without** the `fluvio` feature refuses to start the relay at
  all (logged at `error`), rather than a `LoggingSink` masquerade that
  would mark outbox rows `published_at` without ever reaching the broker
  the operator asked for — the same shape as the family's artifact-store
  "no fallback on an explicit backend choice" rule
  (`agents/share/bulk-import-export.md` §12). The initial connection
  retries indefinitely rather than falling back, for the same reason.
  `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a local SC+SPU
  broker (Fluvio's own documented Docker Compose layout, translated to
  this repo's Podman conventions) for opt-in manual runs; **not** wired
  into any automated CI stage. Tests: `cargo build`/`clippy --all-targets
  -D warnings`/`fmt --check` clean under both default features and
  `--features fluvio` (the real `fluvio` 0.50 API compiling is the actual
  verification of correct usage); `tests/fluvio_relay.rs` is a
  `#![cfg(feature = "fluvio")]`-gated, `#[ignore]`d round-trip (enqueue →
  `FluvioSink` → `drain_once` → assert `published_at`) with its run
  command documented inline — it needs a live broker, which no automated
  run in this repo stands up, so it is verified by compiling under the
  feature, not by an actual execution (same posture as case's
  `tests/fluvio_relay.rs`, BUS-1, and person's
  `s3_round_trip_against_a_live_endpoint`, BLK-4). This crate has no
  `compliance/soup.tsv` (unlike case), so no SOUP register update was
  needed.
- [x] Name search + OpenAPI/Swagger. *(Postgres `ILIKE`; superseded by
  the Tantivy item below, which removed the `ILIKE` query and its
  `escape_like` guard rather than leaving them dormant.)*
- [x] Prometheus metrics — `GET /metrics.prom` (root, public) serves a
  process-wide `prometheus::Registry` (`src/metrics.rs`, `OnceLock`)
  in text-exposition format; CRUD/merge handlers increment
  `organization_{created,updated,deleted,merged}_total`; a labelled
  `http_requests_total` is declared for a future request middleware.
  Brings parity with the older Axum services. DB-free render test +
  OpenAPI path test; `/metrics.prom` added to `auth::is_public_path`.
- [x] **Tantivy full-text search + fuzzy/phonetic + blocking**
  (replacing the `ILIKE` search). `src/search/{mod,index}.rs`: the
  index schema (`pid` stored; name / legal name / alternate names /
  phonetic codes / identifiers / keywords / address / url full-text;
  `jurisdiction` + `active` exact), a `SearchEngine` facade
  (`index_organization` — idempotent replace-in-place —
  `delete_organization`, `clear`, `search`, `fuzzy_search`,
  `phonetic_search`, `candidates`, `stats`), and a process-wide
  `OnceLock` engine keyed on `ORGANIZATION_SEARCH_INDEX_PATH`.
  Indexing is wired into `src/streaming.rs` (the one seam both the
  native and FHIR controllers write through), **after** the write is
  durable and best-effort: a failed index write is logged at `ERROR`
  and never fails a committed request. `GET /search` gains `fuzzy` /
  `phonetic` (§6.11); `check-duplicates` now **blocks** on the index
  instead of scanning up to 1000 rows, so a duplicate beyond the old
  cap is reachable. Recovery: `cargo loco task search_reindex`
  (`src/tasks/search.rs`, paginated, clears first, skips unreadable
  payloads) plus an automatic boot rebuild when the index is empty and
  the table is not. Tests: 16 DB-free unit pins (exact / fuzzy /
  phonetic retrieval, secondary fields, replace-not-duplicate, delete,
  clear, identifier-only and fuzzy-name blocking, empty and
  unparseable queries, zero limit, tokenise / Soundex / address
  helpers) + 4 DB-gated request tests (keyword hit, index follows
  update and delete, fuzzy/phonetic modes over the wire,
  identifier-only duplicate detection).
  - **Not** wired to Tantivy: the FHIR `GET /fhir/Organization` search,
    which is a structured multi-parameter filter (`identifier`,
    `address-city`, …) over a capped scan rather than a free-text
    query. Moving it is a separate item, not a side effect of this one.
- [x] **Per-field masking + GDPR export endpoint.** `src/privacy.rs`:
  `mask_organization` (telephone → tail, email → first char + domain,
  `street_address` dropped, `TaxId`/`Vat` values masked; registry
  identifiers and the public fields untouched) and
  `export_organization` (the `{entity, pid, exported_at, masked,
  record, note}` envelope). Endpoints `GET /{pid}/masked` and
  `GET /{pid}/export` (§6.12–13), the export **audited** either way.
  `src/auth.rs` gains `authorize_record` + `organization_resource_attrs`
  so `GET /{pid}` and the export honour the ABAC **`mask` obligation**
  (case is the family reference). **Consent is deliberately absent**: an
  organization is not a data subject, and the natural persons behind one
  are the person service's to record — a second, unauthoritative home
  for consent is worse than none. Tests: 10 DB-free masking/export pins
  + a dedicated `tests/masking.rs` binary proving the obligation
  redacts the ordinary `GET`, carries into the export, and audits both.
- [x] Record merge — `POST /merge` folds a duplicate into a survivor
  (union fields, former-name alias, soft-delete, `merge_records` history
  + snapshot, `Merged` event); pure `src/merge.rs`; `/merges/recent`.
- [~] Richer validation (identifier formats, URL, country codes).
  **Partial**: SEC-M5 (below) validates the deterministic-identifier
  formats (LEI/GLN/DUNS/VAT check digits). Still open: URL well-formedness
  and country-code (ISO 3166) validation — `src/validation.rs` only
  length-bounds `url` and `address.country` today, it does not check
  their shape.
- [ ] Cross-service link **target** readiness — organization is a v1
  link target only (§8;
  [`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md)),
  so no `entity_links` table. Confirm the `created`/`deleted`/`merged`
  events carry the fields the aggregator's presence oracle + merge-repoint
  need (`pid`; `merged_from` on merge), and confirm the matcher adapter
  never sees cross-service links (only within-entity `relationships[]`
  reach `MatchingEngine`). Mirrors umbrella spec §13 T-13.
- [x] Request-level integration tests (Postgres; `#[ignore]`-gated).
- [x] Offline token verification — `src/auth.rs` embeds
  `authentication-verifier` behind a process-wide `Verifier`
  (env-configured keys/issuer/audience); `AuthUser`/`MaybeAuthUser`
  extractors; `/whoami` protected; audit + merge `actor` from the token.
  (Originally shipped against the prior RS256-JWT + JWKS model.)
  - [x] **Switch to PASETO v4.public** per
    [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
    **Done:** `src/auth.rs` verifies PASETO v4.public tokens
    against the auth-service's published Ed25519 key (`authentication-verifier`
    0.2 `from_paseto_keys_*`); same `Claims` shape (`kid`/`iss`/`aud`/`exp`);
    env vars `ORGANIZATION_PASETO_KEYS` / `ORGANIZATION_TOKEN_ISSUER` /
    `ORGANIZATION_TOKEN_AUDIENCE`. Supersedes the RS256-JWT model.
  - [x] Blanket `/api/*` enforcement — `auth::enforce` (pure, unit-tested)
    wired as an `axum::middleware::from_fn` layer in `App::after_routes`,
    gated by `ORGANIZATION_REQUIRE_AUTH` (lenient bool, **default off**).
    Public paths (`/_health`, `/_ping`, `/api-docs/openapi.json`,
    `/swagger-ui*`) stay open; everything else needs a valid bearer token
    when the flag is on. Off by default keeps current behaviour and the
    existing DB-gated tests green. Family contract:
    [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md)
    (credential now PASETO per `authentication-sessions.md`; `enforce()`
    shape unchanged).
  - [x] paseto-keys-over-HTTP fetch at boot (vs env injection) — fetch +
    cache the auth-service `/.well-known/paseto-keys` at startup.
    **Done 2026-07-04:** new `ORGANIZATION_PASETO_KEYS_URL` env var (§7);
    when set, `auth::init_from_env` (called from `App::after_routes`
    before serving) fetches the key set once via
    `Verifier::from_paseto_keys_url` (`authentication-verifier` `fetch`
    feature) and seeds the process-wide verifier — fetched set wins
    (`tracing::info!`); on fetch failure it warns and falls back to the
    `ORGANIZATION_PASETO_KEYS` env path, so the service always boots.
    Unset/blank URL ⇒ prior behaviour exactly. Originally fetch-once
    only; superseded by AU-2 below, which adds a periodic refresh loop.
    Tests: local ephemeral-port HTTP listener serving the test key set
    (fetched verifier accepts a token signed by that key) + fast-failing
    URL fallback (no panic) + no-URL env path.
  - [x] **AU-2: key rotation + ABAC policy hot-reload without a
    restart** *(2026-08-01; the loco-style half of the rollout — case
    was the reference, the five axum-style services landed the same
    day as AU-1)*. The verifier and the ABAC policy were boot-only
    `OnceLock` snapshots, so a rotated key set or an edited policy file
    could never reach a running process. Now `ReloadableVerifier` /
    `ReloadablePolicy` (both read per request by the blanket guard and
    the bearer extractors) back two background loops started from
    `App::after_routes`: `auth::spawn_key_refresh` re-fetches
    `ORGANIZATION_PASETO_KEYS_URL` every
    `ORGANIZATION_PASETO_KEYS_REFRESH_SECS` (default 3600s; `0`
    disables; a failed fetch keeps the current key set rather than
    locking every caller out), and `auth::spawn_policy_watcher` polls
    `ORGANIZATION_ABAC_POLICY_FILE`'s mtime every 15s and calls
    `reload_policy()` (a malformed edit falls back to the built-in
    default). **Acceptance:** `tests/enforcement.rs`, the activation
    proof against the real router (§11).
- [x] **Bulk import/export — adopt the family contract**
  ([`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md)).
  **Delivered as BLK-5** — see the dedicated entry at the top of this
  section and §9/§10.7/§11 for the full contract, endpoints, stable-key
  rule, and tests. *(DOC-2, 2026-08-04: this item was found still open
  here — describing the identical work — after BLK-5 had already
  shipped and been marked done above; collapsed into one entry rather
  than left duplicated and contradicting itself.)*
- [x] **FHIR R5 API** (`Organization`) — **reference implementation** for
  the family contract (**Done**: `src/fhir/{mod,resources,search}.rs` +
  mounted `src/controllers/fhir.rs`, wired in `app.rs`; read/create/update/
  delete/search at `/fhir/Organization{,/{id}}` + `GET /fhir/metadata`
  `CapabilityStatement`; `OperationOutcome` errors, searchset `Bundle`,
  `application/fhir+json`; 9 DB-free unit tests; clippy-clean)
  ([`agents/share/fhir.md`](../../../agents/share/fhir.md)). Map the stored
  `organization_matcher::Organization` DTO to a FHIR **`Organization`**
  resource (`high` fidelity, §3): `name`/`alias` → `name`/`alias`,
  identifiers (LEI/DUNS/…) → `identifier` (token `system|value`),
  addresses → `address`, telecom → `telecom`, `part_of` → `partOf`
  reference; `active`. New `src/fhir/` module (resource structs,
  `to_fhir_organization`/`from_fhir_organization`, `FhirOperationOutcome`,
  searchset `Bundle`, search-param parsing) + a mounted
  `src/controllers/fhir.rs` (`routes()` added in `app.rs`): read/create/
  update/delete/search at `/fhir/Organization{,/{id}}` + `GET
  /fhir/metadata` `CapabilityStatement`. Reuses the native model helpers,
  validators, event/audit path, and the blanket auth+ABAC guard (§8 —
  `/fhir/*` guarded, action from HTTP method). Supported search params:
  `_id`, `_lastUpdated`, `_count`, `identifier`, `name`, `address`,
  `address-city`, `address-postalcode`. Tests: DTO↔resource round-trip,
  each interaction, search→Bundle, `OperationOutcome` on 404/400/422,
  `CapabilityStatement` matches mounted routes. First; copied by the other
  in-scope services.

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

- [x] **2026-07-19 — Batch dedup + stored review queue + decision
  endpoints.** `POST /deduplicate` (pairwise scan over the R-DUP cap,
  persists candidates), the `review_queue` migration
  (`m20260719_000001`) + raw-SQL `models/review_queue` module
  (normalized-pair upsert / list / first-writer-wins decide — the same
  module the person/worker/place/thing registries share), `GET
  /review-queue`, `POST /review-queue/{id}/decision` (reviewer = bearer
  `sub`; `review_decision` audit row), OpenAPI paths + schemas.
  **Acceptance:** DB-free wire-token + decision serde pins; the
  Postgres-gated request round-trip (scan → stable-id re-scan → list →
  decide → 422 on re-decide → 404 unknown → decided status survives
  re-scan) green — full `--ignored` suite 16/16; clippy pedantic clean.
  The front-end `/review` drag-to-decide board consumes it.

- [x] **Header-based API versioning** *(2026-07-08)*. `src/version.rs`
  (`resolve_version`, pure/unit-tested — no header ⇒ current version;
  bare-major alias; unsupported ⇒ error) layered as `require_version_mw`
  in `App::after_routes`. Copied from the event-service reference
  implementation of
  [`agents/share/api-versioning.md`](../../../agents/share/api-versioning.md);
  organization's URLs were already version-free, so this rollout step
  was purely additive (the negotiation header + `406`, not a URL
  migration). Details in §9.

- [x] **Pagination on `GET /api/organizations` and `GET
  /api/organizations/search`** *(2026-08-01)*. `?limit=&offset=`, with
  `X-Total-Count`/`X-Limit`/`X-Offset` response headers per
  [`agents/share/restful.md`](../../../agents/share/restful.md);
  omitting both parameters preserves the pre-pagination behaviour
  exactly (first 100 / 50). `limit` clamps to 500; `offset` beyond
  10 000 is `400` (SEC-G7 — an unbounded offset is a cheap DoS via
  discarded row materialisation). Search's total comes from Tantivy's
  `Count` collector, not the page length. Details in §6.2/§6.11.

- [x] **`seed_examples` CLI task (EX-4)** *(2026-08-04)*.
  `cargo loco task seed_examples` loads the repo's shared demo fixture
  (`examples/data/organizations.jsonl`, 20 rows) via the model-layer
  create (no duplicate check, no audit row, no event — deliberate for a
  seed task); refuses to insert into a non-empty `organizations` table.
  New `src/tasks/seed_examples.rs`; DB-free fixture-parse tests +
  `tests/seed_examples_db.rs` (first run seeds all 20, second run is a
  no-op). Details in §11.

- [x] **FE-4 — `/review` upgraded to the person T-25 standard**
  *(2026-08-04)*. The front-end's board was unspecified and untested,
  same starting state as person's before its own T-25 completion (root
  `tasks.md` FE-4). Brought to parity: `?status=`/`?limit=` filters on
  `GET /review-queue` (no `offset`; "all" is the *absence* of `status`,
  confirmed against this crate's own handler — an unrecognised token,
  including the literal `"all"`, is `422`, not the person-service's
  `INVALID_STATUS` code but the same shape); a keyboard-reachable queue
  `<table>` with real `Compare`/`Confirm`/`Reject` buttons alongside the
  existing SVAR Kanban drag-to-decide; an inline (non-modal) side-by-side
  comparison panel over two parallel `GET /api/organizations/{pid}`
  calls; `provenance` surfaced on both the Kanban card description and
  the table. One deliberate deviation from the person pattern, forced by
  this crate's own wire shape: `GET /review-queue`'s `ReviewQueueItem`
  never serializes `score_breakdown` (the column exists in
  `review_queue` — `models/review_queue.rs` — but
  `controllers/organizations.rs::review_row_to_item` omits the field, so
  the stored scan never reaches the browser). Rather than a
  permanently-empty breakdown table, the front-end's comparison panel
  calls the existing `POST /api/organizations/match` endpoint against
  the loaded pair for a **live** `MatchBreakdown` — a legitimate reuse of
  an already-shipped, no-persistence endpoint, not a new one. The
  front-end's `ReviewQueueItem` TypeScript type was also missing
  `provenance` entirely (a pre-existing gap from when BLK-5 added the
  column server-side); added. `/merge` now reads `?main=&duplicate=`
  via `$app/state`'s `page.url.searchParams` to pre-fill both id
  fields, so `$lib/review`'s `mergeHref` deep-link (shown once an item
  is `confirmed`, in either survivor order) actually lands filled in.
  New `src/lib/review.ts` (pure: `REVIEW_STATUSES`/`REVIEW_LIMITS`,
  `isReviewStatus`, `canDecide`, `MATCH_COMPONENTS` — this matcher's six
  weighted components, name/address/url/jurisdiction/founding-date/
  keywords, summing to 1.00 per
  `organization-matcher-rust-crate/src/config.rs`'s
  `MatchConfig::default` — `breakdownRows`, `mergeHref`), 15 new unit
  tests. 57 new i18n keys × 13 locales (parity-tested). `pnpm check`
  (0/0), `pnpm test` (72, up from 54), `pnpm build`, and
  `pnpm test:e2e` (10/10, up from 7 — three new: the keyboard table, the
  live-breakdown comparison, the merge query-string prefill) all green.

- [x] **Real OpenTelemetry OTLP export (PRO-H12 slice 4 of 7)**
  *(2026-08-30)*. New `src/observability.rs` — this crate carried no
  observability module at all before this change, and is the first of
  the four remaining loco-idiomatic registries (organization,
  care-pathway, case, portfolio) to carry it; PRO-H12 slices 1–3
  (course, place, thing) were all person-style crates. Close port of
  course-service's, itself a port of person-service's, itself a port of
  link-graph-service's original reference: a `tracing-opentelemetry`
  bridge over an OTLP/gRPC `SdkTracerProvider`/`SdkMeterProvider`,
  export-on-by-default at `OTLP_ENDPOINT` (default
  `http://localhost:4317`) with `service.name` from `OTLP_SERVICE_NAME`
  (default `organization-service`) — both variables deliberately
  unprefixed, matching every other crate carrying this pipeline.
  `App::init_logger` installs it; `App::on_shutdown` flushes it.
  Real adaptation, confirmed rather than assumed: this crate has
  exactly **one** router-construction surface
  (`App::routes`/`App::after_routes`; even its own request-level test
  suite boots the real `App` via loco's testing harness rather than a
  second hand-rolled router), unlike the person-style crates' two, so
  `observability::trace_mw` is layered once, as the outermost
  middleware in `after_routes`. No `tonic` dev-dependency rename was
  needed: this crate declares no `tonic` dependency at all (no gRPC
  stub — `agents/share/overview.md`'s capability matrix), so the
  in-process OTLP collector tests take a plain `tonic = "0.14"`
  dev-dependency, exactly as course's did. `tests/otlp_export.rs` +
  `tests/otlp_middleware.rs` + `tests/otlp_collector/` (ported from
  course-service) prove real export against a real in-process gRPC
  listener in a normal `cargo test` run — no external collector,
  nothing `#[ignore]`d, no database. **Acceptance:** `cargo fmt
  --check`, `cargo clippy --all-targets -- -D warnings`, `cargo deny
  check`, `cargo bench --no-run`, and the MSRV check (`cargo +1.96
  check --all-targets`) all clean; `cargo test --lib` 198/198 (was
  190, +8 new observability unit tests); `cargo test --test
  otlp_export --test otlp_middleware` 4/4 (real protobuf crossing a
  real in-process gRPC socket).

- [ ] **ORG-T1 (S) Persist `score_breakdown` on stored review-queue rows.**
  `controllers/organizations.rs::review_row_to_item` hard-codes
  `score_breakdown: None` when mapping a stored `review_queue` row to
  the wire `ReviewQueueItem`, even though the column exists in
  `models/review_queue.rs` and IS written on insert
  (`deduplicate`/`check_duplicates`'s persist path). The front-end's
  FE-4 completion papered over this by calling `POST
  /api/organizations/match` a second time against the loaded pair for a
  live breakdown — a real workaround for a real gap, not a design
  choice. *(Verified: `grep -n score_breakdown
  src/controllers/organizations.rs` shows the field hard-coded `None` at
  the `review_row_to_item` call site while `models/review_queue.rs`
  both stores and reads it back.)* This is a three-part change (spec +
  code + test) per the family discipline. **Acceptance:** `GET
  /review-queue` returns the stored `score_breakdown` for a row that has
  one; a DB-gated test scans, re-fetches the queue, and asserts the
  breakdown round-trips without a second `/match` call.

- [ ] **ORG-T2 (M) Extend the ABAC `mask` obligation to `list`, `search`,
  `check-duplicates`, and `deduplicate`.** Only the single-record `GET
  /{pid}` and `GET /{pid}/export` handlers call
  `crate::auth::authorize_record`/honour the `mask` obligation; `list`,
  `search`, `check_duplicates`, and `deduplicate` return the raw stored
  record with no masking pass at all. This violates
  `agents/share/security.md` invariant 5 ("masking on every read
  path... list / search / check-duplicates... must never reveal more
  than the equivalent single read") and is the same SEC-G3-shaped gap
  the family tracks as partial for person. *(Verified:
  `grep -n "async fn list\|async fn search\|async fn check_duplicates\|async fn deduplicate\|authorize_record"
  src/controllers/organizations.rs` shows `authorize_record` called only
  at the `GET /{pid}` (line 159) and export (line 218) sites.)* Three-part
  change. **Acceptance:** a DB-gated test with a `mask`-obligation policy
  proves a `list`/`search`/`check-duplicates` response redacts the same
  fields `GET /{pid}/masked` does; `deduplicate` review-queue entries
  likewise carry masked names/identifiers when the caller is
  mask-obligated.

- [ ] **ORG-T3 (M) Real-time duplicate check on create (`409`).**
  `POST /api/organizations` has no duplicate short-circuit at all — the
  handler validates and inserts unconditionally, leaving
  `check-duplicates`/`deduplicate` as separate, opt-in calls a caller
  can simply never make. This is already named in §15 Roadmap and §16
  Open questions but has no closable §13 task with acceptance criteria.
  *(Verified: `grep -n "StatusCode::CONFLICT\|409"
  src/controllers/organizations.rs` returns no hits — no `create` path
  ever returns `409`.)* Three-part change (spec — resolve the §16 open
  question — + code + test), matching the family's real-time-create
  duplicate-detect flow (`agents/share/dataflow.md`). **Acceptance:** a
  DB-gated test creating a record that duplicates an existing one gets
  `409` with candidate matches in the body; creating a genuinely new
  record still succeeds; the existing `check-duplicates`/`deduplicate`
  behaviour is unchanged.

- [ ] **ORG-T4 (S) URL well-formedness + ISO 3166 country-code validation.**
  `src/validation.rs` only length-bounds `url`, `address.country`, and
  `same_as[i]` (`check_opt_text`/`string_array_caps`) — it never checks
  they parse as a URL or a real ISO 3166-1 alpha-2/3 code. The crate's
  own `[~]` task entry above already names this as deferred ("Still
  open: URL well-formedness and country-code (ISO 3166) validation");
  this promotes it to a closable item. *(Verified: `grep -n
  "country\|url" src/validation.rs` shows only `check_opt_text`
  length-only calls, no format check.)* Three-part change.
  **Acceptance:** an `422` with a field-scoped reason for a malformed
  `url` or an unrecognised `address.country`/`jurisdiction` code;
  existing valid records are unaffected; unit tests for the new
  validators plus a request-level `422` pin.

- [ ] **ORG-T5 (S) Wire FHIR `GET /fhir/Organization` search onto the
  Tantivy index.** The FHIR search handler
  (`controllers/fhir.rs::search`) still runs a capped Postgres scan via
  `OrganizationModel::list`, exactly as the Tantivy roll-out task above
  already calls out ("Not wired to Tantivy: the FHIR `GET
  /fhir/Organization` search... Moving it is a separate item, not a
  side effect of this one"). *(Verified: `grep -n "async fn search"
  src/controllers/fhir.rs` shows the handler calling the model list
  path, not `crate::search::SearchEngine`.)* Three-part change.
  **Acceptance:** `GET /fhir/Organization?name=` and `?address-city=`
  resolve through the Tantivy engine (fuzzy/phonetic where the query
  param allows it) instead of the scan cap; the `CapabilityStatement`'s
  declared search params are unchanged; existing FHIR search tests stay
  green plus a new test proving a hit beyond the old scan cap is found.

## 14. Implementation status

Done: loco boot; organizations table + migration; CRUD (blank name →
`422`, unknown pid → `404`) with `?limit=&offset=` pagination
(`X-Total-Count`/`X-Limit`/`X-Offset`, SEC-G7-bounded); `/match` and
`/check-duplicates` embedding organization-matcher, blocked on the
search index; audit log; in-memory event streaming (Phase 1: canonical
`Envelope` + `EventPublisher` seam, `EventView` projection frozen for
`/events/recent`) plus the durable outbox (Phase 2, transactional,
default-off) and its Phase-3 relay + retention (`src/relay.rs`,
default-off) with a real-broker `FluvioSink` (BUS-3, feature-gated, off
by default); Tantivy full-text search (fuzzy + phonetic + duplicate
blocking, replacing the earlier `ILIKE` name search); batch dedup +
a stored `review_queue` with confirm/reject decision endpoints; record
merge (`/merge` + `merge_records` history); per-field masking + the
masked view + the audited GDPR right-of-access export, wired to the
ABAC `mask` obligation (no consent model — an organization is not a
data subject); input-size caps (SEC-M1) and deterministic-identifier
check-digit validation (SEC-M5); offline **PASETO v4.public**
verification (`AuthUser`/`MaybeAuthUser`, `/whoami`, audit + merge
`actor` from the token) per
[`authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
— originally shipped against RS256-JWT/JWKS, since switched — plus
ABAC policy authorization inside the blanket guard, and **AU-2**: both
the verifier and the policy are hot-reloadable without a restart
(periodic PASETO-key refresh + a policy-file mtime watcher); OpenAPI 3
+ Swagger UI; Prometheus metrics (`/metrics.prom`, root + public,
CRUD/merge counters); a **FHIR R5 API** for `Organization` — the
**family reference implementation** (read/create/update/delete/search +
`CapabilityStatement`, behind the same auth+ABAC guard); header-based
API versioning (`Accepts-version`, copied from the event-service
reference); BLK-5 async bulk import/export (JSONL + CSV,
local-filesystem artifact store; keyless rows route to the review
queue with `provenance = "import"`); the `seed_examples` CLI task
(EX-4); DB-free tests; request-level test suite plus five dedicated
Postgres-gated test binaries (`enforcement`, `masking`, `outbox_audit`,
`seed_examples_db`, `fluvio_relay`; §11); loco scaffolding leftovers
removed (no workers/tasks/data stubs); real **OpenTelemetry OTLP
export** (`src/observability.rs`, PRO-H12 slice 4 of 7 — structured
logging plus a `tracing-opentelemetry` bridge over an OTLP/gRPC
exporter, `trace_mw` layered on this crate's one router-construction
surface); green build + clippy.

Still open (§13): richer validation beyond identifier check-digits (URL
well-formedness, ISO country codes); real-time duplicate check on
create (`409`); an `S3` `ArtifactStore` backend for BLK-5; a
`ConnectionTrait`-generic `streaming::create_and_emit`/`update_and_emit`
so the BLK-5 per-row upsert can be advisory-lock-protected (SEC-B3)
without a pool deadlock; moving the FHIR structured search onto the
Tantivy index; cross-service link **target** readiness confirmation
(§8, §13).

## 15. Roadmap

v0.1 (here): CRUD + matching MVP. v0.2: search + audit + streaming. v0.3:
privacy + merge + OpenAPI + auth middleware (PASETO v4.public per
[`authentication-sessions.md`](../../../agents/share/authentication-sessions.md),
superseding the RS256-JWT model). v0.4 (delivered, §13/§14): FHIR R5,
header-based API versioning, ABAC policy authorization, BLK-5 bulk
import/export, key rotation + policy hot-reload (AU-2).

## 16. Open questions

- Should identifiers/address be normalised into their own tables (vs the
  single JSONB payload) once search lands? Tantivy search has since
  landed (§13) without normalising — this question is unresolved, not
  answered by that landing.
- Real-time duplicate check on create (409) vs the explicit endpoint?
  Still just the explicit `check-duplicates`/`import` paths; `POST
  /api/organizations` never blocks on a match today.
- ~~Periodic re-fetch of the PASETO key set (key rotation)~~ —
  **RESOLVED by AU-2** (§13, §7): `auth::spawn_key_refresh` re-fetches
  `ORGANIZATION_PASETO_KEYS_URL` on `ORGANIZATION_PASETO_KEYS_REFRESH_SECS`
  (default hourly), and the ABAC policy file is likewise watched and
  hot-reloaded.

## 17. References

- schema.org/Organization; loco.rs; the organization-matcher spec.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
