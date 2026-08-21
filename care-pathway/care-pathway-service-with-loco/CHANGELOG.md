# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]





### Added — cargo-fuzz harness for the request-path logic (FUZZ-2)

A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate with
three coverage-guided libFuzzer targets. Until now the harnesses covered
only the dependency-light libraries; the services had none, despite
carrying the surface that actually faces the network.

- **`validate_json`** — the real request path: arbitrary bytes →
  `serde_json` → `CarePathway` → `validation::problems`. Never-panic,
  deterministic, and a **bounded problem report**.
- **`validate_built`** — the validator driven directly, building the
  `CarePathway` from raw bytes so the fuzzer controls array cardinality and
  entry contents without first having to learn JSON. A run of NUL bytes
  becomes a run of blank entries — the exact SEC-M8 shape.
- **`merge_pathways`** — the merge fold over two arbitrary payloads:
  never-panic; the survivor keeps its `name`; deterministic; and
  **absorbing**, so a retried merge cannot inflate the record.

The sub-crate declares an empty `[workspace]` table: this crate is a
workspace root, so `fuzz/` would otherwise be pulled in as a member, and
a cargo-fuzz build needs its own sanitizer flags and lockfile. Nightly
only, so it is exempt from the repo MSRV. See
[`fuzz/README.md`](./fuzz/README.md).

### Fixed — an over-long array produced one `422` problem string per entry (SEC-M8)

`validation::problems` reported an over-long array's cardinality
violation once and then still walked **every** entry, so a payload with
ten thousand malformed `condition_codes` came back with ten thousand problem strings — which
the controller joins into a single `422` body. A small request bought a
large response. Worse here than a blank check: each entry also ran an
ICD-10 / ICD-11 / SNOMED CT code validation, SNOMED including a Verhoeff
check digit.

Every per-entry loop now walks a new `inspected()` helper, which yields
at most `MAX_ARRAY_LEN` entries. The cardinality problem already rejects
the payload, so inspecting the tail decides nothing; bounding the
**report** is the same input-bounding rule (SEC-M1) as bounding the work.
The helper is named rather than inlined at each call site so a per-entry
loop added later without it reads as different from the ones that have
it. Pinned by a test.

Case landed this first as the reference; this is the roll-out
(repo `tasks.md` SEC-M8b).

Measured with `benches/service_bench.rs`: the oversized-array rejection
path went from **112 µs to 4.9 µs**, a ~96% reduction.

### Fixed — the search index built a new Tantivy writer on every write

`SearchEngine::index_pathway` (and `delete_*` / `clear`) called
`self.index.writer(WRITER_HEAP_MB)` per call. Tantivy's `IndexWriter`
allocates its whole 50 MB arena and spawns merge threads on
construction, so **every create, update, merge, and soft-delete paid
that setup synchronously**, on the request path. Measured at ~155 ms per
indexed document against a fresh index; holding one writer for the
process brings it to ~78 ms, the remainder being the durable commit and
reader reload that indexing-on-write inherently costs.

It was also a concurrency hazard, not only a slow one: an `IndexWriter`
holds the index directory's exclusive lock, so taking and releasing it
per call left two simultaneous writes able to collide on it. One owner
for the process cannot.

The engine now holds a `Mutex<IndexWriter>` created in `new()`. A
poisoned lock recovers the guard rather than failing for ever — the only
operations held across it are `delete_term` / `add_document` / `commit`,
and a permanently dead index would be the worse outcome.

Found by the new benchmark, which is the point of having one.

### Added — Criterion benchmarks

- `benches/service_bench.rs`, covering the CPU-bound halves of a request
  — the part a database benchmark hides behind I/O. Three groups:
  **validation** (every create and update pays it; the `oversized_arrays`
  case exercises the SEC-M1 input caps, because rejecting an abusive
  payload has to be cheap or the caps are not doing their job),
  **merge** (a whole-record fold, with a scaling case showing the cost
  sits in the collections it unions), and **search** (indexing one
  document — what every write pays synchronously — plus exact / fuzzy /
  phonetic retrieval and the `candidates` blocking query a duplicate
  check actually calls, against a populated index). The validation group additionally prices `condition_code_issue` on its own, since it is the one rule whose cost grows with how thoroughly a pathway is coded. The SOUP register gained the `criterion` annotation — the crate's own IEC 62304 §8.1.2 gate caught the unannotated dependency, as designed.
- `criterion` is a new dev-dependency; test-only, so it is not in any
  release artefact.

### Added — declared MSRV (Rust 1.95)

- `Cargo.toml` now declares `rust-version = "1.95"`, the repository's
  **current stable minus three** floor
  (`spec/rust-msrv-n-minus-3.md`). Sourced from `ci/msrv.txt` and
  enforced by `scripts/ci-check.sh msrv`, which asserts the declared
  value matches that file and then compiles the crate — `--all-targets`,
  so benches and tests count — against the 1.95 toolchain. Behaviour is
  unchanged; what changes is that the floor is now a checked claim
  rather than an unstated assumption.

## [0.1.0] - 2026-08-04
### Added — Durable event bus: real Fluvio broker sink (BUS-3, 2026-08-03)

Ported from case-service's BUS-1 reference implementation
(`case/case-service-with-loco/src/relay.rs`), mechanically adapted to
this crate's `CARE_PATHWAY_` env prefix and `care_pathway` entity token.
All three phases of the durable event bus are now done: transactional
outbox (Phase 2), relay/retention (Phase 3, already landed), and now the
real-broker sink.

- `src/relay.rs` — `FluvioSink` (`impl EventSink`), behind this crate's
  own `fluvio` Cargo feature (off by default, so a default build's
  dependency tree and behaviour are unchanged). `spawn()` now selects
  `FluvioSink` over the existing `LoggingSink` when
  `CARE_PATHWAY_FLUVIO_ENDPOINT` is configured (default topic
  `mxi.care_pathway.events`, overridable via `CARE_PATHWAY_EVENT_TOPIC`).
  An endpoint configured **without** the `fluvio` feature compiled in is
  a clean refusal to start the relay (logged at `error`), never a
  silent `LoggingSink` fallback that would mark outbox rows published
  without ever reaching a real broker (the same "no fallback on an
  explicit backend choice" posture as the bulk artifact store, §12 of
  `agents/share/bulk-import-export.md`). The initial broker connection
  retries indefinitely rather than falling back.
- `Cargo.toml` — `fluvio = { version = "0.50", optional = true }` +
  `[features] fluvio = ["dep:fluvio"]`, alongside the crate's existing
  `s3` optional-feature pattern.
- `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` — an opt-in local
  Fluvio broker (`mxi-care-pathway-fluvio-*` containers), for manual
  exercise of the `fluvio` feature; not part of any automated CI stage.
- `tests/fluvio_relay.rs` — a `fluvio`-feature-gated, `#[ignore]`d live
  round-trip test, verified today by compiling under `--features
  fluvio` (no broker is stood up by any automated run in this repo).
- `compliance/soup.tsv` — a `fluvio` SOUP row.

### Added — Privacy: field masking + GDPR export (2026-08-02)

Repo tasks.md P-2 (as P-1/organization). A `CarePathway` is a
**template** — it names no patient — so the masking scope here is
deliberately thinner than organization's: only `provider_name` /
`provider_id` are redacted; every clinical field (`name`,
`condition_codes`, `interventions`, `keywords`, `identifiers`) is left
alone, since masking it would defeat the registry for no privacy gain.

- `src/privacy.rs` — `mask_pathway` + `export_pathway`, mirroring
  organization's shape (masked-value tail-preserving redaction, an
  export envelope that declares whether it is partial).
- `src/auth.rs` — `authorize_record` + `care_pathway_resource_attrs`.
  The resource attributes are `care_setting` (a policy discriminator)
  and a new `sensitive_setting` flag: `true` for `mental_health` /
  `palliative` pathways, the two settings that carry special-category
  treatment under UK Common Law Duty of Confidentiality and analogous
  regimes even though the template names no one.
- `GET /api/care-pathways/{pid}` now honours the ABAC `mask`
  obligation, returning the redacted view instead of the full record.
  New `GET /{pid}/masked` (always-redacted view) and `GET /{pid}/export`
  (audited GDPR right-of-access) endpoints.
- The export is audited as a **disclosure** via the existing
  `disclosure::action::EXPORT` (this crate's own HIPAA §164.528
  machinery, richer than organization's plain `AuditModel::record`) —
  every export, masked or not, is a recordable compliance event.
- **Explicitly out of scope, not silently skipped**: the
  patient-identifying fact is a specific person's *enrolment* on a
  pathway — `pathway_instances.subject_ref`, a `person:<uuid>`
  reference — which lives on the instance layer, not on `CarePathway`
  at all. Masking/authorizing that linkage (the clinical analogue of
  the `case ↔ person` `subject_of` edge) is tracked in spec §16 as a
  follow-up.
- DB-gated: `tests/masking.rs` (new, 2 tests — the obligation
  end-to-end, and a record-level-decision proof that a
  non-sensitive-setting read is *not* masked by the same
  `dept=partner` policy) + 2 new tests in
  `tests/requests/care_pathways.rs` (`masked_view_and_export_are_served`,
  `masked_view_and_export_are_404_for_unknown_pid`) + 2 new DB-free
  unit tests in `src/auth.rs`.
- No SOUP register change (no new dependency).
- Fixed stale spec/AGENTS prose left over from S-2 (2026-08-01): both
  still described the name search as Postgres `ILIKE` in a couple of
  places despite Tantivy having landed.

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration → 2.0,
  sea-query → 1.0. Raw `Statement` queries in `models/audit_logs.rs`
  (the advisory-lock statement) and `tests/requests/compliance.rs` move
  to `execute_raw`; a `useless_conversion` in `models/event_outbox.rs`
  from a now-redundant `.into()`.
- **loco's `ColType::PkAuto` now generates a 64-bit primary key.** The
  `care_pathways`, `audit_logs`, and `merge_records` entities move from
  `i32` to `i64`, along with every place that carries one of their row
  ids: the audit hash-chain (`ChainBreak.id`, the checkpoint's
  `anchor_id`), and the compliance test fixtures. The five
  `instances`/`outcomes` tables and `event_outbox` stay `i32` — their
  migrations write raw SQL (`id SERIAL PRIMARY KEY`) rather than the
  loco schema DSL, so they never picked up the width change.
- **sea-orm 2.0 dropped `DatabaseConnection::Disconnected`**, the
  variant `src/compliance/disclosure.rs`'s test used as a stand-in for
  "a connection that errors if touched" (proving the read-audit no-op
  path never reaches the database when auditing is off). Replaced with
  a `MockDatabase` carrying no queued results, which errors identically
  on the first real query — added as a `mock`-feature dev-dependency so
  it never reaches a release build.
- No behavioural change; verified with the full DB-gated suite (46
  tests, unchanged from before this bump) against a freshly migrated
  Postgres 18. The IEC 62304 SOUP gate needed no new entry — `loco-rs`
  and `sea-orm` were already registered in `compliance/soup.tsv`; this
  is a version bump of existing dependencies, not a new one.

### Added — Tantivy full-text search, fuzzy + phonetic, dedup blocking (2026-08-01)

Replaces the Postgres `ILIKE` name search (spec §13 T-6), following
organization's loco-adapted pattern.

- **`src/search/`** — the index schema and a `SearchEngine` facade.
  Indexed: `name`, `alternate_names`, Soundex codes of every name token,
  `provider_name`, identifier values, `keywords`, **`condition_codes`**
  (as lowercased `system:code` pairs) and `interventions` full-text;
  `pathway_code`, `provider_id`, `care_setting` and `active` exact. Only
  `pid` is stored — hits resolve against Postgres, which stays the source
  of truth.
- **What that buys clinically:** a pathway's *defining* attribute is the
  condition it targets, and an `ILIKE` over `name` could not find it. A
  search for `I63` or `thrombolysis` or `NICE-NG128` now reaches the
  right pathway; pinned by a DB-gated test.
- **`GET /search`** keeps `?q=` and its pagination, and gains
  `fuzzy=true` (Levenshtein ≤ 2) and `phonetic=true` (Soundex). Its
  `X-Total-Count` now comes from the index's `Count` collector rather
  than a SQL `COUNT(*)`.
- **`check-duplicates` blocks on the index** (fuzzy title + exact
  identifier + phonetic routes, ≤ 200 candidates) instead of scanning up
  to 1000 rows, so a duplicate's reachability depends on similarity
  rather than insertion order. An unavailable index is a `503`, never a
  silent "no duplicates" — that answer would let a caller create a
  duplicate believing it had been checked.
- **Indexing is wired into `src/streaming.rs`**, the seam both the native
  and FHIR controllers write through, after the write is durable and
  best-effort: a failed index write is logged at `ERROR` and never fails
  a committed request.
- **Recovery:** `cargo loco task search_reindex` rebuilds from the
  database, and an empty index over a populated table is rebuilt at boot
  (`CARE_PATHWAY_SEARCH_BOOT_REINDEX=0` opts out).
- New environment variables: `CARE_PATHWAY_SEARCH_INDEX_PATH`,
  `CARE_PATHWAY_SEARCH_BOOT_REINDEX`.
- `tantivy` is annotated in `compliance/soup.tsv` — the SOUP gate failed
  the build until it was, which is the IEC 62304 §5.3.3 machinery doing
  its job rather than an obstacle.

### Added — pagination on list and search (2026-08-01)

Follows the family convention fixed in `agents/share/restful.md` and
first implemented in organization.

- **`GET /api/care-pathways` and `GET /api/care-pathways/search` take `?limit=` and `?offset=`**
  and report `X-Total-Count` / `X-Limit` / `X-Offset`. The body stays a
  bare array, so no existing caller changes.
- Defaults reproduce the old hard caps (100 / 50), `limit`
  clamps to 500 rather than erroring, and an `offset` past 10 000 is a
  `400` (an unbounded offset makes the database materialise and discard
  arbitrarily many rows — SEC-G7).
- The search total is a `COUNT(*)` over the same predicate rather than
  the page length: a page cannot tell a caller how much there is, which
  is the point of the header.
- Pinned by a DB-gated request test walking a window, checking the total
  exceeds the page, the clamp, and the `400`.

### Added — key rotation and policy hot-reload without a restart (2026-08-01)

AU-2, the loco-style half of the rollout (case was the reference; the
five axum-style services landed the same day as AU-1).

- **The verifier and the ABAC policy are now reloadable holders**
  (`ReloadableVerifier` / `ReloadablePolicy`) that the blanket guard
  **and** the bearer extractors read per request. They were boot-only
  `OnceLock` snapshots, so a rotated key set or an edited policy could
  not have reached a running process at all.
- **`spawn_key_refresh`** re-fetches `CARE_PATHWAY_PASETO_KEYS_URL` every
  `CARE_PATHWAY_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a no-op
  when the URL is unset). A failed fetch **keeps the current key set** —
  a transient auth-service outage must not lock every caller out.
- **`spawn_policy_watcher`** polls `CARE_PATHWAY_ABAC_POLICY_FILE`'s mtime every
  15 s and calls `reload_policy()`; a malformed edit falls back to the
  built-in default rather than leaving the service unprotected.
- The existing `tests/enforcement.rs` activation proof already covered
  the guard, so it needed no change — it now exercises the reloadable
  holders by construction.
- New environment variable: `CARE_PATHWAY_PASETO_KEYS_REFRESH_SECS`.


### Added — row-level record integrity (2026-07-25)

- Every `care_pathways` row now carries a `content_hash` — SHA-256 over
  its `pid`, `name`, payload, `active` flag and `deleted_at` —
  recomputed on **every** write (migration
  `m20260726_000008_record_integrity`). Set inside the model write
  helpers and the erasure path, so no caller can omit it.
  `GET /api/compliance/records/verify` recomputes and names any row
  changed outside the service.
- This closes the gap the audit chain deliberately left: the chain
  attests to the **trail**, this attests to the **records**. Neither
  subsumes the other, and the remaining gap — a row deleted outright in
  SQL — is covered by the chain, because a legitimate delete writes to
  it.
- `created_at` / `updated_at` are excluded from the digest on purpose:
  they are ORM- and database-set, so binding them would produce false
  mismatches. Stated in spec §12.1 rather than left implicit.

### Changed — audit-write failure is now a deployment choice (2026-07-25)

- `CARE_PATHWAY_AUDIT_FAIL_CLOSED` (**default off**) decides what
  happens when a read-audit write fails: off logs and serves the read
  (previous behaviour); on refuses it with `503` on both the native and
  FHIR surfaces, disclosing nothing the service cannot account for.
- `record_access` returns `Result<(), AuditWriteRefused>`, so the choice
  is explicit at every call site instead of swallowed in the helper.
  Mutation audits were already fail-closed under the `outbox` transport.

### Added — regulatory compliance controls (2026-07-25)

The family's **reference implementation** of the four control-driving
frameworks in `agents/share/compliance-for-healthcare.md` §2 (spec §12,
entity spec §12.4). Migration `m20260725_000007_compliance` adds
`prev_hash` / `hash` / `context` / `disclosure` / `redacted_at` to
`audit_logs`, all nullable or defaulted, so existing rows stay valid.

- **HIPAA — tamper-evident audit history.** A SHA-256 hash chain over
  `audit_logs`: each row binds its own content and its predecessor's
  hash, so an insert, delete, reorder, or edit breaks verification
  (§164.312(c)). `GET /api/compliance/audit/verify` reports the counts,
  every break with its row id and kind, and the chain head. Appends are
  serialised with `pg_advisory_xact_lock`; under
  `CARE_PATHWAY_EVENT_TRANSPORT=memory` a concurrent-append fork is
  possible and is reported (and documented) rather than hidden.
- **HIPAA — read and disclosure auditing.** `CARE_PATHWAY_AUDIT_READS`
  (**default off**) audits reads, searches, exports, and FHIR reads,
  recording the caller's declared `X-Purpose-Of-Use` /
  `X-Disclosure-Recipient` / `X-Destination-Region` alongside the
  deployment's standing declarations.
  `GET /api/care-pathways/{pid}/audit/disclosures` is the §164.528
  accounting, and states whether it is complete or incomplete.
- **GDPR Art. 17 erasure that survives the chain.**
  `POST /api/care-pathways/{pid}/erase` tombstones the payload, redacts
  audit content, and appends a chained `erased` row — the chain still
  verifies, and the record that *something happened, when, and by whom*
  survives. Irreversible, idempotent, and **destructive** under ABAC.
- **GDPR / EHDS declarations.** Data residency, lawful basis, Art. 9(2)
  condition, and transfer safeguard default to `undeclared`, are
  reported at `GET /api/compliance`, and are stamped into every audit
  row. A cross-region export is recorded as a Ch. V transfer.
- **ONC / HTI conformance machinery.** A declared `meta.profile` on
  every rendered resource, must-support / cardinality validation, and
  **terminology validation against bound value sets** (ICD-10 / ICD-11 /
  SNOMED CT); `POST /fhir/PlanDefinition/$validate`; SMART discovery at
  `/fhir/.well-known/smart-configuration` (served only when a real
  authorization server is configured); an extended
  `CapabilityStatement`; and FHIR Bulk Data `$export` → status →
  NDJSON → cancel.
- **IEC 62304 / SaMD evidence.** `compliance/lifecycle.md` (safety
  classification + clause→artefact index), `compliance/soup.tsv` (the
  §8.1.2 SOUP register), a CycloneDX SBOM derived at compile time from
  the crate's own `Cargo.lock` (`GET /api/compliance/sbom`,
  `cargo run --bin sbom`), a machine-checked requirement→test
  traceability matrix (`compliance/traceability.tsv` +
  `tests/traceability.rs`), and `scripts/sbom.sh` /
  `scripts/build-reproducible.sh`.
- **`GET /api/compliance`** reports software identification, build
  provenance, the live control state, the data-protection declarations,
  and, per framework, what is **not** claimed — asserted by tests, so
  the report cannot quietly become marketing.

**Not claimed:** ONC certification (this serves FHIR R5; certification
targets R4 + US Core, and `PlanDefinition` has no US Core profile),
SMART App Launch itself (the credential is PASETO, not OAuth 2.0),
medical-device qualification, or an ISO 14971 risk file. See spec
§12.5.

### Changed (2026-07-25)

- `/fhir/metadata` and `/fhir/.well-known/smart-configuration` are now
  on the blanket-guard **public** allow-list: FHIR and SMART discovery
  must be reachable before a client holds a credential, and neither
  document exposes pathway data.
- `POST …/erase` joins `merge` / `deduplicate` / `import` in
  `auth::DESTRUCTIVE_POST_SUFFIXES`, so an `access=write` caller cannot
  reach an irreversible operation.
- FHIR create/update now validate against the declared profile and its
  terminology bindings in addition to the payload rules, so a code that
  is well-formed JSON but invalid in a bound system is a `422`.
- `validation::condition_code_issue` is a new public, index-free form of
  the existing per-code check, so the FHIR layer can report against a
  FHIR element path.

### Added — instance outcomes (2026-07-20)

- Recorded closure `outcome` on instances + an `instance_measures`
  table (clinical / PROM measures over time); a record-measure
  endpoint; and `GET /api/care-pathways/{pid}/outcomes` — the
  closed-instance outcome distribution + per-measure latest-value
  averages, derived only from what was recorded (migration
  `m20260720_000006_outcomes`).

### Added — instance layer (2026-07-20)

- An operational layer over the pathway registry: patients enrolled on
  a pathway template (`pathway_instances` + steps + care team +
  events; migration `m20260720_000005_instances`), with an
  active↔on_hold→terminal lifecycle, a review cadence, urgency
  escalation, step completion, and care-team assignments. Derived
  views: caseload by setting/urgency, the overdue-review register,
  care-team load, and the per-pathway chronic cohort. Instance state
  is never part of the matcher payload.

### Added — registry insight views (2026-07-20)

- Five read-only derived views (`controllers/insights.rs`): setting +
  `specialty:<x>` directory, condition-coverage gaps, cross-provider
  variants (with the `jurisdiction:<x>` facet), provider directory,
  and language coverage. No migration, no matcher change — facets from
  existing DTO fields + two disclosed keyword conventions.

### Fixed

- 2026-07-18 — **Order-dependent enforcement test** (QA-CP-FLAKE):
  `require_auth_gates_api_but_not_openapi` set
  `CARE_PATHWAY_REQUIRE_AUTH` inside the shared requests binary, but
  the flag's `OnceLock` was cached by whichever sibling test booted
  first — it only passed when it happened to run first. Moved to its
  own `tests/enforcement.rs` binary (the case / patient-flow
  pattern). Full DB-gated suite green vs Postgres 18.


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

- **SEC-G6: trailing slash can no longer downgrade a destructive POST.**
  `derive_action` classified `/merge` / `/deduplicate` / `/import` via
  `path.ends_with`, so a trailing slash (`POST …/merge/`) fell through to
  `Write` — a non-admin `access=write` caller could reach a destructive op.
  The path is now `trim_end_matches('/')`-normalised first. Test extended.

- **SEC-B6: relay claims outbox rows with `FOR UPDATE SKIP LOCKED`.** The
  Phase-3 relay drained via a plain unlocked `SELECT … WHERE published_at IS
  NULL`, so with more than one instance every relay would **double-ship** the
  same rows. `drain_once` now runs in a transaction and `unpublished` claims
  rows with `FOR UPDATE SKIP LOCKED` (a second relay skips locked rows; the
  lock releases on commit). Delivery stays at-least-once (consumers dedupe on
  `event_id`).

### Security — input-size caps on payload validation (SEC-M1) (2026-07-13)

- `src/validation.rs` now rejects oversized `CarePathway` payloads before
  they are stored or matched. The matcher runs O(n·m) Jaro-Winkler and
  Jaccard over the payload's text fields and arrays, so an unbounded
  string or array is a CPU/memory DoS — amplified by the
  `check-duplicates` scan over every stored record. New named caps
  enforced in the `problems` entrypoint (collecting *all* over-cap
  problems, surfaced as one `422`): `MAX_TEXT_LEN = 1024` per single
  free-text field (`name`, `pathway_code`, `provider_id`,
  `provider_name`; counted in Unicode scalar values), `MAX_ARRAY_LEN =
  256` per array (`alternate_names`, `condition_codes`, `interventions`,
  `keywords`, `identifiers`, `same_as`, `in_language`), and
  `MAX_ITEM_LEN = 512` per string entry inside an array (including
  `condition_codes[i].code` and `identifiers[i].value`). Messages such as
  `"name: exceeds 1024 characters"`, `"keywords: exceeds 256 entries"`,
  `"keywords[3]: exceeds 512 characters"`. All existing format checks are
  unchanged. New DB-free unit tests cover an oversized field, array,
  and array entry (one problem each) plus a large-but-within-caps record
  (zero problems).

### Changed — event bus: audit now joins the outbox transaction (2026-07-08)

- Under the `outbox` transport, the `audit_logs` write now rides the
  **same transaction** as the entity mutation and its `event_outbox` row
  (`agents/share/event-bus.md` §3 — the three "can never disagree"). It
  was previously a best-effort side channel written *after* the
  transaction committed, so a crash or audit failure could leave a
  committed change + event with no audit row. `AuditModel::record` is now
  generic over `ConnectionTrait`; the `create/update/delete/merge_and_emit`
  functions own the audit write (strict/in-txn under `outbox`, best-effort
  logged under `memory`), and the controllers no longer audit separately.
  New DB-gated `tests/outbox_audit.rs` drives `create_and_emit` under
  `outbox` and asserts entity + event + audit all commit together.
  (The `merge_records` history row stays a best-effort side channel — it
  is merge metadata, not the §3 audit trail.)

### Added — event bus: transactional-outbox storage (Phase 2 start) (2026-07-06)

- New `event_outbox` table (migration `…_000004_event_outbox`) + SeaORM
  entity + `models::event_outbox` — the durable hand-off buffer for the
  event bus (`agents/share/event-bus.md` §3). This crate is the family
  reference for the Phase-2 storage layer. Pieces:
  - `OutboxInsert::from_envelope` — the **pure** envelope→row mapping
    (pid parse, kind token, full-envelope JSONB payload, `occurred_at`
    stamp), DB-free unit-tested.
  - `Model::enqueue` — generic over `ConnectionTrait`, so a request
    handler can pass its own `&DatabaseTransaction` and give the entity
    write and the event one commit boundary.
  - `Model::unpublished` / `Model::mark_published` — the relay worker's
    poll (oldest-unpublished, id order) + ack (`published_at`).
  - Dedup unique index on `event_id`; partial index over unpublished rows.
  - Remaining (roadmap): the tx-aware `OutboxPublisher` behind the
    `EventPublisher` seam + handlers on an explicit transaction, then the
    Fluvio relay worker (Phase 3).

### Added — authz: ABAC policy authorization inside the blanket guard (2026-07-05)

- ABAC authorization landed (supersedes the earlier per-crate
  roles/RBAC sketch; family contract:
  `agents/share/authorization-attributes.md`). When
  `CARE_PATHWAY_REQUIRE_AUTH` is on, a verified PASETO token is
  further checked by the shared policy engine in
  `authentication-verifier` 0.3: the request's action is derived from
  the HTTP method plus the crate's destructive named POSTs
  (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`, `/deduplicate`,
  `/import`), and the policy is evaluated over the token's new `attrs`
  claim, first-match-wins, defaulting to allow-read / deny-mutation.
- New env vars `CARE_PATHWAY_ABAC_POLICY` (inline JSON) and
  `CARE_PATHWAY_ABAC_POLICY_FILE` (path); unset or unparsable ⇒
  `tracing::warn!` + the built-in default policy (`svc=true` ⇒
  everything; `access=admin` ⇒ destructive+write; `access=write` ⇒
  write) — the service always boots.
- `auth::enforce` now takes the HTTP method and the policy and returns
  `403` (deciding-rule reason) for a valid token the policy denies;
  `401` remains missing/bad credential. DB-free unit tests pin the
  family §7 matrix. Flag off ⇒ behaviour-neutral.

### Added

- **Boot-time PASETO key-set fetch over HTTP.** New env var
  `CARE_PATHWAY_PASETO_KEYS_URL`: when set, the service fetches the
  auth-service's published Ed25519 key set once at boot
  (`Verifier::from_paseto_keys_url`, `authentication-verifier` `fetch`
  feature) from `App::after_routes` via the new `auth::init_from_env`,
  seeding the process-wide verifier before serving. The fetched key set
  wins over `CARE_PATHWAY_PASETO_KEYS` (`tracing::info!`); any fetch
  failure logs a warning and falls back to the env key set, so the
  service always boots. Unset/blank URL keeps the prior env-injection
  behaviour exactly. Fetch-once only — a periodic refresh loop on key
  rotation is tracked as a future spec item (spec §16). Tests: a local
  ephemeral-port HTTP listener serving the test key set (the fetch-built
  verifier accepts a token signed by that key), a fast-failing-URL
  fallback pin (no panic), and a no-URL env-path pin. (Spec §9 auth
  section + §13 fetch follow-up.)

### Fixed

- **`cargo fmt` drift.** Reformatted `src/auth.rs` and
  `src/validation.rs` so `cargo fmt --check` passes again (no
  behavioural change).

### Changed

- **Auth pivot — sessions + PASETO (spec-level; code follow-up pending).**
  The family is moving off RS256 JWT + JWKS access tokens to server-side
  cookie sessions plus short-lived **PASETO v4.public** tokens verified
  offline against the authentication-service's published **Ed25519** key;
  the `authentication-verifier` becomes a PASETO verifier and RS256/JWKS
  is decommissioned. Front-ends adopt a BFF + httpOnly cookie + CSRF (the
  browser holds no token). The `CARE_PATHWAY_REQUIRE_AUTH` flag and
  blanket-enforcement semantics are unchanged — only the verified
  credential changes. Human-facing docs (README/agents/index) updated to
  describe the new model; runtime code follow-up is tracked in spec §13.
  Source of truth:
  [agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md).

### Documentation

- **Merge request-body field-name harmonization + worked examples.**
  Fixed the `README.md` Quick-start merge `curl` (was the unrecognized
  `survivor_pid`; now `main_pid`/`duplicate_pid`, matching the controller
  `MergeRequest` and the OpenAPI schema) and the `index.md` worked-flow
  merge row (was `{survivor_pid, dup_pid}`; now `{main_pid,
  duplicate_pid}`). Added a `README.md` multi-problem `422` example and
  an `Authorization: Bearer` / `whoami` example, and an `index.md`
  auth + `CARE_PATHWAY_REQUIRE_AUTH` note plus a cross-reference to the
  un-gated multi-dimension aggregation test. Reworded spec §15 so the
  roadmap reflects that all of the v0.1–v0.3 scope shipped together in
  the still-unreleased `0.1.0` line (the milestone split was never
  tagged).

### Tested

- **Self-merge `422` guard pinned DB-free.** Extracted the merge
  handler's equal-pid check into a pure `is_self_merge(main, dup)`
  predicate and added an un-gated unit test, so the §6.8 self-merge
  rejection holds on the default `cargo test` (previously covered only by
  the `#[ignore]`-gated `merge_with_equal_pids_is_422` request test).
- **Unknown-pid `404` on update + delete.** Added `#[ignore]`-gated
  request tests `update_unknown_pid_returns_404` and
  `delete_unknown_pid_returns_404`, closing the gap where only GET (and
  merge) had a `404` request test.
- **CI now runs the DB-backed request suite.** The `test` job gained a
  dedicated `cargo test --all-features --all -- --ignored` step against
  the already-provisioned Postgres service (the prior single step never
  passed `--ignored`, so every request-level test was silently skipped).
  Also removed a duplicate `- main` push branch in the workflow.

- **Doc harmonization pass (spec is the source of truth).** Refreshed
  the stale `README.md` Status section (now lists CRUD + `ILIKE` search +
  matching + merge + audit + in-memory streaming + OpenAPI/Swagger +
  Prometheus + offline JWT verification + blanket `/api/*` enforcement
  off-by-default as implemented, with only Tantivy full-text, durable
  event bus Phases 2–3, privacy, front-end merge action, and
  JWKS-over-HTTP fetch deferred) and the validation note (now covers
  ICD/SNOMED/UUID/DOI/BCP-47, all problems reported together). Corrected
  the `AGENTS.md` deferred list so blanket `/api/*` JWT enforcement is
  shown as implemented (off by default via `CARE_PATHWAY_REQUIRE_AUTH`)
  and only JWKS-over-HTTP fetch at boot remains deferred. Added a
  §6.12/§9 cross-reference for the `/metrics.prom` public path in the
  spec. Expanded `index.md`'s worked flow with merge / merges / audit /
  events / whoami / docs / metrics examples and a validation note.

### Tested

- **`validation::problems` multi-dimension aggregation pin.** Added a
  DB-free test asserting that a blank `name`, a malformed
  `condition_codes` entry, a malformed `identifiers` entry, and a
  malformed `in_language` tag each surface as a distinct problem in one
  call — pinning the §6.1 "all problems reported together" guarantee
  across every validated dimension at once.

### Added

- **Prometheus `/metrics.prom` endpoint.** A root-level
  `GET /metrics.prom` (Content-Type `text/plain; version=0.0.4`) for
  parity with the older Axum services. `src/metrics.rs` owns a
  process-wide `OnceLock<Metrics>` Prometheus `Registry` with four
  care-pathway counters (`care_pathway_created_total`,
  `_updated_total`, `_deleted_total`, `_merged_total`) plus an
  `http_requests_total` `IntCounterVec` (`method`, `path`, `status`);
  `Metrics::global()` and `Metrics::render()` (TextEncoder →
  text-exposition). The handler lives in `src/controllers/metrics.rs`
  and is mounted at the root via `App::routes` (mirroring
  `controllers/docs.rs`). The path is added to `auth::is_public_path`,
  so it stays open under blanket JWT enforcement (a scraper needs no
  token). The CRUD/merge controllers increment one counter per success
  path (create→created, update→updated, delete→deleted, merge→merged).
  New dependency `prometheus = "0.13"`. Un-gated tests: a DB-free
  `metrics` render test (every metric name + `# HELP`/`# TYPE` preamble +
  content type), an `auth::enforce` public-path test for `/metrics.prom`,
  and an `openapi` test for the documented `/metrics.prom` path.

- **Durable event bus — Phase 1 (in-memory envelope + `EventPublisher`
  seam).** `src/streaming.rs` now carries the canonical, versioned
  `Envelope` (`event_id` UUID dedup key, `schema_version` 1, `entity`
  `"care_pathway"`, `kind`, `pid`, `seq`, `actor`, `name`) and the
  `EventPublisher` trait, with an `InMemoryPublisher` ring buffer wired as
  the process-wide global — a pure refactor of the previous free
  functions. `occurred_at` / `data` are deferred to the outbox stage
  (Phase 2) per `agents/share/event-bus.md`; no new dependency added.
  `GET /api/care-pathways/events/recent` returns the frozen `EventView`
  projection (`{kind, pid, name, seq}`), **byte-identical** to the
  previous wire shape (the front-end recent-activity view depends on it).
  Added `publish_with_actor(kind, pid, name, actor)`; the CRUD/merge
  controller call sites now stamp the `actor` from the bearer token (the
  bare `publish` back-compat surface stays, actor `None`). Phases 2–3
  (transactional outbox → Fluvio) remain infra-gated roadmap. Un-gated
  tests: envelope Serde round-trip + `schema_version == 1`, `EventView`
  projects exactly `{kind, pid, name, seq}`, `InMemoryPublisher`
  publish→recent, `actor` populated/`None`, `seq` monotonic.

- **Blanket `/api/*` JWT enforcement (off by default).** A pure
  `auth::enforce(require_auth, path, headers, verifier)` decision plus an
  `axum::middleware::from_fn` layer wired unconditionally in `app.rs`
  `after_routes`. Gated per-request by `CARE_PATHWAY_REQUIRE_AUTH`
  (`auth::require_auth`, `OnceLock<bool>`; `1`/`true`/`yes`/`on` ⇒ on,
  anything else incl. unset ⇒ off). When on, every `/api/*` route needs a
  valid bearer token (`401` otherwise); the public paths `/_health`,
  `/_ping`, `/api-docs/openapi.json`, `/swagger-ui*` stay open. Default-off
  keeps existing behaviour and the DB-gated request suite green until an
  operator opts in. Un-gated `auth::tests` cover `parse_bool` and
  `enforce` (off/public/protected × no/valid/expired/tampered token); a
  `#[serial]` `#[ignore]` request test asserts `401` on `GET
  /api/care-pathways` and `200` on `GET /api-docs/openapi.json` with the
  flag set. Family contract: `agents/share/jwt-enforcement.md`.

### Changed

- **Validation failures now return `422 Unprocessable Entity`**
  (was `400`) for a blank `name`, on both create and update — the
  family convention (entity spec OQ-1 / T-2). Implemented as a shared
  controller `validate()` returning
  `Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`; pinned
  by DB-free unit tests.

### Added

- **`identifiers` and `in_language` payload validation** in
  `src/validation.rs`: each `identifiers` entry is structurally checked
  against its `scheme` — a canonical 8-4-4-4-12 hex UUID for `Uuid`, the
  `10.<registrant>/<suffix>` shape for `Doi`, and non-blank for every
  other scheme — and each `in_language` entry is checked for BCP-47
  syntax. A malformed entry joins the existing single `422` (all
  problems reported together). Rejecting a malformed *deterministic*
  identifier (UUID / DOI) matters because a shared value short-circuits
  the matcher to `1.0`. Pinned by 6 new DB-free `validation` unit tests
  and the DB-gated request test
  `malformed_identifier_on_create_returns_422`.

- Request-level integration tests
  (`tests/requests/care_pathways.rs`, loco testing harness) covering
  all seven endpoints: create, blank-name `422` on create/update,
  get-by-pid `200`/`404`, list, `/match`, and a stored near-duplicate
  `/check-duplicates` round-trip. `#[ignore]`-gated — they need a
  PostgreSQL `DATABASE_URL`; run with `cargo test -- --ignored`.

- **Inaugural scaffold (v0.1.0).** loco.rs clinical care-pathway
  registry.
  - Generated via `loco new` (loco-rs 0.16) and stripped of the auth
    starter (auth is centralized in the authentication-service).
  - `care_pathways` table (`pid`, denormalised `name`, full
    `CarePathway` payload as JSONB `data`, `active`, soft-delete) +
    `sea-orm-migration` migrator.
  - CRUD controller: create / list / get / update / soft-delete, plus
    `POST /match` and `POST /check-duplicates`.
  - **Embeds `care-pathway-matcher` directly**: the API DTO *is*
    `care_pathway_matcher::CarePathway`, stored verbatim and matched
    with the canonical engine — no separate model or adapter.
  - DB-free tests (`tests/matching.rs`): matcher embedding + JSON
    storage round-trip. Green `cargo build`, clippy clean.

### Notes

- MVP scope is CRUD + matching. Search, streaming, audit, privacy,
  OpenAPI, and richer validation are tracked in spec §13.
