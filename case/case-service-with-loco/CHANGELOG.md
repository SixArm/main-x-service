# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Changed

- **Auth pivot — docs only (code follow-up pending).** The family
  authentication model moved from **RS256 JWT + JWKS** to **server-side
  cookie sessions + offline PASETO v4.public verification** (published
  Ed25519 key replacing the JWKS) — see
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  as the source of truth; RS256/JWKS are decommissioned. Human-facing
  docs (README / AGENTS / index) now describe PASETO v4.public offline
  verification and "blanket auth enforcement"; the `CASE_REQUIRE_AUTH`
  flag and enforcement semantics are unchanged — only the credential
  checked changes. The runtime `src/auth.rs` still verifies the old
  credential; the PASETO code follow-up (verifier swap, published-key
  fetch) is tracked in [spec §13](./spec/index.md). No code change in
  this entry.
- **Documentation harmonization pass.** Expanded `index.md`'s "Worked
  flow" to the full v0.1 surface (list / search / update / delete /
  merge / merges-recent / whoami / audit / events / OpenAPI+Swagger /
  metrics — previously only create / read / dedupe / match), and added a
  worked **merge** request/response example (`{main_pid, duplicate_pid,
  reason?}` → `{main_pid, duplicate_pid, main}`) with its `422` / `404`
  cases and the two-audit-row note (`merged` on the survivor,
  `merged_into` on the duplicate). Removed a duplicate `- main` entry in
  the CI workflow's `push.branches` list. No behavioural change.

### Added

- **Prometheus metrics** at `GET /metrics.prom` (parity with the older
  Axum services). New `src/metrics.rs` owns a process-wide
  `OnceLock<Metrics>` (`Metrics::global()`) holding a `prometheus::Registry`
  with four CRUD counters — `case_created_total`, `case_updated_total`,
  `case_deleted_total`, `case_merged_total` — plus an `http_requests_total`
  `IntCounterVec` labeled by `method`/`path`/`status`. `Metrics::render()`
  encodes the registry to Prometheus text-exposition format
  (`text/plain; version=0.0.4`). A new root-mounted loco route
  (`controllers/metrics.rs`, registered in `app.rs` alongside the docs
  routes — **not** under `/api`) serves it with that content type. The path
  is added to `auth::is_public_path`, so it stays public even under blanket
  JWT enforcement. The cases controller increments the matching counter on
  each create / update / delete / merge success path. The OpenAPI document
  (`src/openapi.rs`) gains a `/metrics.prom` entry under an `observability`
  tag. Un-gated unit tests pin: `render()` yields valid Prometheus text
  (HELP/TYPE lines + a non-zero sample + the label vec), the content-type
  constant, the new `enforce` public-path case, and the OpenAPI entry.
- **Durable event bus — Phase 1** (canonical envelope + publisher seam,
  per [`agents/share/event-bus.md`](../../agents/share/event-bus.md)
  §4–§5). `src/streaming.rs` now models a versioned `Envelope`
  (`event_id: Uuid` dedup key, `schema_version` const `1`, `entity`
  `"case"`, `kind`, `pid`, `seq`, `actor: Option<String>`, `name`) and a
  flat `EventView { kind, pid, name, seq }` projection, with
  `From<&Envelope>`. The free functions are now a thin
  `EventPublisher` trait (`publish` / `recent`) with an
  `InMemoryPublisher` ring buffer as the process-wide global. A new
  `publish_with_actor(kind, pid, name, actor)` records the verified
  caller `sub`; the CRUD/merge handlers pass the `actor` they already
  extract from `MaybeAuthUser`. `occurred_at` and the full-record `data`
  snapshot are deferred to the Phase 2 outbox (no new dependency added).
  Pure refactor: behaviour identical and the `GET /api/cases/events/recent`
  wire shape (`{kind, pid, name, seq}`) is unchanged. Un-gated unit tests
  cover envelope serde round-trip + `schema_version == 1`, the projection's
  exact keys, `InMemoryPublisher` publish→recent, actor populated/None,
  and seq monotonicity. Phases 2–3 (transactional outbox → Fluvio) remain
  infra-gated roadmap.
- **Blanket JWT enforcement** (family contract
  [`agents/share/jwt-enforcement.md`](../../agents/share/jwt-enforcement.md)),
  **off by default**. A new env flag `CASE_REQUIRE_AUTH`
  (`1`/`true`/`yes`/`on` ⇒ on; unset/blank/other ⇒ off) gates an Axum
  `from_fn` middleware wired in `App::after_routes`: when on, every
  non-public request without a valid bearer token is rejected with `401`;
  `/_health`, `/_ping`, `/api-docs/openapi.json` and `/swagger-ui*` stay
  public. The flag is read once per process. Case data is personal data,
  so this gate is the access-control boundary in front of the case API.
  New `src/auth.rs` surface: pure `parse_bool`, `require_auth`,
  `is_public_path`, and a unit-testable `enforce(require_auth, path,
  headers, verifier)`. Un-gated unit tests pin the decision (off/no-token,
  on/public, on/protected/no-token, on/valid, on/expired, on/tampered,
  plus `parse_bool`); a DB-gated `#[serial]` request test asserts un-authed
  `GET /api/cases` ⇒ `401` while `GET /api-docs/openapi.json` ⇒ `200`.
  Activation (setting the flag) and JWKS-over-HTTP fetch remain
  operational follow-ups.

## [0.1.0] - 2026-06-13

Inaugural release. A loco.rs governmental **case** registry, copy-adapted
from the proven `care-pathway-service` with the domain swapped from care
pathway to case.

### Added

- **`cases` table** (`pid`, denormalised `title`, full `Case` payload as
  JSONB `data`, `active`, soft-delete) + `audit_logs` + `merge_records`,
  via `sea-orm-migration`.
- **Embeds `case-matcher` directly**: the API DTO *is*
  `case_matcher::Case`, stored verbatim and matched with the canonical
  engine — no separate model or adapter.
- **CRUD controller** (`/api/cases`): create / list / get / update /
  soft-delete, plus `GET /search?q=` (Postgres `ILIKE` on `title`),
  `POST /match`, `POST /check-duplicates`, `POST /merge`,
  `GET /merges/recent`.
- **Validation → `422`** (family convention): blank `title`, malformed
  `opened_date` (ISO-8601 `YYYY` / `YYYY-MM-DD`), blank identifier value,
  blank `subjects` / `keywords` entries; one response lists every
  problem (`src/validation.rs`).
- **Record merge** (`src/merge.rs` + `models/merge_records.rs`): union
  list fields, keep main's scalars (fall back to the duplicate's), add
  the duplicate's title as a former `alternate_titles` entry; `422` on
  self-merge, `404` on unknown pid.
- **Audit log + in-memory event stream** on every CRUD/merge
  (`models/audit_logs.rs`, `src/streaming.rs`; `created` / `updated` /
  `deleted` / `merged`), with audit / event query endpoints.
- **Offline RS256 JWT verification** (`src/auth.rs`, embeds
  `authentication-verifier`): `GET /whoami` proves end-to-end JWKS
  verification; CRUD/merge stamp the audit + merge `actor` from the
  verified caller. Env: `CASE_JWKS`, `CASE_JWT_ISSUER`,
  `CASE_JWT_AUDIENCE`.
- **OpenAPI 3 + Swagger UI** (`src/openapi.rs`, `controllers/docs.rs`):
  `/api-docs/openapi.json` + `/swagger-ui`.
- **Tests.** DB-free unit tests (validation, merge, auth crypto, openapi,
  streaming, `escape_like`) + `tests/matching.rs` (matcher embedding +
  JSON round-trip) run on `cargo test`. Request-level integration tests
  (`tests/requests/cases.rs`, loco testing harness) cover every endpoint;
  `#[ignore]`-gated on a PostgreSQL `DATABASE_URL` (`cargo test -- --ignored`).

### Notes

- MVP scope is CRUD + `ILIKE` title search + matching. Tantivy full-text
  search, search-blocked dedup candidates, durable event bus, privacy,
  and blanket `/api/*` JWT enforcement are tracked in spec §13.
