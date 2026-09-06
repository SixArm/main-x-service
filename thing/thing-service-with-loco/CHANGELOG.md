# Changelog

All notable changes to this crate are documented here.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.0.0/);
versioning: [SemVer](https://semver.org/spec/v2.0.0.html). See also:
[`index.md`](./index.md), [`spec.md`](./spec/index.md), [`README.md`](./README.md).

## [Unreleased]

### Fixed — doc drift: stale MSRV 1.95/N-3 reference (2026-09-06)

`Cargo.toml` already declares `rust-version = "1.96"` (matching
`ci/msrv.txt` and the repository's current **N-2** policy), but the
`[0.6.0] - 2026-08-27` entry below still says `"1.95"` and "current stable minus
three" — the policy in effect when that entry was written, since
tightened. That entry is left as-written (a dated record of what was
true then); this entry is the correction, matching the pattern already
used elsewhere in the family (e.g. course-service T-29,
authentication-verifier AV-3, organization-matcher). No behaviour
change — `Cargo.toml`, `ci/msrv.txt`, and `scripts/ci-check.sh msrv`
already agreed on 1.96 before this change.

### Changed — MSRV bumped to Rust 1.96, undocumented until now

`Cargo.toml` already declares `rust-version = "1.96"` (matching
`ci/msrv.txt`, the repository's **current stable minus two** floor —
`spec/rust-msrv-n-minus-2/index.md`), but no `CHANGELOG.md` entry ever
recorded the bump from the `[0.6.0]` release's declared `1.95`. Found
while working T-6 (below) — the historical `[0.6.0]` entry is left
as-is (it accurately describes what was true at that release, under
the then-current N-3 policy); this entry documents the bump that
happened after it. Same underlying drift pattern already fixed in
course-service (T-29) and authentication-verifier (AV-3), but a
different shape here: those two crates' stale claim was still in
`[Unreleased]` (so correcting it in place was honest); this crate's
stale claim is under an already-dated, already-released heading, so
rewriting it would misstate history — a new entry is the right fix.

### Added — spec-drift CI guard (T-6)

Copy-adapted `scripts/spec-drift-check.sh` + `.github/workflows/
spec-drift.yml` + `.spec-allow` from the person-service reference
(the same discipline person-matcher/worker-matcher/place-matcher/
event-matcher already carry): fails a PR that changes
`src/matching/**` or `src/models/thing.rs` without also updating
`spec/`, unless the change matches a `.spec-allow` pattern. Verified
against real commits (not by inspection): a code-only change to
`src/models/thing.rs` exits non-zero; adding a `spec/` edit exits `0`.
That exercise found and fixed a real latent bug in the copied script:
`grep -Ev` over an all-comment `.spec-allow` (its documented steady
state) exits `1`, which under `set -e -o pipefail` silently aborted
the script before its intended FAIL message — `(grep … || true) |
paste …` fixes it. The workflow file documents intended CI wiring
(this repo's root `ci.yml` is what GitHub Actions actually runs for a
nested subcrate `.yml`); it is not itself live CI. See spec/13-tasks.md
T-6.

### Added — persist and serve the review-queue `score_breakdown` (T-12)

`POST /api/things/deduplicate` always wrote `score_breakdown: None` on
every stored review-queue row even though the `MatchResult` computed
for each pair carries a real per-component breakdown, and the wire
type `ReviewQueueItem` had no `score_breakdown` field to carry one —
so the `JSONB` column the 2026-07-19 migration already declared went
unused. `MatchBreakdown` now derives `Serialize`; the scan persists it
verbatim, and `ReviewQueueItem` (both the scan response and
`GET /api/things/review-queue`) now returns it. New
`tests/review_queue_score_breakdown.rs` (DB-gated) round-trips a scan
and asserts the breakdown matches the matcher's own component scores.
See spec §13 T-12.

### Security — mask sensitive fields on `check-duplicates` / create's `409` candidates (T-13)

`GET /api/things/search` already honoured `?mask_sensitive=`, but
`check_duplicates`/`find_candidates` returned the full, unmasked stored
record on both `POST /api/things/check-duplicates` and the `409` body
`POST /api/things` returns on a duplicate hit — a caller who cannot see
a thing's full record via `GET` could still recover it by posting a
near-duplicate probe (`agents/share/security.md` invariant 5).
`find_candidates` now takes a `mask_sensitive` flag and redacts each
candidate via the existing `mask_thing` when set; `check_duplicates`
and `create_thing` each accept a matching `?mask_sensitive=` query
parameter (default `false`, same as `search`), so create's own `409`
body honours the flag the caller's create request carries.
`match_thing` is deliberately left unmasked — a `/match` caller
supplies the probe explicitly to compare it, so there is no hidden
record to protect there. See spec §13 T-13.

### Security — GTIN/ISBN/ISSN check-digit verification (T-14)

`validate_gtin`/`validate_isbn`/`validate_issn` previously checked only
length and character class, explicitly documenting "the check digit is
not verified" — so a mistyped or transposed identifier persisted as
valid, and since `thing-matcher`'s deterministic short-circuit fires on
any shared `(property_id, value)` pair, two different physical items
sharing a mistyped GTIN could spuriously match. Added
`gs1_mod10_check_digit_is_valid` (GTIN-8/12/13/14 and ISBN-13, adapted
from the sibling `place-service`'s `gln_is_valid`, generalised from a
fixed 13 digits to any length) and `mod11_check_digit_is_valid`
(ISBN-10 and ISSN, `X` = 10), wired into all three validators. Two
pre-existing test fixtures were checksum-invalid once verification was
added and are now real, checksum-valid identifiers: the dashed ISBN-10
fixture (`0-141-43951-9` → `0-141-43951-3`, the real check digit for
that book) and a GTIN-8 fixture (`12345678` → `12345670`). New unit
tests pin a valid id and a single-digit-transposed invalid one for each
of ISBN-10/ISBN-13/ISSN/GTIN-8/GTIN-13. See spec §5.4 and §13 T-14.

### Changed — `ThingMatcher` trait (T-2)

The concrete matcher facade, formerly `struct ThingMatcher`, is renamed
`ProbabilisticMatcher` (matching the sibling `event-service`/
`worker-service` naming convention). A new `ThingMatcher` trait
(`score`/`is_match`/`threshold`) is implemented for it, and
`AppState::matcher` is now `Arc<dyn ThingMatcher>` rather than
`Arc<ProbabilisticMatcher>`, so an alternative scorer (ML-based,
embedding-based, …) can be substituted with no handler change — every
call site already went through the three trait methods, never the
struct directly. Two new unit tests
(`matching::tests::probabilistic_matcher_implements_thing_matcher`,
`matching::tests::trait_score_matches_the_free_function`) prove the
trait object behaves identically to a direct `compute_match` call.

### Added — real OpenTelemetry OTLP export (T-11, PRO-H12 slice 3 of 7)

- **2026-08-30**: new `src/observability.rs`. This crate carried no
  *working* observability module before this change —
  `opentelemetry`/`opentelemetry-otlp`/`opentelemetry_sdk`/
  `tracing-opentelemetry` were declared at stale 0.27/0.28 pins with
  zero consumers anywhere in `src/` (dead scaffolding from an earlier,
  since-deleted stub), bumped to the family's settled 0.32/0.33 pins in
  the same change. Close port of person-service's, itself a port of
  link-graph-service's original reference. `observability::trace_mw` is
  layered as the outermost middleware on both of this crate's
  router-construction surfaces (`App::after_routes` and
  `api::rest::create_router`).
- Needed the renamed `otlp-test-tonic = { package = "tonic", … }`
  dev-dependency, same as PRO-H9's three crates and place's slice
  (PRO-H12 slice 2): this crate already declares `tonic = "0.12"` +
  `tonic-build` in anticipation of the still-open T-3 (gRPC
  implementation), and a declared-but-code-unused dependency collides
  with an unrenamed dev-dependency the same way a genuinely used one
  does.
- `tests/otlp_export.rs` + `tests/otlp_middleware.rs` +
  `tests/otlp_collector/` (ported from place) prove real export
  against a real in-process gRPC listener. Verified independently:
  `cargo fmt --check`, `cargo clippy --all-targets -D warnings`,
  `cargo deny check`, the MSRV check, and `cargo bench --no-run` all
  clean; `cargo test --lib` 205/205 (was 197, +8 new); `cargo test
  --test otlp_export --test otlp_middleware` 4/4. See spec §13 T-11.

## [0.6.0] - 2026-08-27

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3/index.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.

### Fixed — `POST /api/things` demanded server-owned fields (QA-SERVER-FIELDS)

- **The JSON extractor required `id`, `is_deleted`, `created_at`,
  `updated_at`, `alternate_names`, `identifiers`, `images`, and
  `same_as` on the wire**, even though the server owns every one of
  them. A hand-written create body (the way a real API client writes
  one) omitting any of them was refused with `422 missing field …`
  before the handler, validation, or the repository ever ran —
  demanding a value it then ignored or discarded. Same defect, same
  fix, as the event service's `created_at`/`updated_at` fix
  (2026-08-01): every server-managed field is now `#[serde(default)]`.
  `name` — the one field the server does not own — is also
  `#[serde(default)]` now, so an omitted `name` reaches the normal
  `validate_thing` path (`422 validation_error`) instead of being
  turned away by the extractor's generic "missing field" error.
- **Making the fields optional alone would have been a new bug**: an
  omitted `id` defaults to the nil UUID, and the repository previously
  persisted whatever `id`/`created_at`/`updated_at` the domain value
  carried verbatim rather than overwriting them — so a second
  hand-written create would have collided on the same nil primary key,
  and an omitted timestamp would have stored the Unix epoch. Fixed
  alongside: `create_thing` mints a fresh id whenever the wire value is
  nil (mirroring the event service's existing pattern), and the
  repository now stamps `created_at`/`updated_at` to "now" on insert
  (preserving `created_at` and refreshing `updated_at` on update)
  rather than trusting the passed-in domain value.
- New DB-gated regression suite `tests/api_integration_test.rs`: a
  minimal hand-built create body succeeds and reads back a fresh id,
  ~now timestamps, and empty collections; two consecutive hand-written
  creates mint distinct ids rather than colliding; an omitted `name`
  still fails, but via `validation_error`, not the JSON extractor.
  Found alongside the same defect in the place service (both surfaced
  while writing place's auth activation proof, whose payload is built
  by serializing `Place::new`/`Thing::new` rather than by hand for
  precisely this reason — `tests/enforcement.rs`'s comment is now
  updated to reflect the fix).

  Suite: 3/3 new + crate-wide green vs Postgres 18 (`scripts/ci-check.sh test-db`).

### Added — Durable event bus, real-broker sink (BUS-3, 2026-08-03)

`FluvioSink` (`src/relay.rs`) — the Phase-3 relay's real-broker
`EventSink`, ported from the case-service reference (BUS-1), behind
this crate's own `fluvio` Cargo feature (off by default; `fluvio`
0.50). One producer per topic, partitioned by record `pid` per
`agents/share/event-bus.md` §7. New env vars: `THING_FLUVIO_ENDPOINT`
(unset ⇒ unchanged `LoggingSink` default) and `THING_EVENT_TOPIC`
(default `mxi.thing.events`). An endpoint configured without the
`fluvio` feature refuses to start the relay rather than silently
falling back to `LoggingSink` — that fallback would mark outbox rows
`published_at` without ever reaching the broker the operator asked
for. `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a local
SC+SPU broker for opt-in manual runs (not part of any automated CI
stage); `tests/fluvio_relay.rs` is a feature-gated, `#[ignore]`d
live-broker round-trip (using this crate's own `DATABASE_URL`-direct
DB-gated-test convention, not case's `loco_rs::testing` harness),
verified by compiling under `--features fluvio` rather than an actual
execution (no broker is stood up in this repo's CI). No
`compliance/soup.tsv` exists in this crate, so no SOUP register update
applies.

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16.4 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. Feature renames applied: `auth_jwt` → `auth`,
  `bg_pg` → `worker`.
- **4 raw `Statement` call sites** in `src/db/review_queue.rs` move to
  the `_raw` variants — this crate's only hand-rolled SQL.
- A `useless_conversion` in `src/db/outbox.rs` from a now-redundant
  `.into()`.
- No `EntityTrait`-import gap (`db/models.rs` glob-imports the sea-orm
  prelude per-module, unlike person/place), no `BigDecimal`, no
  `DatabaseConnection::Disconnected`, and the one table already keys
  on `i64` — the smallest fallout of any person-style crate so far.
- No behavioural change; verified with the full DB-gated suite (3
  tests, unchanged count) against a freshly migrated Postgres 18.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-1, completing the five axum-style services (person was the reference).

- **One reloadable verifier, one reloadable policy.** Both left
  `AppState` for process-wide holders that the blanket guard **and** the
  `AuthUser` / `MaybeAuthUser` extractors read per request. Snapshotting
  them in the state meant a rotation or a policy edit could reach one
  path and not the other; only the `require_auth` flag is still a boot
  value, because turning enforcement on or off mid-flight is not
  something to do without a restart.
- **`spawn_key_refresh`** re-fetches `THING_PASETO_KEYS_URL` every
  `THING_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset). A failed fetch **keeps the current key set** —
  a transient auth-service outage must not lock every caller out.
- **`policy()` is a `ReloadablePolicy`**, with `reload_policy()` and
  **`spawn_policy_watcher`** polling `THING_ABAC_POLICY_FILE`'s mtime
  every 15 s; a malformed edit falls back to the built-in default rather
  than leaving the service unprotected.
- **`tests/enforcement.rs`** — the activation proof, in its own binary
  because the auth `OnceLock`s are process-wide, and carrying its own
  minimal router builder because this crate has no HTTP test harness.
  With `THING_REQUIRE_AUTH=1` over the **production** router: public paths
  stay open, a protected read and write without a token are `401`, a
  malformed bearer is `401` (not a 500), a valid token with no
  attributes reads `200` and writes `403`, and `access=write` creates.
- New environment variable: `THING_PASETO_KEYS_REFRESH_SECS`. New
  dev-dependencies: `serial_test`, `tower`.


### Added — row-level integrity digests + verify endpoints (2026-07-28)

Third of the eight `*_REQUIRE_AUTH`-family crates to gain tamper
evidence, and the first `api/rest`-shaped one whose records span
several tables.

- **`src/compliance/mac.rs`** — SHA-256, SHA-3, and (when a key is
  configured) a keyed MAC over the **assembled** record — not just the
  root `things` row. A thing's `identifiers` live in a child table and
  are exactly the kind of field worth editing quietly (it is what a
  downstream system matches on), so the digest is recomputed from a
  full repository read, not the row alone. Stamped in `to_active`,
  the single place both `create` and `update` build their active
  model through, so no write path can forget to re-digest.
- **`src/compliance/record_integrity.rs`** / **`audit_integrity.rs`**
  — the verification side: reassemble each record (or audit row),
  recompute its digests, and name any row whose stored digest no
  longer matches its content.
- **`GET /api/records/verify`** and **`GET /api/audit/verify`**
  (`?limit=`, capped at 1000 — lower than the JSONB-shaped services'
  10000 cap, since each row here costs a repository read, not a single
  JSONB fetch) expose the check over HTTP. Landed the same day as a
  same-day follow-up ("Make every integrity verification reachable")
  that swept for the same defect class already fixed twice elsewhere
  and mounted both handlers.
- Digests are written even when no MAC key is configured — the
  unkeyed SHA-256/SHA-3 pair is the only integrity a default
  deployment's audit rows have.

### Changed — `Config::from_env` gained a testable seam and more variables (2026-07-23)

- The env overlay moved into a pure `Config::from_source(lookup)`;
  `from_env` is now a two-line delegation to it. This makes the
  variable-to-field mapping unit-testable without mutating the process
  environment — which matters because `std::env::set_var` is `unsafe`
  in the 2024 edition (this crate forbids `unsafe`) and process env is
  global state that makes parallel tests flaky.
- Added variables: `SEARCH_CACHE_SIZE_MB`, `STREAMING_BROKER_URL`,
  `STREAMING_TOPIC` (the previously-unreachable config fields).
- A blank or whitespace-only value now counts as **unset** rather than
  overwriting the default with an empty string, and typed values
  tolerate surrounding whitespace (a `.env` line like `SERVER_PORT = 9090 `).
- Pinned by five unit tests; behaviour is otherwise unchanged.

### Added — stored review queue + decision endpoints (2026-07-19)

- `review_queue` table (migration `m20260719_000001_create_review_queue`):
  the batch-dedup scan persists its candidate pairs (normalized pair
  order, UNIQUE upsert — re-scans refresh scores, decided rows keep
  their decision, ids stay stable) and the scan response now reports
  the **stored** rows.
- `GET /api/things/review-queue[?status=&limit=]` — list the stored
  queue (newest first, cap 500).
- `POST /api/things/review-queue/{id}/decision`
  (`{"status": "confirmed" | "rejected"}`) — decide a `pending` item;
  first-writer-wins in SQL, `404`/`422` on unknown/already-decided.

### Fixed

- 2026-07-19 — dedup-report drift: `POST /api/things/deduplicate` now returns the family's
  person/worker-shaped report (see the place service entry — same
  change): counts + `auto_merged` + `queued_for_review` +
  `review_items[]` incl. `detection_method` and lowercase `status`
  wire tokens. Serde pin added.

### Security

- **SEC-M1: input-size caps on the `Thing` payload.** The validator
  enforced format/required rules but capped no field's *size*, so a single
  multi-megabyte text field or a huge array could be a CPU/memory `DoS`
  against the matcher's O(n·m) string / Jaccard scoring, amplified across
  the `check-duplicates` / `deduplicate` scan. `validate_thing` now also
  bounds every scalar text field (`MAX_TEXT_LEN = 1024`: `name`,
  `description`, `disambiguating_description`, `additional_type`, `url`,
  `main_entity_of_page`, `owner`, `subject_of`, `potential_action`),
  string-array cardinality + per-entry length (`MAX_ARRAY_LEN = 256` /
  `MAX_ITEM_LEN = 512`: `alternate_names`, `images`, `same_as`), and the
  `identifiers` cardinality + each identifier's `value` / `name` / `url` —
  field-scoped `422`s *before* persist/match. Factored into
  `thing_size_caps` / `cap_*` helpers. Unit tested.

- **SEC-G5: blanket guard is now guard-all (deny-unless-public).** The
  `enforce` decision previously only gated paths under `/api` or `/fhir`,
  silently allowing any out-of-prefix route (e.g. `/`, `/admin`) through
  with no token when enforcement was on. It now denies every path except an
  explicit public allow-list (`is_public_path`: `/_health`, `/_ping`,
  `/api-docs/openapi.json`, `/swagger-ui*`, `/metrics.prom`, `/api/health`,
  `/fhir/metadata`). The now-unused prefix helpers (`API_PREFIX`,
  `FHIR_PREFIX`, `GUARDED_PREFIXES`, `is_guarded_path`, `PUBLIC_API_PATHS`)
  were removed.
- **SEC-G6: trailing-slash normalisation in `derive_action`.** A trailing
  slash (`/api/things/merge/`, `//`) no longer downgrades a destructive
  named POST (`/merge`, `/deduplicate`, `/import`) to `Write`, which would
  have let a non-admin `access=write` caller reach a destructive op; the
  path is `trim_end_matches('/')`-normalised before the suffix check so it
  stays `Destructive`.

- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance every relay would **double-ship** the
  same rows. `drain_once` now runs in a transaction and `unpublished` claims
  rows with `FOR UPDATE SKIP LOCKED` (a second relay skips locked rows; the
  lock releases on commit). Delivery stays at-least-once (consumers dedupe on
  `event_id`).

### Added — authz: ABAC policy authorization inside the blanket guard

- ABAC authorization landed (spec §13 T-4, the final sub-item —
  supersedes the earlier roles sketch; family contract:
  `agents/share/authorization-attributes.md`). When
  `THING_REQUIRE_AUTH` is on, a verified PASETO token is further
  checked by the shared policy engine in `authentication-verifier`
  0.3: the request's action is derived from the HTTP method plus the
  crate's destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES`
  — `/merge`, `/deduplicate`, `/import`), and the policy is evaluated
  over the token's new `attrs` claim, first-match-wins, defaulting to
  allow-read / deny-mutation.
- New env vars `THING_ABAC_POLICY` (inline JSON) and
  `THING_ABAC_POLICY_FILE` (path), read once at `AppState`
  construction by the new `auth::policy_from_env` (restart to
  change); unset or unparsable ⇒ `tracing::warn!` + the built-in
  default policy (`svc=true` ⇒ everything; `access=admin` ⇒
  destructive+write; `access=write` ⇒ write) — the service always
  boots.
- `auth::enforce` now takes the HTTP method and the policy and
  returns `403` (with the deciding-rule reason) for a valid token the
  policy denies; `401` remains missing/bad credential. `AppState`
  carries the policy (`policy: Arc<Policy>`) alongside the verifier
  and the `require_auth` flag.
- DB-free unit tests pin the family §7 matrix: action derivation,
  empty-`attrs` read-only default, `access=write` / `access=admin` /
  `svc=true` tiers, deny-beats-later-allow, 401-vs-403, bad-policy
  fallback.
- Flag off ⇒ behaviour-neutral: no authn and no authz, exactly as
  before.

### Added — boot-time HTTP fetch of the PASETO key set (2026-07-04)

- Boot-time key-set fetch landed (the fetch part of spec §13 T-4;
  family contract shared with the sibling services). New env var
  `THING_PASETO_KEYS_URL`: when set (non-blank), `App::after_routes`
  calls the new `state::boot_verifier` (async), which fetches the
  key-set JSON once via `Verifier::from_paseto_keys_url` — the
  `authentication-verifier` path dep now enables its `fetch` feature.
  A successful fetch **wins** over any `THING_PASETO_KEYS` env value
  (`tracing::info!` records the source URL and key count); any fetch
  failure `tracing::warn!`s and falls back to the env path (else the
  empty reject-all key set) — the service **always boots**.
  Unset/blank URL ⇒ previous behaviour exactly.
- Boot order matters and is now explicit: the fetched verifier is
  installed with `AppState::with_verifier` **before** the shared-store
  insert and before `require_auth_mw` captures the state, so both
  router surfaces (the loco router's enforcement middleware and the
  `FromRef` handler extraction) consult the fetched key set.
- Fetch happens once at boot; there is deliberately no refresh loop —
  key-rotation re-fetch is a roadmap item (spec §15).
- New DB-free tokio tests in `src/api/rest/auth.rs`: a local
  ephemeral-port axum listener serves the in-process test key set and
  the fetch-built verifier accepts a token signed by that key; a
  fast-failing URL (`http://127.0.0.1:1/`) falls back to the env/empty
  path without panicking.

### Added — offline PASETO v4.public bearer verification

- Peer-side bearer-token verification landed (the verification part of
  spec §13 T-4), per the family-wide design in
  `agents/share/authentication-sessions.md` §5: a new `AuthUser`
  extractor and `GET /api/whoami` endpoint (`src/api/rest/auth.rs`)
  verify **PASETO `v4.public`** (Ed25519) bearer tokens offline —
  signature, footer `kid`, `iss`, `aud`, `exp` — via the monorepo
  `authentication-verifier` 0.2 path dependency. No shared secret, no
  per-request introspection hop.
- The verifier is built from the environment at boot:
  `THING_PASETO_KEYS` (the Ed25519 key set the auth service publishes
  at `/.well-known/paseto-keys`), `THING_TOKEN_ISSUER` (default
  `authentication-service`), `THING_TOKEN_AUDIENCE` (default
  `main-x-service`). Absent/blank/unparseable key set ⇒ empty key set:
  every token is rejected but the service still boots.
  `AppState::with_verifier` swaps in a replacement (e.g. one built from
  a freshly fetched key set).
- New DB-free unit tests in `src/api/rest/auth.rs` mint `v4.public`
  tokens in-process (throwaway Ed25519 key via `rusty_paseto` +
  `ed25519-dalek` dev-deps) and pin valid / missing / non-bearer /
  expired / tampered / no-key outcomes.
- Blanket `/api/*` enforcement has since landed default-off (see the
  next section); roles + published-key HTTP fetch remain open in spec
  §13 T-4.

### Added — blanket `/api/*` auth enforcement (default-off, 2026-07-04)

- The enforcement remainder of spec §13 T-4, per the family contract
  in `agents/share/jwt-enforcement.md`: a pure `auth::enforce`
  decision plus the `auth::require_auth_mw` Axum middleware require a
  valid PASETO `v4.public` bearer token on every `/api/*` route when
  `THING_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
  case-insensitive via `auth::parse_bool`; anything else including
  unset/blank ⇒ off — the default, so behaviour is unchanged until a
  deployment opts in). The flag is read once at `AppState`
  construction (`auth::require_auth_from_env`, carried as
  `AppState::require_auth`); restart to change.
- Public allow-list (`auth::PUBLIC_API_PATHS`): `/api/health` stays
  public inside the enforced prefix; root-level `/_health`, `/_ping`,
  `/api-docs/openapi.json`, `/swagger-ui*`, and `/metrics.prom` sit
  outside the `/api` scope and are never gated (segment-aware prefix
  check, so `/api-docs` is not mistaken for `/api/...`).
- Wired on **both** router surfaces via
  `axum::middleware::from_fn_with_state` — the hand-written
  `create_router` and the loco router in `App::after_routes` — inside
  the CORS layer so preflight requests are still answered.
- New DB-free unit tests in `src/api/rest/auth.rs` pin the full
  matrix: off + no token ⇒ pass; on + public / out-of-scope paths ⇒
  pass; on + protected + no token ⇒ `401`; on + valid token ⇒ pass;
  on + expired / tampered ⇒ `401`; plus the lenient `parse_bool`
  flag-parser semantics.
- Still open in spec §13 T-4: roles (editor / read-only / service)
  and fetching the published Ed25519 key set over HTTP at boot.

### Added — matcher bridge

- New `src/matching/adapter.rs` exposing
  `to_matcher_thing(&service::Thing) -> thing_matcher::Thing`.
  Projects the schema.org-shaped service record into the matcher
  crate's builder shape: 1:1 mapping for `name`, `description`,
  `disambiguating_description`, `url`, `main_entity_of_page`,
  `owner`, `alternate_names`, `same_as`; singular `additional_type`
  / `subject_of` become first entries of the matcher's list fields;
  first `images` entry becomes the matcher's single `image`;
  identifiers map via `map_identifier_property` to schema.org
  canonical tokens (`doi`, `isbn`, `issn`, `gtin`, `sku`, `mpn`,
  `serialNumber`, `uri`, `uuid`; `Custom(s)` passes through
  verbatim); registry-only fields are dropped.
- `src/matching/mod.rs` now re-exports the sibling `thing-matcher` crate
  as `matcher_lib`, so callers can reach `MatchingEngine`,
  `MatchConfig`, `MatchResult`, `MatchBreakdown`, `Confidence`, and
  every public matcher type without taking a separate dependency.
- Field-routing rules are inline-documented in `adapter.rs` and
  pinned by `tests/duplicate_detection.rs`.

### Added — tests

- New `tests/duplicate_detection.rs`. Black-box bridge tests that
  drive service records through `to_matcher_thing` and assert on
  the canonical `MatchingEngine::match_things` output. Covers
  identical clones, name typos (Jaro-Winkler), deterministic
  short-circuits (DOI / ISBN / UUID), negative cases
  (unrelated records, same name + different ISBNs), per-adapter field
  routing, and config-preset invariants (strict ⊆ lenient).

### Added — bridge benchmarks

- New `benches/bridge_bench.rs` (Criterion). Three groups:
  `bridge_adapter_only` (projection cost on minimal vs. rich
  records), `bridge_end_to_end` (adapter + engine call), and
  `bridge_one_to_many` (single query vs. 10 / 50 / 100 candidates).
  Regression guard for the duplicate-check hot path.

### Added — observability

- New `src/metrics.rs` exposing a process-wide `LazyLock<Metrics>`
  Prometheus registry. Standard counters
  (`thing_created_total` / `_updated_total` / `_deleted_total` /
  `_matched_total`, labeled `http_requests_total`) and histograms
  (`http_request_duration_seconds`, `thing_match_score`,
  `thing_search_duration_seconds`).
- **Prometheus metrics endpoint `GET /metrics.prom`.** The previously
  dead `src/metrics.rs` registry is now actually scraped: a new
  `handlers::metrics_prom` handler renders `thing::metrics::METRICS`
  in text-exposition format (`text/plain; version=0.0.4`), served at
  the application **root** path `/metrics.prom` (not under `/api`) so
  a default Prometheus scrape config (`metrics_path: /metrics.prom`)
  finds it. Wired both as a loco controller group
  (`api::rest::metrics_routes`, registered in `App::routes`) and on the
  hand-written Axum router (`create_router`), and added to the
  `OpenAPI` document under a new `observability` tag. The `thing_*`
  counters (`thing_created_total` / `_updated_total` / `_deleted_total`
  / `_matched_total`) and the latency/score histograms are now
  externally observable. New DB-free tests pin the `/metrics.prom`
  `OpenAPI` path and the root loco-route binding (`api::rest::tests`).
  Brings parity with the older Axum services, which already expose
  Prometheus metrics.

### Added — UI

- `assets/static/css/themes/` ships 39 standalone Lily Design System
  themes (light, dark, dracula, nord, cyberpunk, … + four
  United Kingdom NHS variants). The layout's theme picker now lists
  all 39; default is `light`. Selection swaps the `<link href>` of
  `<link id="lily-theme-css">` at runtime; persisted in
  `localStorage["lily-theme"]`. The command palette also lists all
  39 themes.

### Changed — Loco background jobs

- Dropped the `bg_redis` and `bg_sqlt` features from the `loco-rs`
  dependency. Background jobs are now backed exclusively by
  PostgreSQL (`bg_pg`), using the same database as application data
  — no external Redis broker. `config/development.yaml` and
  `config/production.yaml` updated to `queue.kind: Postgres` with
  `uri: DATABASE_URL`. Removes the `rusty-sidekiq` →
  `redis 0.22.3` future-incompat warning chain.

### Changed — documentation

- Reduced healthcare / clinical / patient / hospital / clinician /
  practitioner framing across spec.md, AGENTS.md, agents/*, README,
  CLAUDE.md, and index.md. Preserved: FHIR R5 resource and field
  names (e.g. `Patient.birthPlace`, `Practitioner` resource),
  national-identifier proper nouns (United Kingdom National Health
  Service Number, Australia IHI), paper citations, the
  `compliance-for-healthcare.md` doc, and `HIPAA` / `NHS` / `PHI`
  as compliance regimes.
- `spec.md §11 Testing Strategy` now lists the bridge integration
  tests; `agents/testing.md` gained a `## Bridge Integration Tests`
  section; `agents/restful.md` gained adapter + Prometheus blocks;
  `index.md` gained a worked example showing the canonical bridge
  in action.

### Fixed

- The thing-matcher crates.io 0.3.0 API drift (Sweden personnummer
  renamed from `se_personnummer` to `se_workernummer`,
  `united_kingdom_national_health_service_number` shortened to
  `uk_nhs_number`) is now caught at the matcher level by each
  matcher's `tests/adapter_contract.rs` — see the matcher
  CHANGELOG.

### Removed

- The unused `jsonwebtoken` dependency (never referenced in `src/` or
  `tests/`). The family auth design has pivoted from RS256 JWT / JWKS
  to cookie sessions + short-lived PASETO v4.public tokens (see
  `agents/share/authentication-sessions.md`); the still-pending auth
  task in `spec/13-tasks.md` T-4 now targets PASETO verification via
  the `authentication-verifier` crate.
