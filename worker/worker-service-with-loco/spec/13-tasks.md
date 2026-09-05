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
- [x] **T-2 — Production Fluvio publisher.** *(done 2026-08-03, via
  BUS-3 — see `AGENTS.md` "Durable event bus relay (Fluvio)")* Superseded
  the originally-scoped `FluvioEventPublisher : EventProducer` shape with
  a real-broker `FluvioSink : EventSink` (`src/relay.rs`), ported from
  the case-service BUS-1 reference, sitting behind the Phase-3 outbox
  relay (default-off via `WORKER_EVENT_TRANSPORT=outbox` +
  `WORKER_EVENT_RELAY`) and this crate's own off-by-default `fluvio`
  Cargo feature — a plain `cargo build`/`cargo test` is unaffected.
  `WORKER_FLUVIO_ENDPOINT` selects it over the default `LoggingSink`; an
  endpoint configured without the feature compiled in makes the relay
  refuse to start (logged, not a silent no-broker fallback).
  - [x] `FluvioSink` implemented behind the `fluvio` feature.
  - **Acceptance (met):** `tests/fluvio_relay.rs` is a
    `#![cfg(feature = "fluvio")]`-gated, `#[ignore]`d live round-trip
    against a local broker (`compose.fluvio.yaml` +
    `Dockerfile.fluvio-cli`); `cargo build --lib --features fluvio` and
    `cargo clippy --all-targets --features fluvio -- -D warnings` prove
    the real `fluvio` 0.50 API compiles. No automated CI run in this repo
    stands up a broker, so the round-trip itself is exercised locally,
    not in CI.
- [~] **T-3 — FHIR capability statement + bundle handling.**
  - [x] *(done 2026-07-07, via T-12)* `GET /fhir/metadata` returns a
    CapabilityStatement listing the **`Practitioner`** resource — the
    wire `resourceType` this server actually emits (T-12 switched it
    from the original non-standard `Worker`; see §6.8) — with its
    supported interactions and search params.
  - [x] *(done 2026-07-07, via T-12)* `GET /fhir/Practitioner` wraps
    search results in an ad hoc `searchset` `Bundle`
    (`src/api/fhir/handlers.rs::search_fhir_workers`).
  - [ ] Promote the ad hoc `Bundle` to typed `Bundle`/`BundleEntry`
    structs (`src/api/fhir/bundle.rs` is a placeholder module reserved
    for this); no `POST`/transaction `Bundle` support yet.
  - **Acceptance:** Touchstone FHIR validator passes on a sample
    bundle round-trip — not yet run; the `searchset` shape has not been
    validated against a real FHIR test kit.
- [ ] **T-4 — FHIR Organization resource.**
  - [ ] Bidirectional Organization mapping.
  - **Acceptance:** `POST /fhir/Organization` round-trips a record.
- [ ] **T-5 — Dedup / merge / privacy integration tests.**
  - [ ] Real-time dedup on create.
  - [ ] Batch dedup + auto-merge.
  - [ ] Mask + export round-trip.
  - **Acceptance:** `cargo test --test api_integration_test` covers
    all three workflows.
- [~] **T-6 — gRPC implementation.** **Landed 2026-09-02 (repo
  `tasks.md` PRO-H11 — following person-service's reference
  implementation).**
  - [x] Promoted the stub to a working Tonic server:
    `proto/worker.proto` (package `worker`) + `build.rs` (`tonic-build`,
    already correctly pinned to 0.12 in this crate's manifest — unlike
    person's, which had to be fixed from a mismatched 0.14 — but still
    dead scaffolding with no `build.rs`/`proto/` until now) +
    `src/api/grpc/service.rs` (`WorkerGrpcService`), covering
    `CreateWorker` / `GetWorker` / `ListWorkers` / `DeleteWorker`.
    Deliberately not the full REST surface: no `UpdateWorker` RPC, no
    match/merge/search/assessments/FHIR over gRPC. The proto `Worker`
    message is also a deliberate **partial** projection (id, name,
    gender, `worker_type`, birth date, tax id, timestamps) — not every
    field the domain model carries (identifiers, addresses, telecom,
    documents, emergency contacts, links); extending it is follow-up.
  - [x] **No duplicated business logic.** Every RPC delegates into the
    exact functions REST already calls: `crate::validation::validate_worker`
    (`CreateWorker`), the shared duplicate-detection core
    (`check_duplicates_internal`, bumped from private to `pub(crate)`
    rather than copied), the same `WorkerRepository` trait methods
    (`create`/`get_by_id`/`list_active`/`delete` — this crate's
    repository takes no `AuditContext`, unlike person's, so there is
    no `audit_context_of` equivalent needed here), and
    `auth::authorize_record` + `crate::privacy::mask_worker` for
    `GetWorker`'s record-level ABAC + masking, matching
    `handlers::get_worker`'s own logic exactly. `worker_type` parses
    via the domain enum's existing `serde` implementation
    (`serde_json::from_value`) rather than a hand-rolled second
    mapping that could drift from it.
  - [x] **Auth parity, not an unauthenticated side door.**
    `grpc_enforce` (gRPC metadata → `authentication_verifier::Claims`
    → the same `Policy::evaluate`) is the blanket-guard counterpart of
    REST's `auth::enforce`, gated by the same `WORKER_REQUIRE_AUTH`
    flag. `GetWorker`/`DeleteWorker` additionally run
    `authorize_record` against the loaded record, same as REST's
    single-record handlers. **Documented, not silently missing:** the
    HIPAA §164.528 disclosure-accounting audit row REST writes on
    every read is not yet written on the gRPC path; `ListWorkers`
    applies only the blanket `Read` check, not REST's per-record
    read-visibility filtering.
  - [x] **Verified live, not merely compiled.**
    `tests/grpc_integration_test.rs` binds a real
    `tonic::transport::Server` on an OS-assigned port and drives it
    with a real `WorkerServiceClient` over an actual HTTP/2 connection:
    a Create→Get→List→Delete→Get(`NOT_FOUND`) round trip against the
    same database/search-index REST integration tests use, plus a
    blank-family-name → `INVALID_ARGUMENT` proof, an unrecognised
    `worker_type` → `INVALID_ARGUMENT` proof, and a malformed-id →
    `INVALID_ARGUMENT` proof (not `INTERNAL`). All four pass against a
    real Postgres (`scripts/ci-check.sh test-db
    worker/worker-service-with-loco`, full suite green). `grpcurl` was
    not additionally run by hand — unavailable in this sandbox — but
    the automated test proves the identical claim the spec's original
    acceptance criterion named, repeatably.
  - **Acceptance:** `grpcurl` against `WorkerService.GetWorker`
    round-trips a record — satisfied by `tests/grpc_integration_test.rs`
    (above); a literal `grpcurl` CLI run is optional local confirmation,
    not additionally exercised.
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
    `tests/api_integration_test.rs` — `test_fhir_practitioner_route_is_mounted`
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
  - [x] **Durable event bus — Phase 3, `FluvioSink` (BUS-3).** *(done
    2026-08-03, ported from the case-service BUS-1 reference)* The
    real-broker `impl EventSink`, behind this crate's own `fluvio` Cargo
    feature (off by default — the dependency tree and boot behaviour of a
    default build are unchanged). One producer per topic
    (`fluvio::Fluvio::connect_with_config` + `topic_producer`, held for the
    sink's lifetime), partitioned by record `pid` per
    `agents/share/event-bus.md` §7. Config: `WORKER_FLUVIO_ENDPOINT` (the
    broker's SC address; unset ⇒ `LoggingSink`, unchanged default
    behaviour) and `WORKER_EVENT_TOPIC` (default `mxi.worker.events`).
    **No silent fallback**: an endpoint configured **without** the
    `fluvio` feature refuses to start the relay at all (logged at
    `error`), rather than a `LoggingSink` masquerade that would mark
    outbox rows `published_at` without ever reaching the broker the
    operator asked for — the same shape as the family's artifact-store
    "no fallback on an explicit backend choice" rule
    (`agents/share/bulk-import-export.md` §12). The initial connection
    retries indefinitely rather than falling back, for the same reason.
    `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` provision a local
    SC+SPU broker (Fluvio's own documented Docker Compose layout,
    translated to this repo's Podman conventions) for opt-in manual
    runs; **not** wired into any automated CI stage. Tests: `cargo
    build`/`clippy --all-targets -D warnings`/`fmt --check` clean under
    both default features and `--features fluvio` (the real `fluvio`
    0.50 API compiling is the actual verification of correct usage).
    `tests/fluvio_relay.rs` is a `#![cfg(feature = "fluvio")]`-gated,
    `#[ignore]`d round-trip (create under outbox transport →
    `FluvioSink` → `drain_once` → assert `published_at`), connecting
    directly via `DATABASE_URL` rather than through
    `loco_rs::testing::prelude::request` — this crate's dev-dependencies
    do not enable loco's `testing` feature (unlike case's), so the test
    follows the same direct-`DATABASE_URL` pattern this crate's own
    DB-gated outbox atomicity tests already use
    (`src/db/repositories.rs::tests`) rather than introducing the loco
    harness for one file. It needs a live broker, which no automated run
    in this repo stands up, so it is verified by compiling under the
    feature, not by an actual execution (same posture as case's
    `tests/fluvio_relay.rs` and person's
    `s3_round_trip_against_a_live_endpoint`, BLK-4). SOUP register
    updated. BUS-2 (link-graph Fluvio consumer) and rolling `FluvioSink`
    to the remaining services remain.

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

- [x] **T-14 — Workforce assessments (aptitude / personality /
  psychometric / selection).** *(done 2026-07-23; renumbered from a
  duplicate "T-10" during the 2026-08-04 doc audit — T-10 was already
  taken by the cross-service-links task above, landed the same week;
  the CHANGELOG's own "task T-10" references predate the renumbering
  and describe this same task)* Record and serve the
  tests a worker has taken, as a worker sub-resource. Spec: domain model
  §5.5, functional requirements §6.9, API §9.2, persistence §10.5.
  - [x] **Domain model** (`src/models/assessment.rs`):
    `AssessmentCategory` (the four families) × `AssessmentScale` (the 13
    measured dimensions) with the `permits` rule — a category accepts its
    own scales, and `psychometric` additionally accepts aptitude and
    personality scales, because a psychometric test covers both;
    `ScoreBand::from_percentile` (the norm-referenced 10/30/70/90 split,
    clamping rather than panicking out of range); the `AssessmentStatus`
    lifecycle machine; `Assessment::is_valid_on` (completed and
    unexpired), `mean_percentile`, and `masked`.
  - [x] **Persistence**: the `worker_assessments` migration
    (`m20260723_000001`, per-scale results as JSONB) + the SeaORM entity
    + `src/db/assessments.rs` (insert / worker-scoped list + find /
    update / soft-delete, and the row↔domain conversion — a drifted
    stored token or malformed `results` is a mapped error, never a
    panic).
  - [x] **Validation** (`validate_assessment`): instrument required,
    scale-in-category, one reading per scale, percentile ∈ [0, 100],
    `0 ≤ raw ≤ max` with `max > 0`, expiry not before administration, a
    completed assessment carrying its date and results, and SEC-M1 caps
    — the full problem list in one `422`.
  - [x] **Endpoints** (`src/api/rest/assessments.rs`, mounted on both
    router surfaces + OpenAPI): the five CRUD routes plus the derived
    `GET /api/workers/{id}/assessment-profile`. Worker-level ABAC on
    every route, the `mask` obligation honoured on **every** read path
    (single, list, and profile — invariant 5), audit rows on reads and
    mutations, and update re-validating the *merged* record so it cannot
    reach a state a create would refuse.
  - **Acceptance:** 21 DB-free unit tests across the model (scale↔category
    consistency, the psychometric overlap, band boundaries, the lifecycle
    matrix, validity, masking, token round-trips), the persistence
    conversion (round-trip + drift-is-an-error), and the pure API
    derivations (filters, most-recent-current-reading, selection
    suitability, the masked profile withholding scores, every category
    present when empty, deterministic recency ordering) — plus five
    validation tests and a DB-gated round-trip
    (`round_trip_insert_find_update_delete`, `#[ignore]`, needs
    `DATABASE_URL`). Met: `cargo test --lib` green (225 passed, 5
    ignored); `cargo clippy --all-targets` clean; `cargo test --doc`
    green.
  - **Follow-ups (not queued):** front-end views for the profile, and a
    FHIR `Observation` projection of assessment results.

- [x] **BUG-1 — `workers.gender` was persisted in the wrong case.**
  *(fixed 2026-07-23)* `src/db/repositories.rs` stored the bare `Debug`
  form of [`Gender`](crate::models::Gender) (`"Male"`, `"Unknown"`) at
  all three write sites (create, update, and the merge row
  replacement), but the `workers` table's CHECK constraint admits only
  `'male' | 'female' | 'other' | 'unknown'`
  (`migrations/2024122800000002_create_workers/up.sql`). Against a
  constrained schema **every create and update failed** with
  `violates check constraint "workers_gender_check"`; the DB-gated
  outbox tests were red for the same reason. The search index
  (`src/search/mod.rs`) and the FHIR surface (`src/api/fhir/mod.rs`)
  had already lowercased, so the three DB writers were the outliers —
  and the sibling person-service had already fixed the identical bug
  the same way, so this restores family consistency.
  - Fix: `.to_lowercase()` on all three writers, and the read parser
    (`from_db_models`) now lowercases before matching so rows written
    by the old path on an unconstrained deployment still round-trip
    instead of silently reading back as `Unknown`.
  - **Data migration** (`m20260723_000002_normalize_worker_gender_case`,
    added on request): `UPDATE workers SET gender = lower(gender) WHERE
    gender <> lower(gender)` — idempotent, and a **no-op on a
    correctly-constrained schema** (where the bad writes were rejected
    in the first place). It exists for deployments whose `workers`
    table was created without the constraint (hand-rolled schema, older
    schema file, or a bulk load through another tool), where the values
    were accepted and would now block a later ADD CONSTRAINT. Values
    still outside the vocabulary after lowercasing (`'M'`,
    `'not stated'`, …) are **deliberately left alone**: rewriting them
    to `'unknown'` would destroy data only an operator can interpret,
    so ADD CONSTRAINT fails loudly on them instead. The `up.sql`
    carries the query that finds them. `down` is a documented no-op —
    re-capitalizing would violate the constraint *and* corrupt rows
    that were always lowercase.
  - **Acceptance:** a DB-free regression pin,
    `db::repositories::tests::gender_is_persisted_as_a_constraint_legal_token`,
    asserts every `Gender` variant persists as a token the CHECK
    constraint admits **and** as its serde wire token (so the DB, the
    search index, and FHIR agree on one spelling). Verified to fail
    against the pre-fix code. The two DB-gated outbox tests
    (`create_enqueues_a_created_outbox_row`,
    `merge_enqueues_merged_with_merged_from_and_deleted`) now pass
    against Postgres 18. The data migration has its own DB-gated pin,
    `tests/gender_normalization_db.rs` — it reproduces the affected
    deployment (drop the constraint, plant a `'Male'` row), runs the
    migration's **real SQL** via `include_str!` so test and migration
    cannot drift, asserts the value is normalized, and then *proves*
    the repair by re-adding the constraint (which only succeeds if
    every row is legal), plus an idempotent re-run. `cargo test --lib`
    green (226 passed); clippy `--all-targets` clean.

- [x] **T-15 — `Config::from_env` loads the environment.** *(done
  2026-07-23; renumbered from a duplicate "T-11" during the 2026-08-04
  doc audit — T-11 was already taken by the bulk import/export task
  above)* The function was a stub that returned `Config::default()`
  and ignored the process environment, so every documented variable
  (`DATABASE_URL`, `SERVER_PORT`, `SEARCH_INDEX_PATH`, …) was inert —
  the integration-test harness, which builds its state from
  `Config::from_env()`, could never be pointed at a test database.
  - Env → `.env` (best-effort) → default precedence, over the 14
    variables in the `from_env` doc table (the family's 11 plus
    `SEARCH_CACHE_SIZE_MB`, `STREAMING_BROKER_URL`, `STREAMING_TOPIC`,
    which were previously unreachable config fields).
  - Blank / whitespace-only ⇒ **unset** (an empty `SERVER_HOST` must
    not bind the server to nothing); a malformed typed value ⇒
    `Error::Config` naming the variable and its raw value, never a
    silent default.
  - The overlay lives in a pure `Config::from_source(lookup)` seam so
    it is testable without mutating process env — `std::env::set_var`
    is `unsafe` in the 2024 edition, which this crate forbids.
  - **Acceptance:** five unit tests (defaults, every variable applied,
    blank-as-unset, malformed-refused-by-name, whitespace tolerance)
    green on a bare `cargo test --lib`; clippy clean. The same seam and
    tests landed in all six `*-service-with-loco` crates that carry a
    `Config`, so the family is uniform.

- [x] **T-16 — Remove dead `FluvioProducer`/`FluvioConsumer` stub types.** *(resolved 2026-09-04.)*
  `src/streaming/producer.rs::FluvioProducer` and
  `src/streaming/consumer.rs::FluvioConsumer` both `todo!()` on every
  method and have zero callers anywhere in the crate (verified:
  `grep -rn "FluvioProducer\|FluvioConsumer" --include="*.rs" .` matches
  only their own definition/doc-comment lines) — leftover `EventProducer`/
  `EventConsumer`-trait scaffolding from before the Phase-3 outbox relay
  (`src/relay.rs::FluvioSink`, T-2) superseded this shape entirely. Same
  situation as the dead `WorkerRepository::search` SQL method removed in
  0.6.0 (QA-CUST-SQL): a plausible-looking, unexercised stub reachable
  from library code. Remove both types (and the now-pointless
  `EventConsumer` trait/module if `FluvioConsumer` was its only
  implementor).
  - **Acceptance:** `grep -rn "FluvioProducer\|FluvioConsumer" src/ tests/`
    returns nothing; `cargo test --lib` and
    `cargo clippy --all-targets -- -D warnings` stay clean.
  - **Resolved.** `src/streaming/consumer.rs` deleted outright (its only
    content was `FluvioConsumer`); `FluvioProducer` removed from
    `src/streaming/producer.rs`; the now-pointless `EventConsumer` trait
    (and its `pub mod consumer;` declaration) removed from
    `src/streaming/mod.rs` — `EventProducer` stays, since
    `InMemoryEventPublisher` still implements it. Module doc comments
    updated to describe the removed scaffolding without naming the
    removed types literally, so the acceptance grep stays true even in
    the historical note. `grep -rn "FluvioProducer\|FluvioConsumer" src/
    tests/` returns nothing; `cargo test --lib` (314 passed) and
    `cargo clippy --all-targets -- -D warnings` both clean.

- [x] **T-17 — `ProbabilisticMatcher::threshold()` ignores the configured `MATCHING_THRESHOLD`.** *(resolved 2026-09-04.)*
  `ProbabilisticMatcher::threshold()` (`src/matching/mod.rs:203-210`) is a
  public `#[must_use]` accessor that returns a hardcoded literal `0.85`
  regardless of the `MatchingConfig` the matcher was built with (verified:
  its own doc comment admits "hard-coded ... tracked as a TODO"), while the
  trait's real `is_match`/`classify_match` path correctly delegates to
  `ProbabilisticScorer::is_match`, which reads `self.config.threshold_score`
  (verified: `src/matching/scoring.rs:156-159`) — the value `Config::from_env`
  (T-15) actually loads from `MATCHING_THRESHOLD`. No caller of `.threshold()`
  exists today (verified: `grep -rn '\.threshold()' src/` — zero hits), so the
  divergence is latent, not yet a live bug, but the method's answer is wrong
  the moment anything calls it or `MATCHING_THRESHOLD` is set to a non-default
  value.
  - [x] Expose the scorer's real configured `threshold_score` (an accessor on
    `ProbabilisticScorer`) instead of the duplicated literal.
  - **Acceptance:** a unit test builds a `ProbabilisticMatcher` from a
    `MatchingConfig{threshold_score: 0.42, ...}` and asserts
    `matcher.threshold() == 0.42`; `cargo test --lib` green; clippy clean.
  - **Resolved.** Added `ProbabilisticScorer::threshold_score()`
    returning `self.config.threshold_score` (the same field `is_match`/
    `classify_match` already read); `ProbabilisticMatcher::threshold()`
    now delegates to `self.scorer.threshold_score()` instead of the
    hard-coded `0.85`. New unit tests in both `src/matching/mod.rs` and
    `src/matching/scoring.rs` pin a non-default `0.42` round-trips
    through each layer.

- [x] **T-18 — DB-gated round-trip test for the review-queue persistence module.** *(resolved 2026-09-05.)*
  `src/db/review_queue.rs` (228 lines: normalized-pair upsert / list /
  first-writer-wins decide, added under the 2026-07-19 "Stored review
  queue" task) carries no `#[test]`/`mod tests` of its own (verified:
  `grep -n '#\[test\]\|mod tests' src/db/review_queue.rs` — no matches),
  and this crate's `tests/` directory has no `review_queue_db.rs`
  (verified: `find tests -name '*.rs'` lists `api_integration_test.rs`,
  `duplicate_detection.rs`, `enforcement.rs`, `fluvio_relay.rs`,
  `gender_normalization_db.rs`, `grpc_integration_test.rs`, `otlp_*` — no
  review-queue file) — unlike what the 2026-07-19 task's acceptance note
  leans on ("the person crate's env-gated DB round-trip … the module is
  byte-identical family-wide"). "Byte-identical to a tested module
  elsewhere" is not "tested here": a worker-specific migration drift
  would go undetected by this crate's own suite.
  - **Acceptance:** a new `#[ignore]`d, `DATABASE_URL`-gated
    `tests/review_queue_db.rs` inserts a pair, upserts a re-scan (score
    refreshed, decision preserved), lists pending rows, and exercises the
    first-writer-wins decision path against a real migrated Postgres;
    green under `scripts/ci-check.sh test-db
    worker/worker-service-with-loco`.
  - **Resolved.** New `tests/review_queue_db.rs`, adapted from (not
    copied verbatim from) person's `tests/review_queue_db.rs` — worker's
    module has in fact already drifted from person's (no `provenance`
    column, unboxed `DecideOutcome::Decided`), confirming the "byte-
    identical" premise this task questioned no longer held even before
    this test existed to prove it. Connects via the family-standard
    `DATABASE_URL` env var (not person's bespoke
    `REVIEW_QUEUE_TEST_DATABASE_URL` + inline migration application),
    matching this crate's own `tests/gender_normalization_db.rs`
    pattern — migrations are applied by `scripts/ci-check.sh test-db`
    before the suite runs. Covers insert, pair-order normalization,
    re-scan upsert (score refreshed, decision preserved), the
    first-writer-wins `decide` path (`Decided` /
    `AlreadyDecided` / `NotFound`), and the pending-status list filter;
    cleanup is scoped to the test's own two record ids (never a
    blanket table wipe, since other suites may hold rows of their
    own). Verified against a real Postgres 18 via `scripts/ci-check.sh
    test-db worker/worker-service-with-loco`: full DB-gated suite
    passes (11 + 23 + 1 + 4 + 1 across the crate's suites), 0 failed;
    `cargo test --lib`: 314 passed (unchanged — no unit tests added),
    0 failed; `cargo build`/`clippy --all-targets -- -D warnings`
    clean.

- [ ] **T-19 — FHIR Practitioner round-trip fidelity: parse the fields `from_fhir_worker` still drops.**
  T-12 (done) mounted `/fhir/Practitioner`, but five `TODO`s remain in the
  FHIR→domain direction (verified: `grep -n TODO src/api/fhir/mod.rs` —
  lines 301, 409, 420, 421, 423): identifiers always decode to
  `IdentifierType::Other` regardless of the FHIR `Identifier.system`, and
  `additional_names`, `marital_status`, `multiple_birth`, and
  `managing_organization` are silently dropped on `POST`/`PUT
  /fhir/Practitioner` even though `to_fhir` emits them going out — so a
  client that reads a `Practitioner`, edits an unrelated field, and writes
  it back loses that data. No open task tracks closing these (T-3's open
  item is `Bundle` typed-struct promotion + Touchstone validation, a
  different surface).
  - [ ] Map `Identifier.system` back to the domain `IdentifierType` enum
    via its existing serde vocabulary instead of defaulting to `Other`.
  - [ ] Parse `additional_names`, `marital_status`, `multiple_birth`, and
    a `managing_organization` reference back from the resource.
  - **Acceptance:** a round-trip unit test in `src/api/fhir/mod.rs` —
    `to_fhir` a `Worker` carrying two-plus `IdentifierType` variants,
    `additional_names`, `marital_status`, and `multiple_birth`, then
    `from_fhir_worker` the result, and assert the values survive instead
    of degrading to `Other`/`None`/`vec![]`; `cargo test --lib` green.

- [ ] **T-20 — gRPC surface: close the T-6 documented parity gaps.**
  T-6 (landed 2026-09-02) explicitly documents three REST-vs-gRPC gaps as
  "not silently missing" but none is a queued, checkable task item
  (verified: neither `spec/13-tasks.md`'s T-6 block nor `AGENTS.md`'s
  "gRPC server" section lists them as their own acceptance-bearing
  bullets — only prose): (1) no `UpdateWorker` RPC exists at all; (2) the
  HIPAA §164.528 disclosure-accounting audit row REST writes on every
  `GET /api/workers/{id}` is not written by `WorkerGrpcService::GetWorker`;
  (3) `ListWorkers` applies only the blanket `Read` ABAC check, not REST's
  per-record read-visibility filtering. (2) and (3) are compliance-relevant
  divergences between two API surfaces over the same data — invariant 5
  (`agents/share/security.md` §3, "masking/authorization must hold on every
  read path") reads as squarely in scope.
  - [ ] Add an `UpdateWorker` RPC delegating to the same path
    `handlers::update_worker` uses.
  - [ ] Wire the disclosure-accounting audit write into `GetWorker`.
  - [ ] Apply `ListWorkers`' REST counterpart's per-record read-visibility
    filter to the gRPC path.
  - **Acceptance:** `tests/grpc_integration_test.rs` gains an
    `UpdateWorker` round trip (Create→Update→Get shows the change), a
    disclosure-audit-row-written assertion on `GetWorker`, and a
    `ListWorkers` test proving a record an ABAC policy would filter from
    REST's list is excluded from gRPC's too; green against a real Postgres
    (`scripts/ci-check.sh test-db worker/worker-service-with-loco`).
