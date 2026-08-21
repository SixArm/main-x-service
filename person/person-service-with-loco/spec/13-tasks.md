## 13. Tasks

- [x] **2026-08-21 — TSV bulk format + fuzzed row decoders.**
  `BulkFormat::Tsv` is accepted for import and export alongside `jsonl`,
  `csv`, and (export-only) `parquet`. TSV shares the CSV codec rather
  than forking it — the codec takes a `delimiter: u8` and
  `BulkFormat::delimiter()` is the one place mapping format → byte. The
  **streaming** import path (SEC-B2) threads the delimiter through
  `RowStream::new` as well, since that is what actually runs for an
  uploaded file; a TSV that only worked through the buffered `decode`
  would pass tests and fail in production, so there is a streaming TSV
  test specifically. The delimiter is declared by the caller, never
  inferred. A new `bulk_decoders` fuzz target drives both delimiters and
  the JSONL split/parse path over arbitrary bytes, pinning never-panic,
  decode determinism, and the §7 per-row error contract.

Spec-driven work breakdown. Each task has an acceptance criterion;
tick the box when an automated test or clearly described manual check
confirms the criterion is met. Tasks small enough to land in a single
PR; split larger tasks (`T-12a`, `T-12b`).

- [x] **BLK-2: CSV format wiring + keyless-row → duplicate-detection →
  review-queue routing.** `BulkFormat` gains `Csv`; `process_import_job` /
  `process_export_job` take a `format` param and dispatch to `jsonl` or
  `csv` (the worker reads `job.format`; stored artifact filenames carry
  the matching extension). A row with no strong identifier, no `tax_id`,
  and no explicit `id` (`stable_key::is_keyless`, plus the new
  `row_has_explicit_id` — needed because `Person::id` defaults to a fresh
  UUID on parse, so the parsed record alone can't tell "no id given" from
  "an id was given") runs the same search-blocking + matcher duplicate
  detection `POST /check-duplicates` uses; a likely duplicate
  (`IMPORT_REVIEW_THRESHOLD = 0.7`) still creates the row (never withhold
  legitimate data) and inserts a `review_queue` pair with the new
  `provenance = "import"` column (migration
  `2026080200000001_review_queue_provenance`; `operator` for existing
  interactive/batch-scan rows, never touched by a re-scan upsert). DB-gated
  tests: keyless-duplicate creates + queues for review (asserts the pair,
  its provenance, and its detection method), CSV import creates a keyed
  row, CSV export round-trips; extended the existing JSONL import/export
  suite for the new signatures. Green: `cargo test --lib` (304 pass),
  `cargo test -- --ignored` against a real Postgres, `clippy --all-targets
  -D warnings`, `cargo fmt --check`. (Repo tasks.md BLK-2.)

- [x] **BLK-3: Parquet export (feature-gated).** *(done 2026-08-02)*
  `format: "parquet"` on `POST /api/persons/export` — **export-only**
  (`BulkFormat::is_export_only`; the import handler refuses it) and
  **feature-gated** behind this crate's own `parquet` Cargo feature (off
  by default; `arrow` 59.1.0 + `parquet` 59.1.0, matched release line).
  `BulkFormat::parse` recognises the token regardless of build
  configuration; `process_export_job` returns a clean `422` rather than a
  silent JSONL fallback when the feature is off. The person §10.6 CSV
  column-flattening declaration was extracted into a new shared
  `src/bulk/columns.rs` (used by both `csv.rs` and the new
  `parquet_format.rs`, §10.7) so the two formats' column sets cannot
  drift apart. `Scalar`/`Bool` columns become nullable Arrow
  `Utf8`/`Boolean` (a real null for an absent field); `Json` columns
  (arrays/arrays-of-objects) become non-nullable `Utf8` carrying the same
  JSON text CSV puts in its cells. DB-free tests (readable-bytes
  round-trip via `parquet`'s own Arrow reader: row count, scalar column,
  JSON-cell column, a genuine null for an absent scalar; empty-slice;
  bool-column typing) plus a DB-gated export round-trip. Also closed
  IEC 62304 §5.3.3 SOUP annotations for the three new direct dependencies
  (`arrow`, `parquet`, the dev-only `bytes` needed to read Parquet bytes
  back in tests — `parquet::file::reader::ChunkReader` has no
  `std::io::Cursor` impl). Green: `cargo build`/`test`/`clippy --all-targets
  -D warnings`/`fmt --check`, each run **twice** — default features and
  `--features parquet` — plus the DB-gated suite against a real Postgres
  under both. **Known gap:** family CI does not pass `--features parquet`
  to this crate, so the feature is exercised only by local runs today
  (repo tasks.md BLK-3, spec §10.7 records this explicitly rather than
  silently).

- [x] **BLK-4: S3-compatible `ArtifactStore` (feature-gated).**
  *(done 2026-08-02)* Ported the care-pathway service's reference
  design (`agents/share/bulk-import-export.md` §12): `ArtifactStore`
  (`src/bulk/store.rs`) became `#[async_trait]` (`put`/`get`/
  `presigned_get`, the last defaulting to `None`) and gained
  `S3ArtifactStore` behind this crate's own `s3` Cargo feature (off by
  default; `aws-config`/`aws-sdk-s3`/`aws-credential-types` 1.x).
  `PERSON_BULK_ARTIFACT_BACKEND` selects `local` (default) or `s3`; an
  unknown value warns and falls back to local; `s3` without the feature
  is a clean error, never a silent local-storage fallback that would
  lose a deployment's export data. S3 config:
  `PERSON_BULK_S3_{BUCKET(required),ENDPOINT,REGION(default
  us-east-1),FORCE_PATH_STYLE(default on)}`; credentials from the
  standard AWS chain. `S3ArtifactStore::split_reference` refuses a
  reference naming a different bucket (IDOR guard); `presigned_get`
  clamps its TTL to `[1, 3600]` seconds. `AppState::new`
  (`src/api/rest/state.rs`) became `async fn … -> crate::Result<Self>`
  since the S3 client's credential resolution is async; both call sites
  (`app.rs`'s `after_routes`, `tests/common/mod.rs`) were already async.
  Test suite ported verbatim from care-pathway (local round-trip,
  missing-artifact error, no-presigned-URL-for-local, `is_safe_key`
  unit tests, unsafe-key rejection on `put`, outside-base rejection on
  `get`, without-the-feature error, unknown-backend fallback, the
  default/local backend names, foreign-bucket reference rejection,
  path-style default, and an `#[ignore]`d live-`MinIO` round-trip with
  its run command documented inline). SOUP register updated for the
  three new direct dependencies. Green:
  `cargo build`/`test`/`clippy --all-targets -D warnings`/`fmt --check`
  under default features, `--features s3`, `--features parquet`, and
  `--features s3,parquet`; `cargo deny check` shows the same
  pre-existing `rsa`/`jsonwebtoken`/loco-rs advisory as before (verified
  unrelated, not introduced here); the DB-gated suite
  (`scripts/ci-check.sh test-db`, 21+20+1 tests) against real Postgres,
  which exercises the real boot path through the now-async
  `AppState::new`. (Repo tasks.md BLK-4.)

- [x] **SEC-B10 (security): person merge audit in-transaction.** The merge
  `UPDATE` (survivor) + `DELETE` (duplicate) audit rows are now written on the
  merge transaction (`log_update_on`/`log_delete_on`) **before** commit, so a
  crash after commit can't lose them and an audit failure rolls the merge
  back (was best-effort post-commit). DB-gated test asserts both rows present.
  (Repo tasks.md SEC-B10.)

- [x] **SEC-B9 (security): wire the idempotency key.** Both submit handlers
  read an `Idempotency-Key` header; `create_or_get_idempotent` returns the
  original job (no re-store/re-enqueue) when the key already names one,
  backstopped by the `UNIQUE (entity, kind, idempotency_key)` constraint on
  the check-then-insert race. Blank key ⇒ absent; key-less ⇒ always creates.
  DB-gated same-key/keyless tests + pure key-trim test. (Repo tasks.md
  SEC-B9.)

- [x] **SEC-B8 (security): bulk audit gaps.** A successful import now writes
  a job-level `IMPORT` audit row (`log_import`) with the acting operator +
  reconciled counts; the export audit is written **before** the job finishes
  and its error **propagates**, so a failed audit blocks delivery (`failed`,
  no `download_url`). Actor threaded into both (fallback `system` only when
  no caller). Pure summary builders unit-tested. **Done (2026-08-05):**
  per-row audit actor threading — `PersonRepository::create`/`update`/
  `delete`/`merge` all now take an `&AuditContext` (no more hard-coded
  `AuditContext::default()` inside the repository); every request handler
  (REST `create_person`/`update_person`/`delete_person`/`merge_persons`,
  the FHIR `Patient` create/update/delete) builds it from the verified
  caller via the new `auth::audit_context_of` helper, and the bulk import
  pipeline threads the job's own actor (`bulk::worker::actor_audit_context`)
  into every per-row create/update it performs, including the keyless
  duplicate → review-queue path. `persons.created_by`/`updated_by`/
  `deleted_by` are stamped from the same actor (previously always `None`/
  hard-coded `"system"`). DB-gated tests assert the real actor — not
  `"system"` — lands on the `CREATE`/`UPDATE` audit rows for a direct
  repository call, a merge, and a bulk import row. (Repo tasks.md SEC-B8.)

- [x] **SEC-B3 (security): serialise bulk upsert (create-create race).**
  The per-row find→create/update now runs under a transaction-scoped
  advisory lock on the stable key (`pg_advisory_xact_lock(hashtext(key))`,
  `import_upsert_locked`), so two concurrent importers of the same key
  produce exactly one record (the second upserts the first's row). A
  `UNIQUE(system,value)` was rejected — the registry permits duplicate
  identifiers by design (dedup is a workflow). DB-gated concurrency test +
  pure lock-key test. (Repo tasks.md Phase 5 SEC-B3.)

- [x] **SEC-G8 (security): default-off exposure pin.** A named unit test
  pins that with `PERSON_REQUIRE_AUTH` off (the shipped default) the
  sensitive reads — a person's PII, GDPR export, audit trail, and
  `same_identity` links — are open without a token, so activation is a
  **tracked release gate** (see `agents/share/security.md` §4). (Repo
  tasks.md Phase 5 SEC-G8.)

- [x] **SEC-G3 (security): record-level read authz on `search_persons`.**
  The person search page was masked **only** by the client `mask_sensitive`
  query param, so a mask-only ABAC policy was defeated simply by omitting the
  param (and a deny had no effect) — the aggregate read revealed more than the
  equivalent single `GET /api/persons/{id}` (breaks `security.md` invariant
  #5). Search now runs `auth::read_visibility` (= `authorize_record(Read).ok()`,
  the case-service idiom) on every hit: a denied record is **omitted**
  (concealed, so its existence never leaks — not a whole-page `403`), and a
  `mask` obligation returns the masked view even when the client did not ask;
  the `mask_sensitive` convenience still masks on request. No-op when
  `PERSON_REQUIRE_AUTH` is off (pre-SEC-G3 behaviour preserved). The
  per-result decision is the pure `search_result_disposition`, unit-tested for
  the full omit/mask/full matrix. (Repo tasks.md Phase 5 SEC-G3.)

- [x] **SEC-G7 (security): bound the `search_persons` pagination offset.**
  `GET /api/persons/search` now rejects `offset > MAX_SEARCH_OFFSET` (10 000)
  with `400 OFFSET_TOO_LARGE` before asking the index for `offset + limit`
  hits (unbounded offset ⇒ CPU/memory `DoS`; the add could also overflow —
  now `saturating_add`). Pure `search_offset_within_bound` unit-tested +
  DB-gated `400` integration test. (Repo tasks.md Phase 5 SEC-G7.)

- [x] **SEC-M1 (security): input-size caps on the `Person` payload.**
  `validate_person` now bounds every scalar text field (`MAX_TEXT_LEN =
  1024`), string-array cardinality + per-entry length (`MAX_ARRAY_LEN = 256`
  / `MAX_ITEM_LEN = 512`), and the inner text + cardinality of the nested
  collections (names, `additional_names`, identifiers, addresses, telecom,
  documents, emergency_contacts, photo, tax_id, marital_status) → field-scoped
  `422` before persist/match, closing the O(n·m) matcher `DoS`. Factored into
  `person_size_caps`/`cap_*`. Unit tested. (Repo tasks.md Phase 5 SEC-M1.)

- [x] **SEC-B4 (security): bulk artifact hardening.** *(store confinement +
  IDOR + TTL done 2026-07-13; physical artifact deletion done 2026-08-05)*
  (1) `LocalFsArtifactStore` now **confines** `get` to the store's
  canonicalised base and validates keys with `is_safe_key` (no
  `..`/absolute), closing an arbitrary-file read via a crafted `file://`
  reference. (2) `GET /import|export/{id}` now returns `404` unless the
  caller **owns** the job (`is_job_owner`: `actor == sub`) or is elevated
  (`access=admin`/`svc=true`), closing an IDOR/BOLA on the status +
  download URL. (3) `create` stamps
  `expires_at = created_at + BULK_ARTIFACT_TTL_SECS` (7 days) and the
  status handler treats an expired job as `404` (`artifact_expired`).
  **Done 2026-08-05:** `ArtifactStore` gained a `delete` method
  (idempotent — an already-gone or never-written artifact is success, not
  an error), implemented for both backends (`LocalFsArtifactStore`,
  confined to the store base exactly like `get`; `S3ArtifactStore`, via
  `delete_object`, itself naturally idempotent). A new `src/bulk/sweep.rs`
  finds every `bulk_jobs` row past `expires_at` that has not yet been
  physically swept (`artifact_deleted_at IS NULL`), deletes its
  `input_url`/`result_url`/`error_report_url` artifacts, and stamps
  `artifact_deleted_at` so a swept row is never reprocessed. A per-job
  delete failure is logged and left unstamped for the next pass rather
  than failing the whole sweep. Migration
  `2026080500000001_bulk_jobs_artifact_deleted_at` adds the column plus a
  partial index (`WHERE artifact_deleted_at IS NULL`) matching the
  sweep's own query. Run via the new `bulk_artifact_sweep` loco task
  (`cargo loco task bulk_artifact_sweep op:apply`; report-only by default,
  mirroring `integrity_resign`'s dry-run posture) — this crate has no
  in-process periodic-timer convention, so scheduling is external (cron /
  a `CronJob` / a systemd timer), the same posture the family already
  takes for other operator-triggered maintenance. Pure `job_needs_sweep`
  eligibility logic and `delete`'s idempotency are unit tested; a
  DB-gated test (`bulk::sweep::db_tests`) proves an expired job's
  artifact bytes are physically gone after a sweep pass, a non-expired
  job's artifact survives, and re-running the sweep is a safe no-op (the
  swept row no longer qualifies). (Repo tasks.md Phase 5 SEC-B4.)

- [x] **SEC-B2 (security): bound bulk import/export against OOM.** *(caps
  + fuzz done 2026-07-13; end-to-end streaming done 2026-08-05)* The
  import upload is read chunk-by-chunk and rejected `413` past
  `MAX_IMPORT_BYTES` (64 MiB) before materialisation (`spool_field_capped`
  / `exceeds_cap`); the pipeline rejects a load over `MAX_IMPORT_ROWS`
  (1M); a caller `limit` is clamped to `MAX_EXPORT_ROWS` (1M) via
  `clamp_export_limit`. proptest fuzzes the JSONL parse boundary (never
  panics on random / truncated-UTF-8 / giant input).

  **The import read path is now streaming end to end** — no stage holds
  the file, as bytes or as rows. Upload chunks go straight to a
  `SpooledUpload` temp file and from there through
  `ArtifactStore::put_stream` into the store; the worker opens the
  artifact with `ArtifactStore::get_stream`; `jsonl::LineReader` frames
  rows from an `AsyncRead` with a carry buffer; `csv::RowStream` runs the
  `csv` reader on a blocking task fed by bounded channels; and
  `process_import_stream` consumes one row at a time, applying the
  unchanged validate → dedupe → SEC-B3 locked-upsert per row. The two
  full-file buffers that made this "bounded buffering" rather than
  streaming — the upload `Vec<u8>` and the decoded `Vec<ImportRow>` — are
  gone. **Both caps were kept** (see `bulk::MAX_IMPORT_BYTES` /
  `MAX_IMPORT_ROWS` for the reasoning: the byte cap is now a work/storage
  ceiling rather than a memory one, and the row cap is now observed at the
  row that crosses it rather than before the first row), and a new
  `MAX_IMPORT_ROW_BYTES` (4 MiB) bounds a single JSONL row so one
  enormous unterminated line cannot grow the carry buffer to the whole
  file. `tests/bulk_streaming_memory.rs` **measures** the claim with a
  counting global allocator: ~0.19 MiB peak streaming ~312 MiB of JSONL
  (identical to the peak for a tenth of it), ~0.49 MiB for CSV, against
  ~32.7 MiB for the same rows through the old whole-buffer shape.
  (Repo tasks.md Phase 5 SEC-B2.)

- [x] **SEC-B5 (security): reject self-merge + lock merge participants.**
  `POST /merge` now rejects `main == duplicate` with `422` before any
  fetch (a self-merge tombstoned the record and lost its data);
  integration test `test_merge_into_self_is_rejected`. The repository
  `merge` transaction also locks both participant rows `FOR UPDATE`
  (id-ordered) and re-checks the duplicate is still active, closing the
  concurrent-merge TOCTOU. (Repo tasks.md Phase 5 SEC-B5.)

- [x] **T-1a — Flip peer verification to PASETO v4.public.** *(done
  2026-07-04)* Per
  [authentication-sessions.md](../../../agents/share/authentication-sessions.md)
  §5/§9: the family moved off RS256-JWT + JWKS.
  - [x] `authentication-verifier` 0.2 (path dep; PASETO-only) replaces
    the crates.io 0.1 RS256 version; direct `jsonwebtoken` dep dropped.
  - [x] [`AuthUser`] extractor + `GET /api/whoami` verify PASETO
    `v4.public` (Ed25519) bearer tokens offline — signature, footer
    `kid`, `iss`, `aud`, `exp` — via `bearer_claims` in
    `src/api/rest/auth.rs`.
  - [x] Verifier built from env at boot (`PERSON_PASETO_KEYS` key set as
    published at `/.well-known/paseto-keys`; `PERSON_TOKEN_ISSUER` /
    `PERSON_TOKEN_AUDIENCE`, defaults `authentication-service` /
    `main-x-service`); absent key set ⇒ empty set, every token rejected,
    service still boots.
  - **Acceptance:** DB-free unit tests in `src/api/rest/auth.rs` mint
    `v4.public` tokens in-process (throwaway Ed25519 key) and pin
    valid / missing / non-bearer / expired / tampered / no-key
    outcomes. Met: `cargo test --lib` green.
- [x] **T-1b — Blanket auth enforcement on `/api/*`.** *(done
  2026-07-04; remainders split to T-1c)*
  - [x] Require a valid PASETO bearer token on every route except the
    public allow-list (`/api/health`, loco `/_health` / `/_ping`,
    `/api-docs/openapi.json`, `/swagger-ui*`, `/metrics.prom`), gated
    by a default-off `PERSON_REQUIRE_AUTH` env flag with lenient
    parsing (`1`/`true`/`yes`/`on` ⇒ on; unset/blank/junk ⇒ off;
    family contract: `agents/share/jwt-enforcement.md`). Pure
    `auth::enforce` decision + `Enforcement` middleware state in
    `src/api/rest/auth.rs`, layered unconditionally on **both** router
    surfaces (`create_router` and the loco `after_routes` hook); the
    flag is snapshotted at router construction, so changing it
    requires a restart.
  - **Acceptance:** DB-free unit tests in `src/api/rest/auth.rs`
    (reusing the T-1a in-process token minting) pin the full
    enforcement matrix — off + no token ⇒ Ok; on + each public path ⇒
    Ok; on + protected + no token ⇒ `401`; on + protected + valid ⇒
    Ok; on + expired/tampered ⇒ `401` — plus the flag-parser
    semantics. Met: `cargo test --lib` green.
- [x] **T-1c — Auth follow-ups: boot-time key fetch + authorization.**
  *(fully done — the last open item below was closed by AU-1, 2026-08-01)*
  - [x] Fetch the key set over HTTP from the auth service at boot
    *(done 2026-07-04)*: new `PERSON_PASETO_KEYS_URL` env var —
    unset/blank ⇒ the `PERSON_PASETO_KEYS` env path exactly as before;
    set ⇒ fetch once at boot in `after_routes` via
    `Verifier::from_paseto_keys_url` (verifier `fetch` feature); on
    success the fetched key set **wins** over `PERSON_PASETO_KEYS`
    (`tracing::info!`); on any fetch failure `tracing::warn!` and fall
    back to the env path — the service **always boots**. Swapped into
    `AppState` via `with_verifier` **before** the enforcement
    middleware and shared-store state are built, so both router
    surfaces verify against it. Pinned by DB-free tokio
    tests in `src/api/rest/auth.rs`: fetch from a local ephemeral-port
    listener serving the in-process key set (minted token verifies),
    fallback on a dead port (no panic, token rejected), and the
    URL-unset ⇒ env-path precedence. **Superseded** by AU-1 (below,
    2026-08-01): the one-shot-fetch-only limitation this bullet
    originally noted ("no refresh loop") no longer holds — a
    process-wide `ReloadableVerifier` now re-fetches on
    `PERSON_PASETO_KEYS_REFRESH_SECS`, so a key rotation at the auth
    service is picked up without a restart.
  - [x] ABAC authorization *(done 2026-07-05; supersedes the earlier
    roles/RBAC-on-`roles`/`scope` sketch, per
    [authorization-attributes](../../../agents/share/authorization-attributes.md))*
    — inside the blanket guard (so only when `PERSON_REQUIRE_AUTH` is
    on), a verified token's `attrs` claim is evaluated by the shared
    engine in `authentication-verifier` 0.3: the action is derived
    from the HTTP method + this crate's destructive named POSTs
    (`auth::DESTRUCTIVE_POST_SUFFIXES`: `/merge`, `/deduplicate`,
    `/import`), and the policy — `PERSON_ABAC_POLICY` (inline JSON) /
    `PERSON_ABAC_POLICY_FILE` (path), unset/unparsable ⇒ warn-log +
    built-in default policy, read once at router construction —
    decides first-match-wins with default allow-read / deny-mutation.
    `401` = missing/bad credential; `403` = valid credential, policy
    denied (body carries the deciding rule). Acceptance met: DB-free
    unit tests in `src/api/rest/auth.rs` pin the §7 matrix — action
    derivation; empty `attrs` ⇒ GET ok / POST 403; `access=write` ⇒
    POST/PUT ok, DELETE + merge 403; `access=admin` ⇒ destructive ok;
    `svc=true` ⇒ everything; configured deny beats later allow;
    401-vs-403 split; bad policy JSON falls back to the default —
    `cargo test --lib` green. **Superseded** by AU-1 (below,
    2026-08-01): the policy is now a `ReloadablePolicy` an operator can
    edit on disk and have take effect without a restart.
  - [x] DB-gated request test (`#[ignore]`, Postgres): with
    `PERSON_REQUIRE_AUTH` set, an unauthenticated `GET /api/persons/…`
    returns `401` while `GET /api-docs/openapi.json` stays `200`.
    *(done 2026-08-01 via AU-1's `tests/enforcement.rs`, below — its own
    binary because the auth `OnceLock`s are process-wide: public paths
    stay open, a protected read/write without a token is `401`, a
    malformed bearer is `401` not `500`, a valid token with no
    attributes reads `200`/writes `403`, `access=write` creates, and
    forcing the flag off is asserted to fail the "protected" case.)*
  - **Acceptance:** valid token whose attributes satisfy the policy
    gets `2xx`; a valid token the policy denies gets `403`; no/bad
    token gets `401`. Key-set fetch from a stub auth service at boot,
    and the DB-gated request test, are both met (`cargo test --lib`
    green; `tests/enforcement.rs` green against Postgres). Activation
    (`PERSON_REQUIRE_AUTH=1`) remains the operational decision.
- [x] **AU-1 — PASETO key rotation and ABAC policy hot-reload without a
  restart.** *(done 2026-08-01; this crate is the axum-style reference,
  case-service the loco-style one)* Closes the "changing
  `PERSON_PASETO_KEYS_URL`/`PERSON_ABAC_POLICY_FILE` requires a restart"
  gap T-1a/T-1c/T-1b originally shipped with.
  - [x] The PASETO verifier moved out of `AppState` into a process-wide
    `ReloadableVerifier` read per request by both the blanket guard and
    the `AuthUser`/`MaybeAuthUser` extractors (previously two
    independent `Arc<Verifier>` snapshots that a rotation could only
    ever update one of).
  - [x] `spawn_key_refresh` re-fetches `PERSON_PASETO_KEYS_URL` every
    `PERSON_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables;
    no-op when the URL is unset) and swaps the result in. A failed
    fetch **keeps the current key set** (a transient auth-service
    outage must not lock every caller out).
  - [x] `auth::policy()` is a `ReloadablePolicy`; `reload_policy()` +
    `spawn_policy_watcher` poll `PERSON_ABAC_POLICY_FILE`'s mtime every
    15s. A malformed edit falls back to the built-in default rather
    than leaving the service unprotected.
  - [x] New `tests/enforcement.rs` — the T-1c DB-gated activation proof
    (above).
  - **Acceptance:** met — new env var `PERSON_PASETO_KEYS_REFRESH_SECS`;
    `tests/enforcement.rs` green against Postgres 18.
- [x] **T-2 — Production Fluvio publisher.** *(superseded/done via BUS-3,
  2026-08-03 — see the "Durable event bus real broker (`FluvioSink`,
  BUS-3)" entry below)* `FluvioSink : EventSink` (the Phase-3 relay's
  broker seam, not literally an `EventProducer` impl as this task
  originally sketched — the relay/outbox architecture superseded that
  shape back in T-20/T-21) lives behind the `fluvio` Cargo feature,
  off by default. Failover is documented: an endpoint configured
  without the feature refuses to start the relay (logged `error`)
  rather than silently falling back to `LoggingSink`.
  - [x] Implement the real-broker sink behind feature flag `fluvio`.
  - [x] Document failover behaviour when the broker is unreachable.
  - **Acceptance (met differently than drafted):** no local-Fluvio-broker
    CI integration test exists — `tests/fluvio_relay.rs` is a
    feature-gated `#[ignore]`d live-broker round-trip verified by
    compiling under `--features fluvio`, matching the family-wide
    precedent (BLK-4's `s3_round_trip_against_a_live_endpoint`,
    case-service's BUS-1 reference); no automated stage in this repo
    stands up a broker.
- [x] **T-3 — Complete FHIR bundle handling.** *(done via T-11, below,
  2026-07-07)* `GET /fhir/Patient` / `GET /fhir/Person` already return a
  `searchset` `Bundle` (`src/api/fhir/bundle.rs`, `src/api/fhir/handlers.rs`);
  every non-2xx FHIR response is a `FhirOperationOutcome`. `POST` takes a
  bare resource body (not itself Bundle-wrapped), matching the family
  contract (`agents/share/fhir.md` §4) rather than this task's original
  "Bundle POST" phrasing.
  - [x] `Bundle` GET / search wrapping.
  - [x] OperationOutcome on malformed bundles.
  - **Acceptance (met differently than drafted):** no Touchstone FHIR
    validator run exists in this repo; correctness is pinned instead by
    T-11's DB-free unit tests (round-trip, missing-name rejection,
    metadata/CapabilityStatement-matches-routes).
- [x] **T-4 — FHIR capability statement endpoint.** *(done via T-11,
  below, 2026-07-07)* `GET /fhir/metadata` returns a `CapabilityStatement`
  (fhirVersion `5.0.0`) listing the `Patient` interactions
  (read/create/update/delete/search-type) and the nine supported search
  params; a unit test pins that it matches the mounted routes.
  - [x] `GET /fhir/metadata` returns a CapabilityStatement listing
    supported resources + interactions.
  - **Acceptance (met differently than drafted):** pinned by a unit test
    against the mounted routes rather than an external R5 schema
    validator.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [ ] **T-6 — gRPC implementation.**
  - [ ] Promote stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `PersonService.GetPerson`
    round-trips a record.
- [ ] **T-7 — Spec-drift CI check.**
  - [ ] Fail PR if `src/matching/**` or `src/models/person.rs`
    changes without a `spec.md` edit (allowlist in `.spec-allow`).
  - **Acceptance:** `bash scripts/spec-drift-check.sh main HEAD`
    exits non-zero on a code-only PR.
- [x] **T-8 — `db::audit` rename clean-up.** *(done 2026-06-15)*
  - [x] Verify no -era symbols remain in `src/db/audit.rs`.
  - **Acceptance:** `cargo check --lib` passes clean; legacy
    domain-specific symbols (e.g. `patient`, `mpi`) absent from
    `src/db/`. Met: a grep of `src/db/` finds zero `patient` / `mpi`
    symbols and `cargo check --lib` is clean.
- [x] **T-9 — Cross-service entity links (write side).** *(complete
  2026-07-15)* See §5.4, §8.6, §9.1, §10.4 and
  [cross-service linking](../../../agents/share/cross-service-linking.md).
  **`same_identity` landed 2026-07-10 (T-22); `works_at` / `member_of`
  affiliations 2026-07-14 (LNK-3); `linked`/`unlinked` events 2026-07-14
  (LNK-1); partition guard 2026-07-15.**
  - [x] Migration creating the `entity_links` table (§10.4 schema, with
    the `UNIQUE (from_pid, kind, to_ref, valid_from)` upsert key).
  - [x] `EntityRef` value type (parse / `Display` + `entity_type → service`
    map) — used via the shared `entity-ref` crate.
  - [x] Link endpoints: `POST` / `GET` / `DELETE`
    `/api/persons/{pid}/links`; create/upsert is optimistic (no
    cross-service call) and supports `same_identity` (person ↔ worker)
    and `works_at` / `member_of` (person → organization, temporal). LNK-3
    extended `validate_edge`'s permit set from `same_identity`-only to
    `{same_identity, works_at, member_of}`, relying on `EdgeKind::permits`
    for the endpoint check; accept/reject matrix unit-tested.
  - [x] Emit `linked` / `unlinked` events on the existing event
    envelope (LNK-1, 2026-07-14): `EventKind` gained `Linked`/`Unlinked` and
    `Envelope` an additive `data` field (`skip_serializing_if` — the CRUD
    wire shape stays byte-identical) carrying the §4.2 edge detail. Under
    `outbox` the edge upsert + its `linked`/`unlinked` envelope commit in one
    transaction (the outbox guarantee); under `memory` the in-memory
    `PersonEvent::Linked`/`Unlinked` is published (lossy dev signal). Unit
    tests pin the tokens, the frozen CRUD shape, and the `for_link` data
    shape; a DB-gated `linked_event_is_enqueued_to_the_outbox` pins the
    transactional enqueue.
  - [x] Partition guard (`agents/share/cross-service-linking.md` §7):
    cross-service links are never a matcher signal. Enforced structurally —
    `entity_links` live in their own table, never a field on the domain
    `Person`, so they cannot reach `to_matcher_person`; and the adapter also
    ignores the within-entity `Person.links`. Regression-guarded by the
    bridge test `links_are_not_a_matcher_signal` (adding link data does not
    move the match score).
  - **Acceptance:** the `validate_edge` accept/reject matrix (incl. the
    affiliation cases), the `linked`/`unlinked` emission (envelope + emit
    tests), and the matcher-partition guard are all tested (green).
- [x] **T-10 — Bulk import / export.** *(all five rollout steps done:
  1 & 3 on 2026-07-10; step 2 on 2026-08-02 as BLK-2 (below); step 4 on
  2026-08-02 as BLK-3 (above); step 5 on 2026-08-02 as BLK-4 (above) —
  this task's own Step 2/4/5 checkboxes went stale because the
  completing work landed under the repo `tasks.md` BLK-2/3/4 labels
  without a pass back through this file; found and reconciled during
  the 2026-08-04 documentation audit)* Person is the family **reference
  entity** for this capability. See §9.2, §10.5 and
  [bulk import/export](../../../agents/share/bulk-import-export.md).
  - [x] **Step 1 (JSONL reference core).**
    - [x] Migration `m20260710_000002_create_bulk_jobs` — `bulk_jobs`
      table (shared doc §3 schema, `UNIQUE (entity, kind,
      idempotency_key)` + `(kind, status, created_at)` index);
      registered. SeaORM entity `db::models::bulk_jobs`; persistence
      `db::bulk_jobs` (`create`, `set_input_url`, `set_status`,
      `finish_import`, `finish_export`, `find_by_id`, `list_recent`).
    - [x] The five endpoints (`bulk::handlers`, mounted on
      `persons_routes`, in OpenAPI): `POST /api/persons/import`
      (multipart, `202 {job_id}`, `dry_run`), `POST /api/persons/export`
      (JSON filter, `202`), `GET /api/persons/import/{id}` +
      `GET /api/persons/export/{id}` (status + counts +
      `errors_url`/`download_url`), `GET /api/persons/bulk-jobs`.
    - [x] `bg_pg` worker `bulk::worker::BulkJobWorker` (registered in
      `connect_workers`) draining `queued → running →
      completed | completed_with_errors | failed`; a thin adapter over
      the pure-ish `bulk::pipeline`.
    - [x] JSONL codec (`bulk::jsonl`, the lossless reference — person
      wire type per line, streaming). Artifact store abstraction
      (`bulk::store::ArtifactStore` + `LocalFsArtifactStore`,
      `PERSON_BULK_ARTIFACT_DIR`; S3 = deployment, deferred).
    - [x] **Stable key** (§10.1, `bulk::stable_key`): a strong
      scheme-scoped identifier (SSN/TAX/NPI/PPN) → `tax_id` → record
      `pid`. Per-row pipeline reuses the single-create validators;
      upsert-in-place on a stable-key match (idempotent re-import), else
      create; events + audit not bypassed (via the repository).
    - [x] Downloadable per-row error report
      (`row_number, field, code, message`; `bulk::error_report` → CSV);
      one bad row never aborts the load; counts reconcile
      (`rows_total = created + upserted + errored`; `to_review` = 0 until
      step 2).
    - [x] Export honours the person list/search filter and writes an
      export audit row (even zero-row).
    - **Acceptance:** DB-free unit (JSONL round-trip, stable-key
      precedence, error-report shape, store round-trip, enum
      round-trips) + DB-gated `#[ignore]` pipeline tests (create → idempotent
      re-upsert with error report; dry-run commits nothing; export JSONL
      round-trip). Met: `cargo test --lib` green (182 passed, 6 ignored);
      `cargo build`, `cargo clippy --all-targets --all-features`, and the
      migration clippy all clean (0).
  - [x] **Step 2** — CSV codec (flattening per §9.2: dotted single-nested,
    JSON-in-cell arrays) + keyless/unmatched rows → duplicate detection →
    review queue with `provenance = import`. *(done 2026-08-02 as
    **BLK-2**, above — see that entry for the full acceptance detail.)*
  - [x] **Step 3** — export masking + gating *(done 2026-07-10)*:
    `bulk::MaskingProfile` (`masked` default / `full`); `ExportParams`
    gains `masking_profile` + `include_soft_deleted`. `process_export_job`
    masks every record via `privacy::mask_person` under the default
    `Masked` profile (a default export never reveals more than the masked
    read view), returns the row count for the audit, and **rejects**
    `include_soft_deleted=true` as `Error::Validation` (not-yet-supported
    — the repository cannot list soft-deleted rows without a larger
    change, so the flag is refused, never leaked/ignored). The
    `POST /api/persons/export` handler accepts `masking_profile` (default
    `masked`; unknown ⇒ `400`) and `include_soft_deleted` (default
    `false`) and gates the **privileged** paths (`full` OR
    `include_soft_deleted`) behind elevated authorisation via
    `auth::authorize_record` (destructive action; no-op when
    `PERSON_REQUIRE_AUTH` is off, else `403` unless `access=admin` /
    `svc=true`); the default masked, active-only export stays open to any
    authorised caller. Per-export audit (`audit_export` →
    `AuditLogRepository::log_export`, `EXPORT` action) records actor,
    filter (`q`/`limit`/`offset`), format, masking profile,
    `include_soft_deleted`, and row count — even for a zero-row export.
    **Acceptance met:** DB-free unit tests (masking applied for `Masked` /
    skipped for `Full`; the privileged-path gate decision;
    `MaskingProfile` round-trip) + DB-gated `#[ignore]` tests (default
    export ⇒ masked JSONL + `EXPORT` audit row; `Full` ⇒ unmasked;
    `include_soft_deleted=true` rejected). `cargo test --lib` green (185
    passed, 8 ignored); `cargo build`, `cargo clippy --all-targets
    --all-features`, migration clippy all clean (0). **Deferred:** a real
    soft-deleted-record export query, and folding the single-record GDPR
    export into the `filter = one pid` special case.
  - [x] **Step 4** — Parquet **export-only**, feature-gated. *(done
    2026-08-02 as **BLK-3**, above.)*
  - [x] **Step 5** — S3-compatible artifact store; roll the contract to
    the other entities. *(the artifact-store half done 2026-08-02 as
    **BLK-4**, above, for this crate; "roll the contract to the other
    entities" is tracked family-wide, not per-crate — see
    `agents/share/bulk-import-export.md` §11 rollout step 6, which
    landed organization + case on 2026-08-03 without an S3 backend for
    either.)*
- [x] **T-11 — FHIR R5 API** (`Patient` primary + `Person` alias) — adopt
  the family contract *(done 2026-07-07)*. **Done:** reconciled the
  unmounted `src/api/fhir/` prototype to the standard — `resourceType`
  flipped from non-standard `"Person"` to **`"Patient"`** (primary;
  `to_fhir_patient`) with a thin `/fhir/Person` demographic **alias**
  (`to_fhir_person`, same fields, `resourceType: "Person"`). Routes are
  **mounted** on both router surfaces (loco `after_routes` via
  `fhir::handlers::routes()` in `App::routes()`, and the hand-written
  `create_router` via `fhir_router(state)`), under the blanket
  auth+ABAC guard (`/fhir/*` not on the public allow-list; action from
  HTTP method). Surface: `GET/POST /fhir/Patient`,
  `GET/PUT/DELETE /fhir/Patient/{id}`, `GET /fhir/Person{,/{id}}` alias,
  `GET /fhir/metadata` (`CapabilityStatement`, fhirVersion 5.0.0,
  Patient interactions read/create/update/delete/search-type + the nine
  search params). Every non-2xx body is a `FhirOperationOutcome`; all
  responses are `application/fhir+json`. Writes reuse the repository
  (audit + events fire) and keep the Tantivy index in sync. 6 new
  DB-free unit tests (`to_fhir` ⇒ `Patient`, alias ⇒ `Person`,
  core-field round-trip, missing-name rejected, render selects type,
  metadata/CapabilityStatement matches routes); `cargo test --lib` green
  (139), `cargo clippy --lib` clean. **Gap:** PHI masked-read is not yet
  driven by ABAC masking obligations — FHIR reads return the full
  resource, consistent with the native default `GET /api/persons/{id}`
  (masking stays opt-in via the separate `/masked` endpoint); wiring
  `authorize_record`-style obligations into FHIR reads is deferred. The
  original detailed acceptance list follows.

  Original contract:
  the family contract
  ([`agents/share/fhir.md`](../../../agents/share/fhir.md)).
  **Reconcile the existing unmounted `src/api/fhir/` prototype**: switch
  the non-standard `resourceType: "Person"` to standard **`Patient`**
  (§3, `high` fidelity), keep a thin `/fhir/Person` alias endpoint for the
  demographic view, and **mount the routes** (the prototype defines
  handlers but wires none). Map the domain `Person` to `Patient`:
  `name`/`additional_names` → `name`, `gender` → `gender`, `birth_date` →
  `birthDate`, `deceased`/`deceased_datetime` → `deceased[x]`, `addresses`
  → `address`, `telecom` → `telecom`, `identifiers` → `identifier` (token
  `system|value`), `marital_status` → `maritalStatus`, `multiple_birth` →
  `multipleBirth[x]`, `managing_organization` → `managingOrganization`,
  `links` → `link`; `active`. Add `FhirOperationOutcome` errors (§5),
  searchset `Bundle` (§6), and `GET /fhir/metadata` `CapabilityStatement`
  (§7). FHIR routes join the existing Axum router under the blanket
  auth+ABAC guard (§8; `/fhir/*` guarded, action derived from HTTP method)
  and honour **masked reads** for PHI (§8). Supported search params:
  `_id`, `_lastUpdated`, `_count`, `identifier`, `name`, `family`,
  `given`, `birthdate`, `gender`.
  - **Acceptance:** tests cover domain↔`Patient` round-trip, each
    interaction, search→Bundle, `OperationOutcome` on 404/400/422, the
    `CapabilityStatement` matching the mounted routes, and masked-read.


- [x] **T-20 — Durable event bus Phase 2 (transactional outbox).** *(done
  2026-07-08)* Per [event-bus.md](../../../agents/share/event-bus.md)
  §3/§5, closes the "DB committed, event lost" crash window by writing
  one `event_outbox` row **inside each write's transaction**. Additive
  and behaviour-neutral until activated: gated on `PERSON_EVENT_TRANSPORT`
  (`memory`, the default, keeps today's post-commit in-memory publish;
  `outbox` also enqueues the durable row). The relay worker
  (Phase 3) is now delivered (T-21); a real Fluvio sink landed
  2026-08-03 (BUS-3, below).
  - [x] `event_outbox` migration (`BIGSERIAL id`, unique `event_id`,
    `entity`/`entity_pid`/`kind`/`occurred_at`/`actor`/`schema_version`/
    JSONB `payload`/`published_at`; partial `WHERE published_at IS NULL`
    index) + its SeaORM entity (`db::models::event_outbox`).
  - [x] `db::outbox::OutboxInsert` — pure `from_envelope` /
    `for_event` / `for_merge` (DB-free), `insert_on(&impl
    ConnectionTrait)` (so the repo threads its **own** transaction), and
    the relay `recent` / `unpublished` / `mark_published` poll+ack.
  - [x] `streaming::Envelope` (canonical §4 shape; `entity: &'static
    str` with `#[serde(skip_deserializing, default)]`, `merged_from`,
    `for_merge`) + `EventTransport` / `transport()` reading
    `PERSON_EVENT_TRANSPORT`.
  - [x] Repository: a `transport` field + `enqueue_outbox<C:
    ConnectionTrait>`, integrated **inside** each write's transaction for
    `create`/`update`/`delete`; a new `merge(survivor, duplicate_id)`
    that in **one** transaction applies the survivor update, soft-deletes
    the duplicate, and enqueues a `Merged` (+`merged_from`) row for the
    survivor and a `Deleted` row for the duplicate. The `/api/persons/merge`
    handler calls `repository.merge(...)` (dropping the old
    update+delete+separate-Merged-publish).
  - **Config:** `PERSON_EVENT_TRANSPORT` (`memory` | `outbox`, default
    `memory`); `PERSON_EVENT_RETENTION_DAYS` (outbox row TTL, default
    `7`, enforced by the Phase-3 relay — T-21).
  - **Acceptance:** DB-free unit tests pin the pure `from_envelope`
    column mapping, `for_merge` (kind=`merged` + `merged_from`), and
    transport parsing; a DB-gated `#[ignore]` test asserts `create` and
    `merge` write the entity rows + the right outbox rows in one
    transaction. Met: `cargo test --lib` green (157 passed, 2 ignored);
    `cargo clippy --lib --tests` clean.


- [x] **T-21 — Durable event bus Phase 3 (outbox relay + retention).**
  *(done 2026-07-08)* Per [event-bus.md](../../../agents/share/event-bus.md)
  §5/§6, the background relay that drains unpublished `event_outbox` rows
  to the durable bus and enforces retention. Copy-adapted from the
  `organization-service` reference (`src/relay.rs`).
  - [x] `src/relay.rs`: the `EventSink` trait (the broker seam) +
    `LoggingSink` (default no-broker sink), `drain_once` (poll
    `Model::unpublished` → `EventSink::send` → `Model::mark_published`,
    at-least-once, stop-on-first-error to keep per-pid order),
    `purge_published` (delete published rows older than
    `PERSON_EVENT_RETENTION_DAYS`), the config parsers, and `spawn`.
  - [x] Wired: `pub mod relay;` in `lib.rs`; `crate::relay::spawn(ctx.db
    .clone())` in `app.rs::after_routes`, gated internally on
    transport=`outbox` **and** `PERSON_EVENT_RELAY`, so the default
    (`memory`) boot is unchanged (no relay loop).
  - **Config:** `PERSON_EVENT_RELAY` (truthy to run the loop, default
    off); `PERSON_EVENT_RELAY_INTERVAL_SECS` (poll interval, default `5`,
    floored at `1`); `PERSON_EVENT_RETENTION_DAYS` (now enforced,
    default `7`).
  - **Delivered (broker-gated):** a real `FluvioSink` `impl EventSink`
    behind the `fluvio` cargo feature landed 2026-08-03 — see BUS-3
    below.
  - **Acceptance:** three DB-free unit tests (logging sink never fails;
    capturing sink records `(entity, key)`; config defaults). Met:
    `cargo test --lib` green (160 passed, 2 ignored); `cargo clippy
    --lib --tests` clean. Default (no `PERSON_EVENT_RELAY`) ⇒ no relay
    loop, behaviour unchanged.


- [x] **T-22 — Cross-service links: `same_identity` write side.**
  *(done 2026-07-10)* Per
  [cross-service-linking.md](../../../agents/share/cross-service-linking.md)
  §4.1/§4.2/§9 (rollout step 2 — the backbone edge), person is the
  reference originator of the `same_identity` (person ↔ worker) edge;
  worker's symmetric side is the follow-up.
  - [x] Migration `m20260710_000001_create_entity_links` — `entity_links`
    table (§4.1 schema) with the idempotent-upsert
    `UNIQUE(from_pid, kind, to_ref, valid_from) NULLS NOT DISTINCT` index
    and the `from_pid` active index; registered in the migrator.
  - [x] SeaORM entity `db::models::entity_links`; persistence
    `db::entity_links` (`upsert` — idempotent, revives a soft-deleted
    row; `list_active`; `find_active`; `list_all_active(since)`;
    `soft_delete`). Depends on the shared `entity-ref` crate.
  - [x] `api::rest::links`: `validate_edge` (DB-free — accepts only
    `same_identity` person → worker), the operator `LinkView` and the
    canonical §4.2 `EdgeDetail`, and the handlers `create_link` /
    `list_links` / `delete_link` / **`bulk_links`**
    (`GET /api/persons/links[?since=]` → `{ "edges": [EdgeDetail…] }`),
    mounted on both router surfaces. Writes gated at the person
    record-level (`authorize_record`) and audited (`person_link`).
  - **Deferred:** cross-service `linked`/`unlinked` **event** emission —
    the durable `Envelope` has no link kind / `data` and the in-memory
    `PersonEvent::Linked` carries only person `Uuid`s, so neither carries
    the §4.2 edge `data` without a cross-cutting refactor; the bulk
    endpoint is the aggregator's sync path (§8).
  - **Acceptance:** six DB-free `validate_edge` unit tests (accept
    `same_identity` person→worker; reject `subject_of`,
    `same_identity`→non-worker, non-`same_identity` kind, malformed ref,
    unknown kind) + a DB-gated `#[ignore]` round-trip (upsert →
    idempotent re-upsert → bulk-list asserts the canonical
    `edge_id`/`edge_kind`/`from_ref=person:<id>` shape → soft-delete).
    Met: `cargo test --lib` green (166 passed, 3 ignored); `cargo build`
    and `cargo clippy --all-targets --all-features` clean (0).

- [x] **2026-07-19 — Stored review queue + decision endpoints.** Persist
  the batch-dedup candidates (`review_queue` migration + the shared
  raw-SQL `db/review_queue` module: normalized-pair upsert / list /
  first-writer-wins decide), report stored rows from the scan, and add
  `GET /api/persons/review-queue` + `POST
  /api/persons/review-queue/{id}/decision`. Front-end `/review` board
  loads the stored queue on mount and drag records decisions.
  **Acceptance:** serde pins for the decision wire tokens; the person
  crate's env-gated DB round-trip (`tests/review_queue_db.rs` — the
  module is byte-identical family-wide) green against Postgres 18;
  `cargo test --lib` + clippy pedantic clean; FE svelte-check / vitest /
  Playwright green.

- [x] **2026-08-03 — Durable event bus real broker (`FluvioSink`, BUS-3).**
  Per [event-bus.md](../../../agents/share/event-bus.md) §5/§8, the
  Phase-3 relay's real-broker sink, ported from case-service's BUS-1
  reference implementation. `FluvioSink` is a second `impl EventSink`
  alongside the existing `LoggingSink` (T-21), behind a new `fluvio`
  Cargo feature (off by default, so a default build's dependency tree
  and behaviour are unchanged). One producer per topic
  (`fluvio::Fluvio::connect_with_config` + `topic_producer`, held for
  the sink's lifetime), partitioned on the record `pid` per §7.
  - [x] `src/relay.rs`: `FluvioSink::connect(endpoint, topic)` +
    `impl EventSink`; `fluvio_endpoint()` (`PERSON_FLUVIO_ENDPOINT`) and
    `event_topic()` (`PERSON_EVENT_TOPIC`, default `mxi.person.events`);
    `build_sink` (feature-gated both ways so the call site is uniform)
    and `run_drain_loop` extracted so `spawn` selects between sinks.
  - [x] `spawn()` selects `FluvioSink` over `LoggingSink` when
    `PERSON_FLUVIO_ENDPOINT` is configured. An endpoint set **without**
    the `fluvio` feature refuses to start the relay (logged at `error`)
    rather than silently falling back to `LoggingSink` — that fallback
    would mark outbox rows `published_at` without ever reaching a real
    broker, the same silent-data-loss shape the bulk artifact-store's
    "no fallback on an explicit backend choice" rule exists to prevent
    (`agents/share/bulk-import-export.md` §12). The initial connection
    retries indefinitely for the same reason.
  - [x] `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a
    local SC+SPU broker (Fluvio's own documented Docker Compose layout,
    translated to this repo's Podman conventions; container names
    prefixed `mxi-person-fluvio-`, host ports offset +100 from
    case-service's so both crates' composes can run side by side).
    Neither is run by any automated stage in this repo.
  - [x] `tests/fluvio_relay.rs`: a `#![cfg(feature = "fluvio")]`-gated,
    `#[ignore]`d live-broker round-trip, run command documented inline —
    verified by compiling under `--features fluvio` (confirming correct
    API usage), not by an actual execution, matching the precedent
    already set by this crate's own
    `s3_round_trip_against_a_live_endpoint` (BLK-4) and by
    case-service's `fluvio_relay.rs` (BUS-1). Builds the outbox row
    directly via `db::outbox::OutboxInsert::for_event` +
    `tests/common::db()` rather than through a `create_and_emit`-style
    helper, since this crate's write path enqueues the outbox row
    inside `PersonRepository::create`/`update`/`delete` and has no
    single-call equivalent — a deliberate deviation from the
    case-service reference test's shape, documented inline in the test
    file's module docs.
  - [x] SOUP register (`compliance/soup.tsv`) updated.
  - **Acceptance:** `cargo build --lib` clean under default features and
    `--features fluvio`; `cargo clippy --all-targets -- -D warnings`
    clean both ways; `cargo fmt --check` clean; `cargo test --lib` green
    under both (311 passed, 21 ignored — same count both ways, the
    feature only adds compiled-out-by-default code plus one `#[ignore]`d
    integration test); `cargo deny check` shows only the same
    pre-existing `RUSTSEC-2023-0071` (rsa, via `jsonwebtoken` →
    `loco-rs`) advisory before and after, not a new one from `fluvio`'s
    tree; the full DB-gated suite reruns clean against real Postgres,
    zero regressions.


- [x] **PERSON-CONTACT-CASE — Fix `merge`/`use_type`/`telecom` writes
  rejected by the database.** *(done 2026-08-04; found writing TUT-2 in
  the repo `tasks.md`, whose premise depends on merge working)* Every
  merge of two *different* persons failed `500 DATABASE_ERROR` on the
  `patient_names_use_type_check` constraint: `src/db/repositories.rs`
  wrote `NameUse`/`IdentifierUse`/`ContactPointSystem`/
  `ContactPointUse`/`LinkType` via `format!("{:?}")` (`"Old"`,
  `"Phone"`, `"Replaces"`) into columns whose CHECK constraints accept
  only lowercase tags — no test caught it because no prior test posted
  a name/identifier with `use_type` set, and the only merge test was
  the self-merge rejection guard (SEC-B5), which exits before the
  insert.
  - [x] Write side switched to the pre-existing `enum_to_tag` helper
    (already correct for `person_addresses`/emergency-contact tables);
    read side switched to `tag_to_enum` instead of hand-rolled
    `PascalCase` match arms. `identifier_type`'s `Other` variant (Debug
    `"Other"`, CHECK `'OTHER'`) fixed the same way.
  - [x] `NameUse` and `LinkType` gained `PartialEq, Eq` for the
    regression test's assertions.
  - [x] New DB-gated
    `test_merge_two_persons_round_trips_alias_name_and_replaces_link`
    (`tests/api_integration_test.rs`) merges two real persons and
    re-fetches the survivor, pinning both the write (insert succeeds)
    and read (stored lowercase tags deserialize to the right enum
    variants) sides together.
  - **Residual, narrower gap, not fixed here:** `LinkType::ReplacedBy`'s
    `#[serde(rename_all = "lowercase")]` produces `"replacedby"`, not
    the CHECK's `'replaced_by'` — nothing in this crate constructs that
    variant today, so it is tracked but not blocking.
  - **Acceptance:** the new DB-gated test is green against Postgres 18
    (`scripts/ci-check.sh test-db`); `cargo test --lib` unaffected.

- [x] **EX-4 — `seed_examples` CLI task.** *(done 2026-08-04)*
  `cargo loco task seed_examples` loads the repository's shared demo
  fixture (`examples/data/persons.jsonl`, 50 rows including five
  deliberate duplicate pairs) into the `persons` table, for the
  tutorials (repo `tasks.md` EX-4).
  - [x] Inserts via the **model-layer create**
    (`db::repositories::SeaOrmPersonRepository::create`) rather than
    `POST /api/persons`, deliberately bypassing real-time duplicate
    detection — the normal create endpoint returns `409` on the second
    half of every duplicate pair (confirmed live by EX-1), which would
    silently drop half the fixture.
  - [x] No audit row or event is written by the seed itself; the
    tutorials that exercise duplicate detection, audit, and events do
    so against the seeded records afterward.
  - [x] Refuses to insert into a non-empty `persons` table (prints a
    message, exits cleanly), so a second run is a no-op.
  - [x] New `src/tasks/seed_examples.rs` (`parse_fixture`, `seed`,
    `SeedExamples`); DB-free unit tests parse the real fixture.
  - **Acceptance:** DB-gated `tests/seed_examples_db.rs` proves a first
    run seeds all 50 rows (including both halves of the
    "Okonkwo/Okonkow" duplicate pair) and a second run changes nothing;
    green against Postgres 18.

- [x] **T-32 (link-graph-service-with-loco LNK-4) — Cross-service
  `same_identity` review-queue bridge + promotion/rejection.** *(done
  2026-08-04)* The link-graph aggregator's periodic suggestion job
  (T-31, that crate's `spec/13-tasks.md`) already POSTs `matcher_suggested`
  `same_identity` edges to `create_link`, but that handler never touched
  `review_queue` — this task closes that gap on the person side, per
  [cross-service-linking.md](../../../agents/share/cross-service-linking.md)
  §5.2 and link-graph's `spec/16-open-questions.md` OQ-9(b).
  - [x] `LinkRequest` gained an optional `score_breakdown` field (only
    meaningful for `kind = "same_identity"` +
    `provenance = "matcher_suggested"`); the link-graph job now sends
    its T-29 `IdentityMatchScore` mapped to JSON there.
  - [x] `create_link` (`src/api/rest/links.rs`), after a successful
    `same_identity`/`matcher_suggested` edge write, best-effort writes a
    `review_queue` row via a new `db::review_queue::upsert_cross_service`
    — deliberately **not** `upsert`, whose pair-order normalization
    (correct for within-entity dedup, where the two ids are
    interchangeable) would silently swap the person/worker columns for
    roughly half of all pairs. `record_id_a` is always the person pid,
    `record_id_b` always the worker pid, stored exactly as given.
    `detection_method = "cross_service_same_identity"`,
    `match_quality` from a newly-extracted
    `models::review_queue::match_quality_for_score` (also now reused by
    the batch-dedup scan and `check_duplicates`, removing three copies
    of the same inlined `if`/`else`).
  - [x] **Design decision — where the write happens:** in person's own
    `create_link` handler, not a second call from link-graph's
    suggestion job. Keeps the write local to the service that owns
    `review_queue` (no other service writes into it today), keeps the
    aggregator's job exactly as simple as T-31 left it (one POST, no
    follow-up call whose failure would leave an edge with no queued
    review), and matches "the originating service owns the write"
    (design §4) extended to this service's own derived state.
  - [x] `review_decision`'s `confirmed` branch now also calls a new
    `links::promote_cross_service_link` (reasserts the edge via the
    same `upsert_and_emit` `create_link` itself uses —
    `provenance="operator", confidence=1.0` — idempotent on
    `entity_links`'s own `(from_pid, kind, to_ref, valid_from)` key, so
    it is the SAME edge id, not a second edge) and the `rejected`
    branch calls a new `links::reject_cross_service_link`
    (soft-delete + `unlinked`, via a new
    `entity_links::find_active_by_key` natural-key lookup, since a
    review row carries no `edge_id`). Both are gated on
    **both** `row.provenance == "matcher_suggested"` **and**
    `row.detection_method == "cross_service_same_identity"`, so an
    ordinary within-entity decision is completely unaffected — pinned
    by a DB-gated regression test, not merely asserted.
  - [x] New `tests/cross_service_link_review.rs` (DB-gated, 4 tests,
    real HTTP router via `tower::ServiceExt::oneshot`, exactly like
    `tests/api_integration_test.rs`): a suggestion POST produces the
    right review-queue row (fields + score breakdown); confirming
    promotes the same edge (not a duplicate); rejecting withdraws it;
    an ordinary `provenance="operator"`/`detection_method="batch_deduplication"`
    decision writes no `entity_links` edge at all (the regression pin).
  - **Acceptance:** `cargo fmt --check` / `cargo clippy --all-targets --
    -D warnings` clean; `cargo test --lib` 315 passed (was 314; +1 for
    `match_quality_for_score`'s boundary test); DB-gated suite green
    against Postgres 18 via `scripts/ci-check.sh test-db` (21 `--lib`
    DB tests + 25 `api_integration_test` + the 4 new
    `cross_service_link_review` tests, all passing; the pre-existing
    suites unaffected).
  - **Follow-up landed (T-33, link-graph, 2026-08-04):** "the
    suggestion job audits every POST it makes" turned out to already be
    true of `create_link`'s existing unconditional `person_link` audit
    write (no `provenance` special-case) — investigated rather than
    assumed, and pinned by a new
    `tests/cross_service_link_review.rs::matcher_suggested_link_creation_is_audited`
    DB-gated regression test rather than left as an unverified claim.
    No `src/` change was needed. `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` /
    `_MAX_EDGES_PER_RUN` scale controls, the durable `suggestion_runs`
    audit table, and the never-auto-promoted-regardless-of-score
    governance test all live entirely on the link-graph side — see that
    crate's own `spec/13-tasks.md` T-33 entry.
