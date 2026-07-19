## 13. Tasks

Spec-driven work breakdown. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [x] **SEC-M1 (security): input-size caps on the `Worker` payload.**
  `validate_worker` bounds scalar text (`MAX_TEXT_LEN = 1024`), string-array
  cardinality + per-entry (`MAX_ARRAY_LEN = 256` / `MAX_ITEM_LEN = 512`), and
  the inner text + cardinality of the nested collections (names,
  `additional_names`, identifiers, addresses, telecom, documents,
  emergency_contacts, photo, tax_id, marital_status) → field-scoped `422`
  before persist/match, closing the O(n·m) matcher `DoS`. Factored into
  `worker_size_caps`/`cap_*`. Unit tested. (Repo tasks.md Phase 5 SEC-M1.)

- [x] **T-1a — Offline PASETO v4.public peer verification.** *(done
  2026-07-04)* Per
  [authentication-sessions](../../../agents/share/authentication-sessions.md)
  §5/§9: the family moved off RS256-JWT + JWKS. Ported from the
  person-service T-1a implementation.
  - [x] `authentication-verifier` 0.2 (path dep; PASETO-only) added.
  - [x] `AuthUser` extractor + `GET /api/whoami` verify PASETO
    `v4.public` (Ed25519) bearer tokens offline — signature, footer
    `kid`, `iss`, `aud`, `exp` — via `bearer_claims` in
    `src/api/rest/auth.rs`.
  - [x] Verifier built from env at boot (`WORKER_PASETO_KEYS` key set as
    published at `/.well-known/paseto-keys`; `WORKER_TOKEN_ISSUER` /
    `WORKER_TOKEN_AUDIENCE`, defaults `authentication-service` /
    `main-x-service`); absent key set ⇒ empty set, every token rejected,
    service still boots.
  - **Acceptance:** DB-free unit tests in `src/api/rest/auth.rs` mint
    `v4.public` tokens in-process (throwaway Ed25519 key) and pin
    valid / missing / non-bearer / expired / tampered / no-key
    outcomes. Met: `cargo test --lib` green.
- [ ] **T-1b — Blanket auth enforcement on `/api/*`.**
  - [x] *(done 2026-07-04)* Require a valid PASETO `v4.public` bearer
    token on every route except the public allow-list, gated by the
    default-off `WORKER_REQUIRE_AUTH` env flag (family contract:
    `agents/share/jwt-enforcement.md`; lenient parse: `1`/`true`/`yes`/
    `on` ⇒ on, unset/blank/`0`/junk ⇒ off). Pure `enforce(...)`
    decision + `require_auth_from_env()` in `src/api/rest/auth.rs`,
    layered via `apply_enforcement` on **both** router surfaces
    (`create_router` and the loco router in `App::after_routes`); the
    flag and verifier are captured at router construction (restart to
    change). Public allow-list (`PUBLIC_PATHS` +
    `PUBLIC_PATH_PREFIXES`): `/_health`, `/_ping`, `/api/health`,
    `/api-docs/openapi.json`, `/metrics.prom`, `/swagger-ui*`. The
    `/fhir` surface is deliberately protected (worker PII).
    **Acceptance met:** DB-free unit tests in `src/api/rest/auth.rs`
    pin the family test matrix — off+no-token ⇒ Ok, on+public ⇒ Ok,
    on+protected+no-token ⇒ 401, on+protected+valid ⇒ Ok,
    on+expired/tampered ⇒ 401, plus the flag-parser test —
    `cargo test --lib` green.
  - [x] ABAC authorization *(done 2026-07-05; supersedes the earlier
    RBAC roles sketch — HR-admin / credentialing-officer / read-only /
    service — per
    [authorization-attributes](../../../agents/share/authorization-attributes.md))*
    — inside the blanket guard (so only when `WORKER_REQUIRE_AUTH` is
    on), a verified token's `attrs` claim is evaluated by the shared
    engine in `authentication-verifier` 0.3: the action is derived
    from the HTTP method + this crate's destructive named POSTs
    (`auth::DESTRUCTIVE_POST_SUFFIXES`: `/merge`, `/deduplicate`,
    `/import`), and the policy — `WORKER_ABAC_POLICY` (inline JSON) /
    `WORKER_ABAC_POLICY_FILE` (path), unset/unparsable ⇒ warn-log +
    built-in default policy, read once at router construction —
    decides first-match-wins with default allow-read / deny-mutation.
    `401` = missing/bad credential; `403` = valid credential, policy
    denied (body carries the deciding rule). Acceptance met: DB-free
    unit tests in `src/api/rest/auth.rs` pin the §7 matrix — action
    derivation; empty `attrs` ⇒ GET ok / POST 403; `access=write` ⇒
    POST/PUT ok, DELETE + merge 403; `access=admin` ⇒ destructive ok;
    `svc=true` ⇒ everything; configured deny beats later allow;
    401-vs-403 split; bad policy JSON falls back to the default —
    `cargo test --lib` green.
  - [x] *(done 2026-07-04)* Fetch the key set over HTTP from the auth
    service at boot: new `WORKER_PASETO_KEYS_URL` env var —
    unset/blank ⇒ the `WORKER_PASETO_KEYS` env path exactly as
    before; set ⇒ fetch once at boot in `App::after_routes` via
    `Verifier::from_paseto_keys_url` (verifier `fetch` feature); on
    success the fetched key set **wins** over `WORKER_PASETO_KEYS`
    (`tracing::info!`); on any fetch failure `tracing::warn!` and
    fall back to the env path — the service **always boots**.
    Swapped into `AppState` via `with_verifier` **before** the
    enforcement middleware and shared-store state are built, so both
    router surfaces verify against it. One-shot fetch; no refresh
    loop (periodic refresh is a §15 roadmap note). Pinned by DB-free
    tokio tests in `src/api/rest/auth.rs`: fetch from a local
    ephemeral-port listener serving the in-process key set (minted
    token verifies), fallback on a dead port (no panic, token
    rejected), and the URL-unset ⇒ env-path precedence.
  - **Acceptance (met):** valid token whose attributes satisfy the
    policy gets `2xx`; a valid token the policy denies gets `403`;
    no/bad token gets `401`. T-1b is complete; activation
    (`WORKER_REQUIRE_AUTH=1`) remains the operational decision.
- [ ] **T-2 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag `fluvio`.
  - **Acceptance:** integration test publishes a `WorkerCreated`
    record end-to-end against a local Fluvio broker.
- [ ] **T-3 — FHIR capability statement + bundle handling.**
  - [ ] `GET /fhir/metadata` returns a CapabilityStatement listing
    the `Worker` resource (the wire `resourceType` this server
    actually emits — see §6.8).
  - [ ] `Bundle` GET / POST / search wrapping.
  - **Acceptance:** Touchstone FHIR validator passes on a sample
    bundle round-trip.
- [ ] **T-4 — FHIR Organization resource.**
  - [ ] Bidirectional Organization mapping.
  - **Acceptance:** `POST /fhir/Organization` round-trips a record.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [ ] **T-6 — gRPC implementation.**
  - [ ] Promote the stub to a working Tonic server mirroring REST CRUD.
  - **Acceptance:** `grpcurl` against `WorkerService.GetWorker`
    round-trips a record.
- [ ] **T-7 — Credential-expiry warning workflow.**
  - [ ] Background scan: `IdentityDocument.expiry_date` within 30
    days → publish `CredentialExpiringSoon` event.
  - [ ] Custom metric `credential_expiry_within_30d`.
  - **Acceptance:** integration test seeding a credential with
    `expiry_date = today + 25d` produces the event + metric.
- [ ] **T-8 — Role + assignment history timeline.**
  - [ ] Per-worker timeline of role / organisation assignments.
  - **Acceptance:** new assignment creates a timeline entry visible
    via `GET /api/workers/{id}/timeline`.
- [x] **T-9 — Mount the FHIR routes on the loco router.** *(Done
  2026-06-13.)*
  - [x] The `/fhir/Worker` handlers in `src/api/fhir/handlers.rs` are
    now registered: `App::routes` adds `workers_routes()` +
    `fhir_routes()` + `metrics_routes()`, and `create_router` mirrors
    the `/fhir/Worker` surface for the integration-test harness. The
    new `fhir_routes()` `Routes` group serves GET/POST `/fhir/Worker`
    and GET/PUT/DELETE `/fhir/Worker/{id}`; its handlers extract
    `AppState` via `FromRef` exactly like the REST surface.
  - [x] **Acceptance:** the mount is pinned by two route tests in
    `tests/api_integration_test.rs` — `test_fhir_worker_route_is_mounted`
    (un-gated; a malformed UUID makes the `Path<Uuid>` extractor return
    `400`, proving the route matched a handler rather than a route-level
    `404`) and `test_fhir_worker_not_found_returns_operation_outcome`
    (DB-gated; a valid-but-absent id returns a FHIR `OperationOutcome`
    `404`). Closes entity-level task T-1.
- [~] **T-10 — Cross-service entity links (write side).** Implements
  domain model §5.4, architecture §8.6, API §9.1, persistence §10.3 —
  per [cross-service linking](../../../agents/share/cross-service-linking.md)
  (rollout §11 step 2, the `same_identity` backbone + `employed_by`).
  **`same_identity` write-side landed 2026-07-14 (LNK-2)** — mirrors the
  person reference (`same_identity` **worker → person**, the inverse
  direction); the bulk endpoint is the sync reconciliation path (design §8),
  event emission deferred as on person. `employed_by` is LNK-3.
  - [x] Migration adding the `entity_links` table (§10.3 schema) with the
    `UNIQUE (from_pid, kind, to_ref, valid_from) NULLS NOT DISTINCT`
    idempotent-upsert key (`migrations/2026071000000001_create_entity_links`).
  - [x] Depend on the shared `entity-ref` crate (`EntityRef` `parse` /
    `Display` + `entity_type → service` map + the §9 edge-kind registry —
    used, not copied); `validate_edge` accepts `same_identity` worker →
    person and (LNK-3, 2026-07-14) `employed_by` worker → organization,
    rejecting any other kind / wrong target / malformed `to_ref` (pure,
    unit-tested matrix). The `role` field carries the job title on an
    `employed_by` edge.
  - [x] `POST` / `GET` / `DELETE /api/workers/{pid}/links` controllers
    (`src/api/rest/links.rs`: optimistic upsert / list / soft-delete;
    **no** cross-service call) + the governed bulk reconciliation pull
    `GET /api/workers/links` (SEC-G1 `Action::Destructive`), both router
    surfaces. Record-level authz (`authorize_record`) + best-effort audit
    (`worker_link` / `worker_links_bulk` via the new `log_export`).
  - [x] Emit `linked` / `unlinked` on the existing event envelope (LNK-1,
    2026-07-14, mirroring person): `EventKind` gained `Linked`/`Unlinked`
    and `Envelope` an additive `data` field (`skip_serializing_if` — the
    CRUD wire shape stays byte-identical) carrying the §4.2 edge detail.
    Under `outbox` the edge mutation + its `linked`/`unlinked` envelope
    commit in one transaction (the outbox guarantee); under `memory` the
    in-memory `WorkerEvent::Linked`/`Unlinked` is published (lossy dev
    signal). Unit tests pin the tokens, the frozen CRUD shape, and the
    `for_link` data shape; a DB-gated `linked_event_is_enqueued_to_the_outbox`
    pins the transactional enqueue.
  - [x] **Matcher-adapter partition guard** (partition rule, §5.1 /
    `cross-service-linking.md` §7): cross-service links are never a matcher
    signal — `entity_links` are never a field on the domain `Worker` (so
    they cannot reach `to_matcher_worker`), and the adapter also ignores the
    within-entity `Worker.links`. Regression-guarded by the bridge test
    `links_are_not_a_matcher_signal` (adding link data does not move the
    match score).
  - **Acceptance:** `validate_edge` accept/reject matrix + the SEC-G1
    `governed_bulk_read_is_classified_destructive` classification + the
    `linked`/`unlinked` emission (envelope + emit tests) + the
    matcher-partition guard are unit-tested (green); a DB-gated
    `round_trip_upsert_bulk_list_delete` + `linked_event_is_enqueued_to_the_outbox`
    pin the DB paths. Remaining: `employed_by` end-to-end integration test.
- [ ] **T-11 — Bulk import / export.** Implements persistence §10.4 —
  per [bulk import / export](../../../agents/share/bulk-import-export.md)
  (the uniform contract; only Worker's stable keys + CSV columns +
  export sensitivity differ, §10).
  - [ ] Migration adding the `bulk_jobs` table (shared §3 schema) with the
    `UNIQUE (entity, kind, idempotency_key)` retried-submit key.
  - [ ] The five endpoints (shared §4): `POST` / `GET
    /api/workers/import`, `POST` / `GET /api/workers/export`, and
    `GET /api/workers/bulk-jobs` (list + by-id).
  - [ ] `bg_pg` background worker draining `queued → running →
    completed | completed_with_errors | failed` with progress updates.
  - [ ] JSONL (lossless reference) + CSV (the §10.4 column set /
    flattening) codecs; Parquet **export-first, feature-gated** (import
    is roadmap).
  - [ ] Per-row import pipeline reusing the **single-create validators +
    worker-matcher + review queue** verbatim: parse + validate →
    upsert-by-stable-key (person-level identifier `(identifier_type,
    system, value)` / `tax_id` / `pid`, §10.4) → else duplicate detection
    → likely-duplicate to review queue with `provenance = import`, else
    create. Emit the normal `WorkerCreated` / `WorkerUpdated` event +
    audit record per row (no bulk bypass).
  - [ ] Downloadable per-row **error report** (`row_number, source_line,
    field, code, message`); one bad row never aborts the load; final
    counts reconcile (`rows_total = created + upserted + to_review +
    errored`).
  - [ ] Export **masking + audit**: `masking_profile` (masked default;
    full/unmasked gated to HR-admin / credentialing-officer),
    `include_soft_deleted` gated, scoped by the existing list/search
    filter; **every export audited** (actor, filter, format, count,
    profile) — even a zero-row export.
  - **Acceptance:** integration tests cover idempotent re-import (same
    file → same state, no duplicates), per-row error report (one invalid
    row skipped + reported, valid rows commit), keyless-row dedupe → review
    queue with `provenance = import`, masked vs full export (full requires
    elevated role), and that an export — including a zero-row export —
    writes an audit record.
- [x] **T-12 — FHIR R5 API** (`Practitioner`) — adopt the family contract.
  *(Done 2026-07-07.)* Reconciled the prototype: `resourceType` switched
  `Worker` → **`Practitioner`** (`FhirWorker::new`), routes re-pointed
  `/fhir/Worker*` → `/fhir/Practitioner{,/{id}}` on **both** router
  surfaces (loco `fhir_routes()` + the `create_router` test harness), and
  `GET /fhir/metadata` added returning a `CapabilityStatement` (fhirVersion
  5.0.0, Practitioner resource, read/create/update/delete/search-type,
  params `_id`/`_lastUpdated`/`_count`/`identifier`/`name`/`family`/`given`/
  `gender`). All FHIR handlers now emit `application/fhir+json` and every
  non-2xx body is a FHIR `OperationOutcome` (§5). Routes stay behind the
  blanket auth+ABAC guard (`/fhir/*` off the public allow-list). New
  DB-free unit tests in `src/api/fhir/mod.rs` pin `to_fhir` ⇒
  `resourceType == "Practitioner"`, core-field round-trip, and missing-name
  rejection; the mount test re-points to `/fhir/Practitioner`.
  `cargo test --lib` green (161 passed); `cargo clippy --lib` clean.
  Documented gaps (unchanged from the prototype, `TODO`-marked in
  `from_fhir_worker`): identifiers decode to `IdentifierType::Other`,
  and `additional_names` / `marital_status` / `multiple_birth` /
  `managing_organization` / `tax_id` / `documents`→`qualification` are not
  yet parsed back; search filters only on the first name param (no
  `_id`/`_lastUpdated`/`gender`/`identifier` filtering yet); masked-read
  obligation not yet wired into the FHIR read path.
  ([`agents/share/fhir.md`](../../../agents/share/fhir.md)). **Reconcile the
  existing unmounted `src/api/fhir/` prototype**: switch the non-standard
  `resourceType: "Worker"` to standard **`Practitioner`** (§3, `high`
  fidelity) and **mount the routes** (handlers exist; T-9 wired the
  prototype `/fhir/Worker` surface, which this task re-points to
  `/fhir/Practitioner`). Map the domain worker to `Practitioner`:
  `name` → `name`, `identifiers` (NPI, professional licence, …) →
  `identifier` (token `system|value`), `telecom` → `telecom`,
  `addresses` → `address`, `gender` → `gender`, `birth_date` →
  `birthDate`, credential `documents` (professional credentials /
  certificates) → `qualification`; `active`. Add `FhirOperationOutcome`
  errors (§5), a searchset `Bundle` (§6), and `GET /fhir/metadata`
  returning a `CapabilityStatement` (§7). Routes join the existing Axum
  router under the blanket auth+ABAC guard (§8; `/fhir/*` guarded — worker
  PII, deliberately off the public allow-list — action derived from the
  HTTP method) and honour masked reads for personal data. Supported search
  params: `_id`, `_lastUpdated`, `_count`, `identifier`, `name`, `family`,
  `given`, `gender`. **Acceptance:** tests cover domain↔`Practitioner`
  round-trip, each interaction (read / create / update / delete / search),
  search→`Bundle`, `OperationOutcome` on error, `CapabilityStatement`
  matches the mounted routes, and masked-read.

- [x] **T-13 — Durable event bus Phase 2 (transactional outbox).**
  *(Done 2026-07-08)* Implements
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §3/§5
  storage + write path, copying the completed **event-service** reference.
  - [x] `event_outbox` migration (`m20260708_000001_create_event_outbox`
    + hand-written `up.sql`/`down.sql`) registered in the migrator; the
    `event_outbox` SeaORM entity in `src/db/models.rs`.
  - [x] `src/db/outbox.rs`: `OutboxInsert` with the pure DB-free
    `from_envelope` mapping, `for_event`/`for_merge` conveniences, a
    `ConnectionTrait`-generic `insert_on` (so a `dyn`-repo threads its own
    transaction), and the relay `Model::recent`/`unpublished`/`mark_published`
    (Phase-3 roadmap poll+ack).
  - [x] `src/streaming/envelope.rs`: the canonical `Envelope`
    (`entity: &'static str` with `#[serde(skip_deserializing,
    default = "default_entity")]`, plus `merged_from`), `EventKind`,
    `EventView` projection, and the `EventTransport` selector read once from
    `WORKER_EVENT_TRANSPORT` (default `memory`).
  - [x] Repository: a `transport` field + `with_transport` builder +
    `enqueue_outbox`; the outbox row is written **inside** each write's
    transaction in `create`/`update`/`delete` (the tx-free soft-delete opens
    one under the outbox transport). A new `merge(survivor, duplicate_id)`
    repo method applies the survivor's rows + soft-deletes the duplicate +
    enqueues `Merged`(+`merged_from`) and `Deleted` outbox rows **atomically
    in one transaction**; the merge handler now calls it (dropping the old
    update + delete + separate publish).
  - **Acceptance:** DB-free unit tests (`from_envelope`, `for_merge`,
    transport parse) + DB-gated `#[ignore]` atomicity tests
    (`create_enqueues_a_created_outbox_row`,
    `merge_enqueues_merged_with_merged_from_and_deleted`) that compile under
    a bare `cargo test --lib`. Met: `cargo test --lib` green (179 passed,
    2 ignored); `cargo clippy --lib --tests` clean.
  - [x] **Phase 3 (relay + retention).** *(Done 2026-07-08, copy-adapted
    from the organization reference.)* `src/relay.rs`: the `EventSink` trait
    (the bus seam), a working no-broker **`LoggingSink`** default,
    `drain_once` (`unpublished` → `sink.send` → `mark_published`,
    at-least-once, per-pid order preserved on a send failure), and
    `purge_published` (retention). A background loop (`relay::spawn`, started
    in `App::after_routes`) ticks every `WORKER_EVENT_RELAY_INTERVAL_SECS`
    and purges every N ticks — **gated by `WORKER_EVENT_TRANSPORT=outbox` AND
    `WORKER_EVENT_RELAY`**, so it is a no-op by default; `purge_published`
    now enforces `WORKER_EVENT_RETENTION_DAYS`. Tests: DB-free
    `LoggingSink`/capturing-sink send + config defaults; the drain/ack seams
    are DB-gated-tested via the outbox suite. **Broker-gated follow-up:** a
    real **`FluvioSink`** (`impl EventSink` behind a `fluvio` cargo feature) —
    the trait is the seam, so the drain loop is unchanged when it lands.

- [x] **2026-07-19 — Stored review queue + decision endpoints.** Persist
  the batch-dedup candidates (`review_queue` migration + the shared
  raw-SQL `db/review_queue` module: normalized-pair upsert / list /
  first-writer-wins decide), report stored rows from the scan, and add
  `GET /api/workers/review-queue` + `POST
  /api/workers/review-queue/{id}/decision`. Front-end `/review` board
  loads the stored queue on mount and drag records decisions.
  **Acceptance:** serde pins for the decision wire tokens; the person
  crate's env-gated DB round-trip (`tests/review_queue_db.rs` — the
  module is byte-identical family-wide) green against Postgres 18;
  `cargo test --lib` + clippy pedantic clean; FE svelte-check / vitest /
  Playwright green.

