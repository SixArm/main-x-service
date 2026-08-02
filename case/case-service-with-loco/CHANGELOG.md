# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]
### Added — Privacy: masked view + GDPR export (2026-08-02)

Repo tasks.md P-3. Narrower than P-1 (organization) and P-2
(care-pathway): case already honoured the ABAC `mask` **obligation** on
`GET /{pid}` via `mask_case`, landed earlier with the record-level ABAC
work. What was missing was the always-redacted view and the export
envelope.

- `GET /api/cases/{pid}/masked` — the always-masked view (`mask_case`),
  regardless of the caller's policy.
- `GET /api/cases/{pid}/export` — the GDPR right-of-access envelope
  (`export_case`, new, beside `mask_case` in `controllers/cases.rs`):
  `{entity, pid, exported_at, masked, record, note}`. Masked when the
  record-level ABAC decision carries the `mask` obligation.
- The export is audited via `disclosure::action::EXPORT` — a constant
  already declared in `disclosure.rs`'s action vocabulary (and covered
  by its distinctness test) but never wired to a live code path until
  now.
- Kept `mask_case`/`export_case` in `controllers/cases.rs` rather than
  extracting a `src/privacy.rs` module — matching how masking was
  already organised in this crate, unlike organization/care-pathway
  which each own a dedicated module. Family capability matrix
  (`agents/share/overview.md`) already reflected this correctly (case's
  "Privacy masking module" cell was `–` before and after; its
  "Record-level ABAC + masking obligations" cell was already `✅`).
- The end-to-end obligation proof (`tests/export_masking.rs`) needed
  its own test binary, separate from the pre-existing `tests/masking.rs`
  (the SEC-G2/G3 concealment proof): both set the process-wide
  `policy()` / `require_auth()` / `compliance::audit_reads()`
  `OnceLock`s, and adding the new test to the existing file let
  whichever test's app-boot ran first silently win the policy for both —
  the second test's env-var changes had no effect, and it failed for a
  reason that looked unrelated to the real cause. Discovered by running
  the DB-gated suite, not by inspection.
- DB-gated: `tests/export_masking.rs` (new) + 2 new tests in
  `tests/requests/cases.rs` (`masked_view_and_export_are_served`,
  `masked_view_and_export_are_404_for_unknown_pid`).
- No SOUP register change (no new dependency).
- Fixed stale spec/AGENTS prose still describing the title search as
  Postgres `ILIKE` in a couple of places missed by S-3.

### Added — Tantivy full-text/fuzzy/phonetic search (2026-08-02)

Repo tasks.md S-3: transfers the care-pathway/organization Tantivy
pattern (S-1/S-2) whole — index module, streaming seam, reindex task
with a boot rebuild, duplicate detection blocked on the index instead
of scanning a capped 1000 rows. What changed is the field set.

- `src/search/` — `CaseIndexSchema`/`CaseIndex` (`pid` STORED;
  `title`/`alternate_titles`/`title_phonetic`/`identifiers`/`keywords`/
  `subjects`/`agency_name` TEXT; `case_number`/`agency_id`/`case_type`/
  `status`/`active` STRING) and `SearchEngine` (`search`/
  `fuzzy_search`/`phonetic_search`/`search_page`/`candidates`). A
  case's defining attribute is who it is about — `subjects` (opaque
  involved-party ids) is now searchable, alongside the agency and every
  identifier scheme.
- `GET /api/cases/search?q=` is now Tantivy-backed with `?fuzzy=true`
  and `?phonetic=true`, and its `X-Total-Count` comes from Tantivy's
  `Count` collector rather than a SQL `COUNT(*)`. Replaces the Postgres
  `ILIKE` title search.
- `POST /api/cases/check-duplicates` now scores a **blocked** candidate
  set (fuzzy title, exact identifier, phonetic title — up to 200) from
  the index instead of an in-memory scan capped at 1000 rows, closing
  the scale cliff where a duplicate past row 1000 was unreachable
  however obvious it was.
- Both endpoints respond `503` (never a silent "no results") when the
  index is unavailable.
- `streaming.rs`'s `*_and_emit` seam indexes/deindexes best-effort after
  every commit, so no write path — native or FHIR — can skip it.
- `tasks/search.rs` — the `search_reindex` CLI task plus a
  rebuild-if-empty on boot, for a fresh deployment or a lost index
  volume.
- `tantivy` added to `compliance/soup.tsv` (IEC 62304 §5.3.3 SOUP
  gate) — indirect relevance: the database stays the source of truth,
  every hit resolves against it through the same record-level ABAC
  concealment as every other read path, so a stale index degrades
  retrieval rather than corrupting a record or bypassing authorization.
- `.gitignore` gains `/data/` (the index's default on-disk path) — a
  derived, rebuildable artifact that must never be committed.
- DB-gated: `search_reaches_secondary_fields_and_tolerates_typos`,
  `check_duplicates_blocks_on_identifier_alone`.

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration → 2.0,
  sea-query → 1.0. Raw `Statement` queries in `models/audit_logs.rs`,
  `src/compliance/erasure.rs`, and `tests/requests/cases.rs` move to
  `execute_raw`; a `useless_conversion` in `models/event_outbox.rs` from
  a now-redundant `.into()`.
- **loco's `ColType::PkAuto` now generates a 64-bit primary key.** The
  `cases`, `audit_logs`, and `merge_records` entities move from `i32` to
  `i64`, along with the audit hash-chain code that carries their row ids
  (`ChainBreak.id`, `Checkpoint.anchor_id`) and the compliance test
  fixtures. `event_outbox` and `entity_links` are unaffected — the
  former's migration writes raw SQL (`id SERIAL PRIMARY KEY`) rather
  than the loco schema DSL, the latter keys on a UUID.
- **sea-orm 2.0 dropped `DatabaseConnection::Disconnected`**, which
  `src/compliance/disclosure.rs`'s test used as a "would fail if
  touched" stand-in to prove the read-audit no-op path never reaches
  the database when auditing is off. Replaced with a `MockDatabase`
  carrying no queued results — added as a `mock`-feature dev-dependency
  so it never reaches a release build.
- No SOUP register change (`compliance/soup.tsv`) — `loco-rs` and
  `sea-orm` were already annotated; this bumps existing dependencies,
  it doesn't add one.
- No behavioural change; verified with the full DB-gated suite (34
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — pagination on list and search (2026-08-01)

Follows the family convention fixed in `agents/share/restful.md` and
first implemented in organization.

- **`GET /api/cases` and `GET /api/cases/search` take `?limit=` and `?offset=`**
  and report `X-Total-Count` / `X-Limit` / `X-Offset`. The body stays a
  bare array, so no existing caller changes.
- Defaults reproduce the old hard caps (100 / 50), `limit`
  clamps to 500 rather than erroring, and an `offset` past 10 000 is a
  `400` (an unbounded offset makes the database materialise and discard
  arbitrarily many rows — SEC-G7).
- The search total is a `COUNT(*)` over the same predicate rather than
  the page length: a page cannot tell a caller how much there is, which
  is the point of the header.
- **The total is the collection's, not the caller's.** This service
  conceals individual records from callers the record-level policy
  denies (§10); deriving the total *after* concealment would leak
  exactly what concealment hides — how many records a caller is not
  allowed to see. `X-Total-Count` therefore describes the query, and a
  caller may legitimately receive fewer rows than it suggests.
- Pinned by a DB-gated request test walking a window, checking the total
  exceeds the page, the clamp, and the `400`.


### Fixed

- 2026-07-18 — **Two order/design test bugs in the DB-gated suites**
  (QA-CASE-MASK): the SEC-G3 masking test was born failing — its
  subject-only deny tripped the coarse blanket guard before the
  record-level concealment it pins could run; now a resource-scoped
  deny (`resource.case_type=investigation`) exercises the real
  contract (guard admits, each read path conceals). And the
  blanket-enforcement pin duplicated inside the shared requests
  binary was order-dependent (`OnceLock`-cached flag) — removed;
  `tests/enforcement.rs` owns it. Full suite green vs Postgres 18.


### Fixed

- 2026-07-18 — **Unknown-pid reads returned 500, not 404.** loco 0.16's
  `IntoResponse` catch-all maps an unmapped `ModelError::EntityNotFound`
  to a 500, so `GET /…/{pid}` with an unknown pid crashed instead of
  404ing (the organization service was immune — its `http_err` helper
  already mapped it; the copy-adaptors dropped it). Controller lookups
  now route through a `model_not_found` mapping. Family-wide fix with
  per-crate request-test pins.


### Fixed

- 2026-07-18 — **Fresh-database `db migrate` failure.** The
  `…_000004_event_outbox` migration used the loco `create_table`
  helper, which pluralizes table names (`event_outbox` →
  `event_outboxes`); its own index DDL then failed and rolled back
  the entire fresh migrate (zero tables). Rewritten as explicit SQL
  creating exactly `event_outbox`; verified against a fresh
  Postgres 18 (all migrations apply, correct table names). Family-wide
  fix (case, care-pathway, organization, portfolio; patient-flow
  shipped with the explicit-SQL form).


### Security

- **SEC-G8: default-off exposure pin.** A new unit test
  (`default_off_exposes_sensitive_reads_activation_is_a_release_gate`)
  documents explicitly that with `CASE_REQUIRE_AUTH` off (the shipped
  default) the most sensitive reads — a case's PII, the audit trail, and
  the governed `subject_of` cross-service links (§10) — are **open without a
  token**. This exposure is by design (`agents/share/security.md` §4), but
  the test pins it so activation is understood as a **tracked release gate**
  and the default cannot be flipped to "secure" silently by assumption.

- **SEC-G6: trailing slash can no longer downgrade a destructive POST.**
  `derive_action` classified `POST …/merge` etc. via `path.ends_with`,
  so `POST /api/cases/merge/` (trailing slash) fell through to `Write` —
  a non-admin `access=write` caller could reach a destructive op. The
  path is now `trim_end_matches('/')`-normalised before the suffix check.
  Test extends `derive_action_matrix` with the trailing-slash cases.

- **SEC-G2/G3: record-level authorization + masking on every read path.**
  Record-level ABAC + the `mask` obligation were enforced only on the native
  `GET /api/cases/{pid}`; `list`, `search`, `check-duplicates`, and the FHIR
  `read` / `search` took no caller and surfaced cases (pid + title, or the
  full `Task` incl. the sensitive `Task.for` subject) to anyone behind the
  blanket guard. Now `list` / `search` / `check-duplicates` **omit** cases
  the caller may not read (a denied record is indistinguishable from
  no-such-record, §10/§12), FHIR `read` returns `403` on deny and honours the
  `mask` obligation like the native GET, and FHIR `search` filters + masks
  per record. New shared `auth::read_visibility` helper; `mask_case` is now
  `pub(crate)`. DB-gated `tests/masking.rs` proves a `dept=blocked`
  (deny-read) caller cannot discover a case via list / native GET / FHIR
  read, while an ordinary caller sees it on every path.

- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance (stateless horizontal scaling) every
  relay would select and **double-ship** the same rows. `drain_once` now runs
  in a transaction and `Model::unpublished` claims rows with `FOR UPDATE SKIP
  LOCKED`, so a second relay skips the locked rows; the lock is held across
  the send window and released on commit (unpublished rows retry next tick).
  Delivery stays at-least-once (consumers dedupe on `event_id`).
- **SEC-M1: input-size caps close the O(n·m) matcher DoS.** The matcher
  runs O(n·m) character-level string similarity (Jaro-Winkler /
  Levenshtein) and O(n·m) Jaccard over a payload's text fields and arrays,
  so an unbounded single string or array was a CPU/memory DoS — amplified
  across the `check-duplicates` scan. `validation::problems` now rejects
  oversized input with a `422` *before* the record is stored or matched:
  each scalar text field (`title`, `case_number`, `agency_id`,
  `agency_name`) is capped at `MAX_TEXT_LEN` = 1024 chars; each array
  (`alternate_titles`, `subjects`, `keywords`, `identifiers`, `same_as`,
  `in_language`) at `MAX_ARRAY_LEN` = 256 entries; and each string entry
  within an array at `MAX_ITEM_LEN` = 512 chars. Violations are collected
  (report-everything), not short-circuited.
- **SEC-B5: lock merge participants against a concurrent-merge race
  (TOCTOU).** The merge handler already rejects `main == duplicate`
  (`422`), but `main`/`duplicate` were read (unlocked) before the write
  transaction, so two concurrent merges of the same duplicate could both
  see it active and fan its data into different survivors. The `outbox`
  merge path now locks both participant rows `FOR UPDATE` (in pid order, so
  opposing merges can't deadlock) and re-checks the duplicate is still
  active before writing, failing the loser closed (`streaming::merge_and_emit`).
- **SEC-G1: authorise + audit the governed bulk-links read.**
  `GET /api/cases/links` returned **every** `subject_of` (case → person)
  edge across all cases with only the coarse blanket-read gate and no audit
  — so a default read-only caller (or, with enforcement off, anyone) could
  enumerate which persons are subjects of which government cases, exactly
  the §10 governance the aggregator conceals. The handler now authorises
  the cross-case dump as a privileged governed read
  (`authorize_record(Action::Destructive, …)` — the default policy admits
  only `svc=true` machine peers or `admin`; a deployment can grant a
  dedicated reconcile identity) and writes a `links_bulk_read` audit row on
  every surfacing. DB-gated `bulk_links_requires_elevated_authority`
  (401 no-token / 403 default caller / 200 `svc`) in `tests/enforcement.rs`.

### Added — cross-service entity links: bulk-read for reconciliation (2026-07-10)

- `GET /api/cases/links[?since=<rfc3339>]` — every active outbound edge
  across all cases, in the **canonical §4.2 shape** (`edge_id` /
  `edge_kind` field names, `from_ref` = `case:<pid>`; distinct from the
  operator-facing `LinkView`), so the link-graph aggregator deserializes
  it straight into its `LinkedEvent` for reconciliation (design §8). Read
  gated by the blanket guard; `since` bounds an incremental pull.
  `EntityLink::list_all_active`. DB-gated test pins the shape. (A bulk
  read of high-sensitivity `subject_of` edges — finer per-caller
  authorisation is a §10 follow-up.)

### Added — cross-service entity links: write side (`subject_of` case → person) (2026-07-10)

- Landed the **write side** of cross-service entity linking
  (`agents/share/cross-service-linking.md` §4.1, §4.2). Case is the
  **reference** originator (rollout step 2, with a documented deviation:
  the design nominally names person + worker for `same_identity`, but
  those are older axum services with no event bus — case is the first
  loco service that both originates a v1 edge AND has the durable-bus
  outbox to emit `linked`/`unlinked`).
  - New `entity_links` table (`migration/…_000005_entity_links`: `id`
    UUID pk, `from_pid`, `kind`, `to_ref`, `role`, `confidence`,
    `provenance`, `valid_from`, `valid_to`, `deleted_at`) with the
    `UNIQUE (from_pid, kind, to_ref, valid_from)` upsert key declared
    `NULLS NOT DISTINCT` (idempotent even for a null `valid_from`).
  - New endpoints under the case prefix (`controllers/links.rs`):
    `POST` / `GET` / `DELETE /api/cases/{pid}/links`. The write is
    **optimistic** — it stores the assertion and emits an event, never
    calling the target service. Validation admits **exactly**
    `subject_of` (case → person); every other kind or endpoint pair is
    `422`.
  - Depends on the shared `entity-ref` crate
    (`entity_ref::{EntityRef, EntityType, EdgeKind}`) for the URN format
    + edge-kind registry (its `permits(from, to)` is the validator),
    rather than copying it per project.
  - Emits `linked` / `unlinked` on the existing event envelope via a new
    transactional `link_and_emit` / `unlink_and_emit` seam (same
    memory/outbox switch as the CRUD `*_and_emit`). The `Envelope` gained
    an **additive** `data: Option<Value>` field
    (`skip_serializing_if = "none"`) carrying the edge detail
    `{ edge_id, from_ref, to_ref, edge_kind, role, confidence,
    provenance, valid_from, valid_to }`; CRUD events omit it, so their
    wire shape and the frozen `EventView` projection are unchanged, and
    `SCHEMA_VERSION` does not bump.
  - Governance (§10): create AND read authorise at the "read the case"
    level (`auth::authorize_record` on the loaded case) and every
    create/withdraw writes a `linked` / `unlinked` audit row. Per-record
    masking + "denied read indistinguishable from no-such-edge" remain
    follow-ups (spec §13).
  - Tests: DB-free unit (validation accept/reject matrix; `data`-carrying
    envelope with the frozen projection) + DB-gated
    (`tests/requests/entity_links.rs`, `#[ignore]`) create → list →
    delete round-trip asserting `linked` then `unlinked`, idempotent
    re-assert, and the `same_identity` → `422` reject.

### Changed — event bus: audit now joins the outbox transaction (2026-07-09)

- Under the `outbox` transport, the `audit_logs` write now rides the
  **same transaction** as the entity mutation and its `event_outbox` row
  (`agents/share/event-bus.md` §3 — the three "can never disagree"). It
  was previously a best-effort side channel written *after* the
  transaction committed, so a crash or audit failure could leave a
  committed change + event with no audit row. `AuditModel::record` is now
  generic over `ConnectionTrait`; the `create/update/delete/merge_and_emit`
  functions own the audit write (strict/in-txn under `outbox`, best-effort
  logged under `memory`), and the native + FHIR controllers no longer audit
  separately. New DB-gated `tests/outbox_audit.rs` drives `create_and_emit`
  under `outbox` and asserts entity + event + audit all commit together.
  (The `merge_records` history row stays a best-effort side channel — it
  is merge metadata, not the §3 audit trail.)

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
