## 13. Tasks

Spec-driven work breakdown. Each task has an acceptance criterion;
tick the box when an automated test or clearly described manual check
confirms the criterion is met. Tasks small enough to land in a single
PR; split larger tasks (`T-12a`, `T-12b`).

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
- [ ] **T-1c — Auth follow-ups: boot-time key fetch + authorization.**
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
    surfaces verify against it. One-shot fetch; no refresh loop
    (periodic refresh is a §15 roadmap note). Pinned by DB-free tokio
    tests in `src/api/rest/auth.rs`: fetch from a local ephemeral-port
    listener serving the in-process key set (minted token verifies),
    fallback on a dead port (no panic, token rejected), and the
    URL-unset ⇒ env-path precedence.
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
    `cargo test --lib` green.
  - [ ] DB-gated request test (`#[ignore]`, Postgres): with
    `PERSON_REQUIRE_AUTH` set, an unauthenticated `GET /api/persons/…`
    returns `401` while `GET /api-docs/openapi.json` stays `200`.
  - **Acceptance (met, except the DB-gated request test above):**
    valid token whose attributes satisfy the policy gets `2xx`; a
    valid token the policy denies gets `403`; no/bad token gets
    `401`. Key-set fetch from a stub auth service at boot: **met** via
    the local-listener tokio tests above (`cargo test --lib` green).
    Activation (`PERSON_REQUIRE_AUTH=1`) remains the operational
    decision.
- [ ] **T-2 — Production Fluvio publisher.**
  - [ ] Implement `FluvioEventPublisher : EventProducer` behind
    feature flag `fluvio`.
  - [ ] Document failover behaviour when the broker is unreachable.
  - **Acceptance:** integration test against a local Fluvio broker
    publishes a `PersonCreated` event end-to-end.
- [ ] **T-3 — Complete FHIR bundle handling.**
  - [ ] `Bundle` GET / POST / search wrapping.
  - [ ] OperationOutcome on malformed bundles.
  - **Acceptance:** Touchstone FHIR validator passes on a sample
    bundle round-trip.
- [ ] **T-4 — FHIR capability statement endpoint.**
  - [ ] `GET /fhir/metadata` returns a CapabilityStatement listing
    supported resources + interactions.
  - **Acceptance:** schema check against R5 CapabilityStatement.
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
- [ ] **T-9 — Cross-service entity links (write side).**
  See §5.4, §8.6, §9.1, §10.4 and
  [cross-service linking](../../../agents/share/cross-service-linking.md).
  - [ ] Migration creating the `entity_links` table (§10.4 schema, with
    the `UNIQUE (from_pid, kind, to_ref, valid_from)` upsert key).
  - [ ] `EntityRef` value type (parse / `Display` + `entity_type → service`
    map), copied per project (drift-accepted).
  - [ ] Link endpoints: `POST` / `GET` / `DELETE`
    `/api/persons/{pid}/links`; create/upsert is optimistic (no
    cross-service call) and supports `same_identity` (person ↔ worker)
    and `works_at` / `member_of` (person → organization, temporal).
  - [ ] Emit `linked` / `unlinked` events on the existing event
    envelope via `EventProducer` (edge detail in `data`; no new transport).
  - [ ] Partition guard in `src/matching/adapter.rs`: `entity_links` are
    never projected into the matcher input.
  - **Acceptance:** integration test creates a `works_at` link
    (`2xx`, `linked` event published, row in `entity_links`), lists it
    via `GET`, deletes it (`unlinked` event, `deleted_at` set); a matcher
    unit test asserts an `entity_links` row never alters a match score.
- [ ] **T-10 — Bulk import / export.** *(rollout steps 1 & 3 done
  2026-07-10; steps 2, 4, 5 remain)* Person is the family **reference
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
  - [ ] **Step 2** — CSV codec (flattening per §9.2: dotted single-nested,
    JSON-in-cell arrays) + keyless/unmatched rows → duplicate detection →
    review queue with `provenance = import`.
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
  - [ ] **Step 4** — Parquet **export-only**, feature-gated.
  - [ ] **Step 5** — S3-compatible artifact store; roll the contract to
    the other entities.
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
  (Phase 3) is now delivered (T-21); a real Fluvio sink is the only
  broker-gated follow-up.
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
  - **Remaining (broker-gated):** a real `FluvioSink` `impl EventSink`
    behind a future `fluvio` cargo feature — the trait is the seam, so
    the drain loop and retention are unchanged when it lands.
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
