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

MVP: CRUD + matching. Out of scope for the MVP (deferred, §13): full-text
search, streaming, audit, privacy/GDPR export, OpenAPI, gRPC, rich
validation. Authentication is out of scope here — provided by the
central authentication-service.

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
2. `GET /api/organizations` — list active (cap 100), `{pid, name}`.
3. `GET /api/organizations/{pid}` — return the stored `Organization`.
4. `PUT /api/organizations/{pid}` — replace the payload; `name`
   required (422 if blank).
5. `DELETE /api/organizations/{pid}` — soft-delete (`active=false`,
   `deleted_at` stamped).
6. `POST /api/organizations/match` — rank an explicit `{query,
   candidates}` set (no persistence).
7. `POST /api/organizations/check-duplicates` — match a query against
   stored organizations; return the ones above threshold, ranked.
8. `GET /metrics.prom` — Prometheus metrics in text-exposition format
   (`text/plain; version=0.0.4`). Mounted at the application **root**
   (not under `/api`), public even under blanket auth enforcement.

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic matching via the
embedded library; soft-delete with audit-friendly timestamps.

### Configuration environment variables

| Variable | Default | Purpose |
|---|---|---|
| `ORGANIZATION_PASETO_KEYS` | empty key set | Published Ed25519 public-key set (`paseto-keys` JSON) for offline PASETO v4.public token verification (`src/auth.rs`). |
| `ORGANIZATION_PASETO_KEYS_URL` | unset ⇒ no fetch | When set, fetch the key set over HTTP **once at boot** (`Verifier::from_paseto_keys_url`, typically the auth-service `/.well-known/paseto-keys`; seeded from `App::after_routes` via `auth::init_from_env`). Success ⇒ the fetched set wins over `ORGANIZATION_PASETO_KEYS`; failure ⇒ warn + fall back to the env key set — the service always boots. No refresh loop (periodic re-fetch on key rotation is a future item, §16). |
| `ORGANIZATION_TOKEN_ISSUER` | `authentication-service` | Expected `iss` (see [`authentication-sessions.md`](../../../agents/share/authentication-sessions.md) §5 claims). |
| `ORGANIZATION_TOKEN_AUDIENCE` | `main-x-service` | Expected `aud` (see [`authentication-sessions.md`](../../../agents/share/authentication-sessions.md) §5 claims). |
| `ORGANIZATION_REQUIRE_AUTH` | unset ⇒ **off** | Blanket `/api/*` enforcement (credential is now a PASETO v4.public token or BFF cookie session). Lenient bool: `1`/`true`/`yes`/`on` ⇒ on; else off. See [`agents/share/jwt-enforcement.md`](../../../agents/share/jwt-enforcement.md) (credential superseded by [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)). |
| `ORGANIZATION_ABAC_POLICY` | unset ⇒ built-in default policy | ABAC authorization policy as inline JSON (evaluated only when enforcement is on). Unparsable ⇒ warn-log + built-in default. See [`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md). |
| `ORGANIZATION_ABAC_POLICY_FILE` | unset | Path to the ABAC policy JSON file (used when `ORGANIZATION_ABAC_POLICY` is unset). Unreadable/unparsable ⇒ warn-log + built-in default. |

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

**Bulk import / export (roadmap, §13).** The family-wide contract — async
`bg_pg` jobs, the `bulk_jobs` table, the five `/api/v1/organizations/{import,export,bulk-jobs}`
endpoints, JSONL/CSV/Parquet, idempotent upsert-by-key, the per-row error
report, and the export privacy/audit posture — is fixed in
[`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md).
Organization declares only the differences (umbrella spec §8.7): **stable
key** = a deterministic globally-unique scheme-scoped identifier the matcher
short-circuits on (`Lei`/`Duns`/`Iso6523`/`Gln`/`Wikidata`/`Ror`/`Isni`/`Vat`,
matched as `(scheme, value)`) or the record `pid`; **CSV** flattens `address.*`
to dotted columns and JSON-encodes the `identifiers` / `alternate_names` /
`same_as` / `keywords` / `tags` / `relationships` arrays, with JSONL the
lossless reference; **export sensitivity** is low–medium (light default
masking protecting `telephone` / `email` + sole-trader records, parity with
the §13 masking task), every export audited.

## 9. API surface

See §6. Responses are raw loco JSON. `404` for unknown `pid`; `422
Unprocessable Entity` for validation failures (blank `name` on create
or replace — family convention); `400` for malformed requests (blank
search `q`, invalid audit pid).

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
`/deduplicate`, `/import`; the latter two ahead of the dedup-scan and
bulk-import features), and the shared engine in
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
blank-name `422` (create + update), unknown-pid `404`, search,
check-duplicates, merge, `whoami` `401`, the blanket-enforcement
gate (with `ORGANIZATION_REQUIRE_AUTH=1` set in-test, un-authed `GET
/api/organizations` ⇒ `401` while `GET /api-docs/openapi.json` ⇒
`200`; `#[serial]` for env-var ordering), the audit endpoints
(`/audit/recent` + `/{pid}/audit` record CRUD actions; invalid pid ⇒
`400`), and the plain-CRUD `created`/`updated`/`deleted` events on
`/events/recent`. These require Postgres
(`config/test.yaml`) and are `#[ignore]`d so the default `cargo test`
stays green — run with `cargo test -- --ignored`.

## 12. Compliance

Organization data is largely public, but contact fields may be
personal data — honour GDPR when the privacy layer lands (§13).

## 13. Tasks (live work queue)

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
  Phases 2–3 (transactional outbox + Fluvio relay) remain infra-gated
  roadmap.
- [x] Name search (Postgres `ILIKE`) + OpenAPI/Swagger.
- [x] Prometheus metrics — `GET /metrics.prom` (root, public) serves a
  process-wide `prometheus::Registry` (`src/metrics.rs`, `OnceLock`)
  in text-exposition format; CRUD/merge handlers increment
  `organization_{created,updated,deleted,merged}_total`; a labelled
  `http_requests_total` is declared for a future request middleware.
  Brings parity with the older Axum services. DB-free render test +
  OpenAPI path test; `/metrics.prom` added to `auth::is_public_path`.
- [ ] Tantivy full-text search + fuzzy/blocking (replacing the `ILIKE`
      search).
- [ ] Per-field masking + GDPR export endpoint.
- [x] Record merge — `POST /merge` folds a duplicate into a survivor
  (union fields, former-name alias, soft-delete, `merge_records` history
  + snapshot, `Merged` event); pure `src/merge.rs`; `/merges/recent`.
- [ ] Richer validation (identifier formats, URL, country codes).
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
    Unset/blank URL ⇒ prior behaviour exactly. Fetch-once only; a
    periodic refresh loop on key rotation stays future work (§16).
    Tests: local ephemeral-port HTTP listener serving the test key set
    (fetched verifier accepts a token signed by that key) + fast-failing
    URL fallback (no panic) + no-URL env path.
- [ ] Bulk import / export — adopt the family contract
  ([`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md)):
  `bulk_jobs` migration, the five `/api/v1/organizations/{import,export,bulk-jobs}`
  endpoints, a `bg_pg` worker, JSONL/CSV/Parquet codecs, a per-row pipeline
  reusing the single-create validators + organization-matcher + review queue
  (`provenance = import`; upsert by deterministic scheme-scoped identifier or
  `pid`), the per-row error report, and export masking + audit (light default
  masking, gated `include_soft_deleted`). Organization-specific declarations
  (stable key, CSV column set, sensitivity) are umbrella spec §8.7; mirrors
  umbrella spec §13 T-14. Tests: idempotent re-import, error report,
  dedupe-to-review, masked vs full export, export audit.

## 14. Implementation status

Done: loco boot; organizations table + migration; CRUD (blank name →
`422`, unknown pid → `404`); `/match` and `/check-duplicates` embedding
organization-matcher; audit log; in-memory event streaming (Phase 1:
canonical `Envelope` + `EventPublisher` seam, `EventView` projection
frozen for `/events/recent`); name search (`ILIKE`); record merge
(`/merge` + `merge_records` history); offline
**PASETO v4.public** verification (`AuthUser`/`MaybeAuthUser`, `/whoami`,
audit + merge `actor` from the token) per
[`authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
— originally shipped against RS256-JWT/JWKS, since switched (§13) —
including the boot-time paseto-keys-over-HTTP fetch
(`ORGANIZATION_PASETO_KEYS_URL`, fetch-once, env fallback; §7, §13);
OpenAPI 3 + Swagger UI; Prometheus
metrics (`/metrics.prom`, root + public, CRUD/merge counters); DB-free
tests;
request-level test suite (Postgres, `#[ignore]`-gated); loco scaffolding
leftovers removed (no workers/tasks/data stubs); green build + clippy.

## 15. Roadmap

v0.1 (here): CRUD + matching MVP. v0.2: search + audit + streaming.
v0.3: privacy + merge + OpenAPI + auth middleware (PASETO v4.public per
[`authentication-sessions.md`](../../../agents/share/authentication-sessions.md),
superseding the RS256-JWT model).

## 16. Open questions

- Should identifiers/address be normalised into their own tables (vs the
  single JSONB payload) once search lands?
- Real-time duplicate check on create (409) vs the explicit endpoint?
- Periodic re-fetch of the PASETO key set (key rotation) — the boot
  fetch (§7 `ORGANIZATION_PASETO_KEYS_URL`) runs once; is a refresh
  loop (or refetch-on-`UnknownKid`) needed before rotation goes live?

## 17. References

- schema.org/Organization; loco.rs; the organization-matcher spec.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
