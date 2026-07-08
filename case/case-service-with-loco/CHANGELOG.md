# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added — authz: ABAC policy authorization inside the blanket guard (2026-07-05)

- ABAC authorization landed (supersedes the earlier per-crate
  roles/RBAC sketch; family contract:
  `agents/share/authorization-attributes.md`). When
  `CASE_REQUIRE_AUTH` is on, a verified PASETO token is further
  checked by the shared policy engine in `authentication-verifier`
  0.3: the request's action is derived from the HTTP method plus the
  crate's destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES`
  — `/merge`, `/deduplicate`, `/import`), and the policy is evaluated
  over the token's new `attrs` claim, first-match-wins, defaulting to
  allow-read / deny-mutation.
- New env vars `CASE_ABAC_POLICY` (inline JSON) and
  `CASE_ABAC_POLICY_FILE` (path); unset or unparsable ⇒
  `tracing::warn!` + the built-in default policy (`svc=true` ⇒
  everything; `access=admin` ⇒ destructive+write; `access=write` ⇒
  write) — the service always boots. Because case data is personal
  data, deployments can express department / purpose-of-use scoping
  as configured policy rules over the same `attrs` claim —
  configuration, not code.
- `auth::enforce` now takes the HTTP method and the policy and returns
  `403` (deciding-rule reason) for a valid token the policy denies;
  `401` remains missing/bad credential. DB-free unit tests pin the
  family §7 matrix. Flag off ⇒ behaviour-neutral.

### Added — test/ci: DB-backed enforcement "activation proof" (2026-07-06)

- New `tests/enforcement.rs` (its own binary, so the enforcement-on
  `OnceLock`s are isolated from the enforcement-off request suite) boots
  the real router with `CASE_REQUIRE_AUTH=1` and mints in-process
  PASETO v4.public tokens (throwaway Ed25519 key) to pin the full matrix
  over the HTTP stack against Postgres: public path open, protected path
  `401` without a token, `403` for a write without `access=write`
  (default deny-mutation), `200` for a read (default allow-read) and for
  a write with `access=write`. `#[ignore]`d (needs a database).
- CI now runs the DB-gated suites: the test step uses
  `cargo test --all-features --all -- --include-ignored` (previously
  `cargo test` silently skipped every `#[ignore]`d request/enforcement
  test, so they never actually ran). The case service is the family
  reference for this pattern; the activation playbook is in
  `agents/share/jwt-enforcement.md`.

### Added — auth: key-rotation refresh loop (2026-07-05)

- The PASETO key set is now **re-fetched periodically** (verifier 0.8
  `ReloadableVerifier`), so a key rotation at the auth-service is picked
  up **without restarting** this service. `auth::verifier()` is now a
  reloadable holder (the guard and extractors read `current()` per
  request); `auth::spawn_key_refresh` (spawned in
  `app.rs::after_routes`) polls `CASE_PASETO_KEYS_URL` every
  `CASE_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables) and swaps
  in the new key set. A failed fetch keeps the current keys (a transient
  auth-service outage never locks callers out). A no-op when
  `CASE_PASETO_KEYS_URL` is unset. Family reference for the pattern.

### Added — authz: hot-reloadable ABAC policy (2026-07-05)

- The ABAC policy is now **hot-reloadable** (verifier 0.7
  `ReloadablePolicy`). `auth::policy()` returns the reloadable holder;
  the guard and `authorize_record` read `policy().current()` per
  request. `auth::reload_policy()` re-reads `CASE_ABAC_POLICY` /
  `CASE_ABAC_POLICY_FILE` and swaps the live policy (malformed ⇒ the
  built-in default, never unprotected). `auth::spawn_policy_watcher`
  (spawned in `app.rs::after_routes`) polls `CASE_ABAC_POLICY_FILE`'s
  mtime every 15 s and reloads on change — operators can edit the
  policy file with **no restart**. A no-op when the file var is unset.
  The case service is the family reference for this pattern.

### Added — authz: record-level resource attributes (2026-07-05)

- Record-level ABAC (this crate is the family reference for
  `authorization-attributes.md` §9). The single-case handlers
  `GET`/`PUT`/`DELETE /api/cases/{pid}` run a second, finer decision
  after loading the record: `auth::case_resource_attrs` derives the
  case's classification into `resource.case_type` / `resource.status` /
  `resource.priority` tokens, and `auth::authorize_record` calls the
  new `authentication-verifier` 0.4
  `Policy::evaluate_with_resource` (path dep bumped 0.3 → 0.4). Gated
  on `CASE_REQUIRE_AUTH`, so a no-op when enforcement is off.
- Deployments can now express, as policy, e.g. "deny write when
  `resource.status=closed` unless `access=admin`" or "deny read on
  `resource.case_type=investigation` unless `dept=investigations`".
  `PUT`/`DELETE` evaluate the **stored** case's attributes (the record
  being modified). No schema change — these are existing fields; a
  per-case sensitivity column stays an optional roadmap add.
- `MaybeAuthUser` gains `claims()`. `GET /api/cases/{pid}` now takes
  `MaybeAuthUser` so a read can be record-gated. DB-free unit tests:
  the resource-attribute mapping (incl. `Custom` lowercasing and absent
  fields) and an end-to-end policy decision (writer denied on a closed
  case, allowed on an open one, admin overrides).
- **Environment attributes** (verifier 0.4 → 0.5). The record-level
  pass now also supplies request context via
  `Policy::evaluate_with_context`: `auth::request_env_attrs` derives
  `env.hour` / `env.after_hours` (UTC) at the service edge (the engine
  stays deterministic), so a deployment can add e.g. "deny write when
  `env.after_hours=true` unless `access=admin`". Verifier 0.5 also adds
  `$sub`/`$email` value templates for ownership rules
  (`resource.owner: ["$sub"]`). DB-free test for the working-hours
  derivation.
- **Mask-on-allow obligation** (verifier 0.5 → 0.6). `authorize_record`
  now returns the decision's **obligations**, and `GET /api/cases/{pid}`
  honours a `mask` obligation by returning a **redacted** case
  (`mask_case` drops `subjects` / `identifiers` / `same_as` / case
  number, keeping the descriptive shell). A policy can thus attach
  `"obligations": ["mask"]` to a conditional read (e.g. cross-department
  access), turning ABAC into the driver for the case service's masking.
  DB-free test for the redaction.

### Added

- **Boot-time paseto-keys-over-HTTP fetch** (the spec §13 follow-up, done
  2026-07-04). New optional env var `CASE_PASETO_KEYS_URL`: when set
  (non-blank), `auth::init` — called from `App::after_routes`, before the
  app serves traffic — fetches the auth-service's published Ed25519 key
  set once over HTTP via `Verifier::from_paseto_keys_url` (the
  `authentication-verifier` crate's `fetch` feature, now enabled). On
  success the fetched key set **wins** over the `CASE_PASETO_KEYS` env
  key set (`tracing::info!`); on failure the service logs a
  `tracing::warn!` and falls back to the env path, so it **always
  boots**. Unset/blank ⇒ prior behaviour unchanged (env key set, else
  empty reject-all). Fetch is once-at-boot only — no refresh loop
  (rotation-triggered refetch is tracked in spec §16). The seeding is
  idempotent (`OnceLock`), and the fetch-or-fallback helper
  (`auth::fetch_or`) is dependency-injected (URL / issuer / audience /
  fallback passed in) so tests cover it without the process global: a
  `#[tokio::test]` local ephemeral-port HTTP listener proves a token
  signed by the served key verifies via the fetch-built verifier, and a
  fast-failing URL (`http://127.0.0.1:1/`) proves fallback without
  panic. Existing env-key auth tests unchanged and green.

### Fixed

- `src/auth.rs` test-module imports had rustfmt drift (an over-long
  `rusty_paseto` `use` line) that broke the crate's `cargo fmt --check`
  gate. Reformatted with `cargo fmt`; no behavioural change, tests
  unchanged and green.

### Changed

- **Auth pivot.** The family
  authentication model moved from **RS256 JWT + JWKS** to **server-side
  cookie sessions + offline PASETO v4.public verification** (published
  Ed25519 key replacing the JWKS) — see
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  as the source of truth; RS256/JWKS are decommissioned. Human-facing
  docs (README / AGENTS / index) now describe PASETO v4.public offline
  verification and "blanket auth enforcement"; the `CASE_REQUIRE_AUTH`
  flag and enforcement semantics are unchanged — only the credential
  checked changes. The runtime `src/auth.rs` verifies PASETO v4.public
  via `authentication-verifier` (env-configured `CASE_PASETO_KEYS` /
  `CASE_TOKEN_ISSUER` / `CASE_TOKEN_AUDIENCE`); the
  paseto-keys-over-HTTP fetch follow-up is tracked in
  [spec §13](./spec/index.md).
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
  Activation (setting the flag) and paseto-keys-over-HTTP fetch remain
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
