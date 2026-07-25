# Care Pathway Service — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test in one PR. Live work queue is §13.
>
> Sibling matcher: [care-pathway-matcher](../../care-pathway-matcher-rust-crate/spec/index.md).
> Sibling front-end: [care-pathway-front-end-with-svelte](../../care-pathway-front-end-with-svelte/spec/index.md).

## 1. Purpose and vision

A registry of clinical care-pathway records for the Main X Index family:
create/read/update/delete and detect duplicates with the canonical
care-pathway-matcher. Built on loco.rs.

## 2. Scope

MVP: CRUD + `ILIKE` name search + matching + record merge + audit log +
in-memory event streaming (durable-bus Phase 1) + OpenAPI/Swagger +
Prometheus metrics + offline PASETO v4 public token verification + blanket
`/api/*` enforcement (off by default) + rich payload validation
(ICD/SNOMED/UUID/DOI/BCP-47). Deferred (§13): Tantivy full-text/fuzzy
search, search-blocked dedup candidates, the durable event bus's real
Fluvio broker sink (Phases 2–3 — transactional outbox + relay/retention
— are done; only the broker-gated `FluvioSink` remains), privacy,
front-end merge action, a PASETO key-set
refresh loop (the boot-time paseto-keys-over-HTTP fetch is done —
`CARE_PATHWAY_PASETO_KEYS_URL`, §9/§13 — but runs once, no re-fetch),
terminology-server code-existence checks, gRPC. Token
issuance is out of scope — provided by the central authentication-service.
The session / cross-service token model is fixed by
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md),
which supersedes the prior RS256-JWT model.

## 3. Stakeholders and users

Clinical informaticians curating pathways; peer services; the
care-pathway front-end.

## 4. Glossary

- **care pathway** — a standardised, evidence-based care plan.
- **pid** — public UUID of a pathway record.
- **data** — the full `CarePathway` payload stored as JSONB.
- **condition code** — ICD/SNOMED code of the target condition.

## 5. Domain model

The API DTO is `care_pathway_matcher::CarePathway`: `name`,
`alternate_names`, `pathway_code`, `provider_id`, `provider_name`,
`care_setting`, `condition_codes`, `interventions`, `keywords`,
`identifiers`, `same_as`, `in_language`.

## 6. Functional requirements

1. `POST /api/care-pathways` — create; `name` required,
   `condition_codes` format-validated against their `system` (ICD-10 /
   ICD-11 / SNOMED CT SCTID Verhoeff; `Custom` non-blank), `identifiers`
   structurally checked (canonical UUID for `Uuid`; `10.…/…` shape for
   `Doi`; other schemes non-blank), and `in_language` checked for BCP-47
   syntax; `422` on any problem, all reported together — also enforced on
   update. Rules in [`src/validation.rs`](../src/validation.rs).
2. `GET /api/care-pathways` — list active (cap 100), `{pid, name}`.
   `GET /api/care-pathways/search?q=` — case-insensitive name search
   (Postgres `ILIKE`, cap 50; blank `q` → `400`).
3. `GET /api/care-pathways/{pid}` — return the stored `CarePathway`.
4. `PUT /api/care-pathways/{pid}` — replace the payload (`422` if
   `name` is blank, or any `condition_codes` / `identifiers` /
   `in_language` entry is malformed).
5. `DELETE /api/care-pathways/{pid}` — soft-delete.
6. `POST /api/care-pathways/match` — rank an explicit `{query,
   candidates}` set (no persistence).
7. `POST /api/care-pathways/check-duplicates` — match a query against
   stored pathways; return those above threshold, ranked.
8. `POST /api/care-pathways/merge` — fold a duplicate into a survivor
   (union fields, former-title alias, soft-delete the duplicate,
   `merge_records` history, `Merged` event); `422` equal pids, `404`
   unknown. `GET /api/care-pathways/merges/recent` — merge history.
9. `GET /api/care-pathways/audit/recent` + `/{pid}/audit` — audit-log
   query; `GET /api/care-pathways/events/recent` — in-memory event
   stream. Each create/update/delete/merge writes an `audit_logs` row
   and publishes a `created`/`updated`/`deleted`/`merged` event.
10. `GET /api/care-pathways/whoami` — echo verified bearer-token claims
   (`401` without a valid token); proves offline PASETO v4 public verification.
11. `GET /api-docs/openapi.json` + `GET /swagger-ui` — OpenAPI 3
   document and a Swagger UI page rendering it.
12. `GET /metrics.prom` — Prometheus metrics in text-exposition format
   (`Content-Type: text/plain; version=0.0.4`), mounted at the root (not
   under `/api`) and public under blanket enforcement so a scraper
   needs no token. Exposes care-pathway CRUD/merge counters
   (`care_pathway_created_total` / `_updated_total` / `_deleted_total` /
   `_merged_total`) plus `http_requests_total`. Registry in
   [`src/metrics.rs`](../src/metrics.rs); handler in
   [`src/controllers/metrics.rs`](../src/controllers/metrics.rs).
13. **Compliance surface** (§12). `GET /api/compliance` — software
   identification, build provenance, IEC 62304 safety classification,
   the live control state, the declared data-protection posture, and
   per-framework "not claimed" lines. `GET /api/compliance/sbom` —
   CycloneDX 1.5 SBOM + SOUP register.
   `GET /api/compliance/audit/verify?limit=` — re-verifies the audit
   hash chain (default 1000 rows, capped 10 000) and reports every
   break. `GET /api/care-pathways/{pid}/audit/disclosures` — HIPAA
   §164.528 accounting, stating its own completeness.
   `POST /api/care-pathways/{pid}/erase` — GDPR Art. 17 erasure
   (**destructive** under ABAC; irreversible; idempotent).
14. **FHIR conformance additions** (§12.3).
   `POST /fhir/PlanDefinition/$validate` — profile + terminology +
   payload validation without persisting (always `200` with an
   `OperationOutcome`). `GET /fhir/.well-known/smart-configuration` —
   SMART discovery, served **only** when the deployment configures an
   authorization server, else `404` with an explanatory
   `OperationOutcome`. `GET /fhir/$export` (`202` + `Content-Location`)
   → `GET /fhir/$export-status/{id}` (manifest) →
   `GET /fhir/$export-file/{id}/{file}` (NDJSON), with
   `DELETE /fhir/$export-status/{id}` to cancel. `/fhir/metadata` and
   `/fhir/.well-known/smart-configuration` are **public** under blanket
   enforcement (discovery must precede the credential).
15. Bulk import/export (deferred, §13) — async, job-based, on the loco
   `bg_pg` worker: `POST`/`GET /api/care-pathways/import`,
   `POST`/`GET /api/care-pathways/export`,
   `GET /api/care-pathways/bulk-jobs`. The uniform family contract
   (execution model, five endpoints, JSONL/CSV/Parquet codecs,
   upsert-by-stable-key + dedupe-to-review, per-row error report, export
   masking + audit) is fixed in
   [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md).
   Care-pathway-specific bits — stable upsert keys (a deterministic
   scheme-scoped identifier the matcher short-circuits on /
   `(provider_id, pathway_code)`, same-provider only / `pid`); CSV
   flattening with every repeated/nested field a JSON-in-cell; clinical
   reference data (no patient-level data), masked-by-default export, still
   audited — are declared in the entity spec
   [§9.4](../../spec/09-api-surface.md) and
   [§10.4](../../spec/10-persistence.md).

## 7. Non-functional requirements

loco-idiomatic; Postgres persistence; deterministic matching via the
embedded library; soft-delete with audit-friendly timestamps.

## 8. Architecture

loco `App` (`src/app.rs`) registers the care-pathways controller. One
`care_pathways` table stores `pid` + denormalised `name` + the full
`CarePathway` JSONB `data`. Matching calls `care-pathway-matcher`
directly on the deserialised payloads — no adapter.

## 9. API surface

See §6. Raw loco JSON. `404` for unknown `pid`; `422` for a validation
failure (blank `name`, a `condition_codes` entry malformed for its
coding system, an `identifiers` entry malformed for its scheme, or an
`in_language` tag that is not valid BCP-47 — family convention, via
`Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`, with every
problem reported in one body); `400` for a malformed body.

**Auth.** Every route may carry `Authorization: Bearer v4.public.…`
(offline PASETO v4 public verification against the auth-service's
published Ed25519 key); handlers take `MaybeAuthUser` to stamp the audit
`actor`. Blanket `/api/*` enforcement is wired (an `after_routes`
middleware calling `auth::enforce`) but **off by default** — gated by
`CARE_PATHWAY_REQUIRE_AUTH` (`1`/`true`/`yes`/`on` ⇒ on). When on, any
`/api/*` route without a valid token is `401`; the public paths
`/_health`, `/_ping`, `/api-docs/openapi.json`, `/swagger-ui*`, and
`/metrics.prom` stay open (matching §6.12 and
`src/auth.rs::is_public_path`). The paseto-keys / issuer / audience come
from `CARE_PATHWAY_PASETO_KEYS` / `CARE_PATHWAY_TOKEN_ISSUER` /
`CARE_PATHWAY_TOKEN_AUDIENCE`. When `CARE_PATHWAY_PASETO_KEYS_URL` is
set, the key set is instead **fetched over HTTP once at boot**
(`Verifier::from_paseto_keys_url`, typically the auth-service
`/.well-known/paseto-keys`; seeded from `App::after_routes` via
`auth::init_from_env`): the fetched set wins over
`CARE_PATHWAY_PASETO_KEYS`; on fetch failure the service warns and falls
back to the env key set, so it always boots. No refresh loop — periodic
re-fetch on key rotation is a future item (§16). See the family contract
`agents/share/jwt-enforcement.md`; the session / token model is fixed by
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md),
which supersedes the prior RS256-JWT model.

**Authorization (ABAC).** Inside the same guard — so only when
`CARE_PATHWAY_REQUIRE_AUTH` is on — a verified token is authorized by
**attribute-based access control** per
[`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md):
the request's action is derived from the HTTP method plus this crate's
destructive named POSTs (`auth::DESTRUCTIVE_POST_SUFFIXES` — `/merge`,
`/deduplicate`, `/import`; the latter two ahead of the dedup-scan and
bulk-import features), and the shared engine in
`authentication-verifier` 0.3 evaluates the policy over the token's
`attrs` claim, first-match-wins. Configure with `CARE_PATHWAY_ABAC_POLICY`
(inline JSON) or `CARE_PATHWAY_ABAC_POLICY_FILE` (path); unset or
unparsable ⇒ warn-log + the built-in default policy (any authenticated
subject reads; `access=write` writes; `access=admin` adds DELETE/merge;
`svc=true` does everything). `401` = missing/bad credential; `403` =
valid credential, policy denied (the body names the deciding rule). This
supersedes the earlier per-crate roles/RBAC sketch.

## 10. Persistence

PostgreSQL via SeaORM + `sea-orm-migration`. Migrations
`m20220101_000001_care_pathways` (the `care_pathways` table),
`m20220101_000002_audit_logs` (the CRUD `audit_logs` trail),
`m20220101_000003_merge_records` (record-merge history), and
`m20260725_000007_compliance` (the `audit_logs` compliance columns —
`prev_hash` / `hash` for the tamper-evident chain, `context` for the
per-access purpose-of-use and standing declarations, `disclosure` for
the §164.528 access/disclosure split, `redacted_at` for GDPR Art. 17,
plus an `entity_pid` index). Every added column is nullable or
defaulted, so rows written before the migration stay valid and are
reported by chain verification as `unchained` rather than as breaks.
`auto_migrate` on in development.

**`audit_logs` is append-only, with exactly one documented exception.**
Helpers only insert and query; the sole statement that modifies an
existing row is `Model::redact_for_entity` (the Art. 17 erasure path),
which destroys `snapshot` and stamps `redacted_at` while leaving `hash`
and `prev_hash` intact so the chain still verifies.

## 11. Testing strategy

DB-free tests: `tests/matching.rs` (matcher embedding + JSON
round-trip), the `src/validation.rs` unit tests (ICD-10 / ICD-11 /
SNOMED-Verhoeff code formats, UUID / DOI identifier shapes, and BCP-47
`in_language` syntax), the `src/auth.rs` unit tests (mint a
real PASETO v4 public token + matching Ed25519 key in-process, then assert
valid → claims
and missing / non-bearer / expired / tampered / empty-verifier → `401`;
plus `parse_bool` cases and `enforce` — off+no-token → `Ok`, on+public →
`Ok`, on+protected+{no/valid/expired/tampered} token → `401`/`Ok`; plus
the boot-time key-set fetch — a local ephemeral-port HTTP listener
serving the test key set proves the fetch-built verifier accepts a token
signed by that key, a fast-failing URL proves the env fallback without
panic, and no-URL pins the plain env path),
the `src/merge.rs` unit tests (former-title alias, scalar fallback, list
union, transferred snapshot), the `escape_like` unit test (search
wildcard neutralisation), the `src/metrics.rs` unit tests (the rendered
Prometheus text carries every metric name plus the `# HELP`/`# TYPE`
preamble, and the content type is `text/plain; version=0.0.4`), and
controller validation unit tests
(blank-name and malformed-code → `422` pins, plus an `is_self_merge`
equal-pid pin for the §6.8 self-merge `422` guard).
Request-level tests (`tests/requests/care_pathways.rs`,
loco testing harness) cover the CRUD + match endpoints, unknown-pid
`404` on GET / PUT / DELETE (and the merge `404`), the audit/event
trail, `whoami` (no token → `401`), blanket enforcement (with
`CARE_PATHWAY_REQUIRE_AUTH=1` set in-test: un-authed `GET
/api/care-pathways` → `401`, public `GET /api-docs/openapi.json` →
`200`; `#[serial]`), and OpenAPI/Swagger but require
Postgres, so they are `#[ignore]`-gated — run with
`cargo test -- --ignored` and a `DATABASE_URL`.

**Compliance tests.** DB-free unit tests cover the pure cores: the hash
chain (determinism, per-field coverage, and the four break scenarios —
edit, delete, reorder, redact — plus the pre-chain-rows boundary), the
purpose-of-use vocabulary and header sanitisation, the erasure tombstone
and context, the safety-class and cross-border logic, the SOUP/SBOM
parsers, the bulk NDJSON and job registry, and the FHIR profile +
terminology validators.
[`tests/traceability.rs`](../tests/traceability.rs) additionally runs
un-gated, failing the build when a requirement in
[`compliance/traceability.tsv`](../compliance/traceability.tsv) names a
test that no longer exists.

`tests/requests/compliance.rs` is DB-gated and carries the checks a unit
test **cannot** make. The load-bearing one is
`chain_survives_a_jsonb_round_trip`: a digest computed in Rust before an
`INSERT` must still match after Postgres has stored the snapshot as
`jsonb` (reordering keys) and returned `created_at` as a `timestamptz`.
`tampering_with_a_row_breaks_verification` rewrites a snapshot with raw
SQL and asserts the chain reports a `content` break — the property the
whole design exists to provide. The rest cover erasure (content gone,
chain still verifying, scoped to its subject, idempotent), the
disclosure accounting's completeness caveat, the posture and SBOM
endpoints, `$validate`, the SMART 404, the `CapabilityStatement`, and
the full Bulk Data kickoff → status → NDJSON → cancel flow.

## 12. Compliance

Care pathways are clinical artefacts, not patient data — but the audit
trail, the instance layer, and every disclosure decision are governed
all the same. This service is the family's **reference implementation**
of the four control-driving frameworks in
[`agents/share/compliance-for-healthcare.md`](../../../agents/share/compliance-for-healthcare.md)
§2; the entity-level engagement analysis is
[entity spec §12.4–§12.5](../../spec/12-compliance.md) and the
repository-wide status is
[`spec/compliance` §8](../../../spec/compliance/index.md). Code lives in
[`src/compliance/`](../src/compliance/), [`src/fhir/profile.rs`](../src/fhir/profile.rs),
and the crate-root [`compliance/`](../compliance/) artefacts.

### 12.1 HIPAA — tamper-evident history and read/disclosure auditing

- **Hash chain** ([`src/compliance/audit_chain.rs`](../src/compliance/audit_chain.rs)).
  Each `audit_logs` row stores a SHA-256 over its own content **and its
  predecessor's hash**, so inserting, deleting, reordering, or editing a
  row breaks verification there and everywhere after
  (§164.312(c)(1)–(2)). Two properties make the digest reproducible from
  the row as Postgres returns it: time is hashed as **epoch microseconds**
  (truncated on write, so the session time zone and subsecond precision
  cannot change it), and JSON is hashed via `serde_json`'s serialization,
  whose `BTreeMap` key order matches what a JSONB round-trip yields.
  `GET /api/compliance/audit/verify` reports `intact` / `redacted` /
  `unchained` counts, every break with its row id and kind
  (`linkage` / `content`), and the chain **head** an operator can record
  externally to detect wholesale truncation.
- **Append serialisation.** `Model::record_with_context` takes
  `pg_advisory_xact_lock` before reading the head and inserting. Under
  `CARE_PATHWAY_EVENT_TRANSPORT=outbox` the audit row shares the entity
  mutation's transaction, so appends are fully serialised. Under
  `memory` the audit write is a best-effort side channel on a pooled
  connection, where the lock is released immediately and concurrent
  writers can fork the chain — reported as a `linkage` break. **A
  compliance deployment should run `outbox`**; the verification
  response's `interpretation` field names this cause explicitly so an
  operator is not sent chasing an intrusion that was a concurrency
  artefact.
- **Read-auditing** ([`src/compliance/disclosure.rs`](../src/compliance/disclosure.rs)).
  `CARE_PATHWAY_AUDIT_READS` (**default off**, so adoption is
  behaviour-neutral) writes an audit row for `read` / `list` / `search` /
  `export` / `fhir_read` / `fhir_search`. The caller declares context in
  headers — `X-Purpose-Of-Use` (normalised against a closed vocabulary,
  never echoed, so a header cannot inject text into the trail),
  `X-Disclosure-Recipient`, `X-Destination-Region` — and the row records
  the declaration **plus** the deployment's standing declarations, so it
  stays interpretable years later. A collection read is recorded against
  the nil `pid`, so it cannot corrupt any single record's accounting.
- **Accounting of disclosures** (§164.528).
  `GET /api/care-pathways/{pid}/audit/disclosures` returns only
  disclosure-classified rows, and states whether the accounting is
  complete or `INCOMPLETE` because read-auditing is off — an empty list
  must not read as "nothing was disclosed".

**Row-level integrity** ([`src/compliance/record_integrity.rs`](../src/compliance/record_integrity.rs)).
The chain covers the trail; this covers the records. Every
`care_pathways` row carries a `content_hash` — SHA-256 over its `pid`,
`name`, payload, `active` flag and `deleted_at` — recomputed on **every**
write. Because the hash is set inside the three model helpers
(`create` / `update_data` / `soft_delete`) plus the erasure path, no
caller can forget it. `GET /api/compliance/records/verify` recomputes and
names any row changed outside the service.

It is **not** a chain: entity rows are mutable by design, so this detects
out-of-band *modification*, not deletion or reordering — the audit chain
covers those, and neither control subsumes the other. `created_at` /
`updated_at` are deliberately **excluded** from the digest: they are set
by the ORM and the database rather than by this code, so binding them
would produce false mismatches. An attacker who alters only a timestamp
is not detected; anything that changes what the record *says* is. Rows
predating the column verify as `unhashed` rather than as mismatches, and
are rehashed on their next write.

### 12.2 GDPR / EU EHDS — erasure, residency, lawful basis

- **Erasure against the immutable chain**
  ([`src/compliance/erasure.rs`](../src/compliance/erasure.rs)).
  `POST /api/care-pathways/{pid}/erase` tombstones the payload (a valid,
  data-free `CarePathway`, so read paths degrade cleanly), soft-deletes
  the record, redacts every audit `snapshot` about it, and appends a
  chained `erased` row. Redaction preserves each row's `hash` and
  `prev_hash`, so the chain still verifies and still proves the events
  occurred. `actor` and `action` survive on purpose — the controller's own
  accountability record under Art. 17(3)(b). Irreversible and idempotent:
  re-erasing, or erasing an already-soft-deleted `pid`, still sweeps any
  audit content held about it, because the subject's right does not lapse
  when the record is retired. **Destructive** under ABAC (`/erase` is in
  `DESTRUCTIVE_POST_SUFFIXES`), so `access=write` cannot reach it.
- **Declarations** ([`src/compliance/mod.rs`](../src/compliance/mod.rs)).
  `CARE_PATHWAY_DATA_RESIDENCY`, `_LAWFUL_BASIS`, `_ART9_CONDITION`, and
  `_TRANSFER_SAFEGUARD` default to `undeclared` rather than to a
  flattering value, are reported at `GET /api/compliance`, and are
  stamped into every audit `context`.
- **Cross-border transfer.** An access naming a destination outside the
  declared region is recorded as `cross_border: true`. Detection is
  conservative — undeclared residency or an unnamed destination never
  manufactures a transfer event. This **declares and records**; it does
  not block. Blocking is a deployment-network decision.
- **EHDS primary vs. secondary use.** The purpose vocabulary separates
  care delivery (`care` / `treatment` / `payment` / `operations` /
  `public-health`) from secondary use (`research` / `policy` /
  `statistics`), which is also classified as a disclosure.

### 12.3 ONC / HTI — profile and terminology conformance

- **Profile** ([`src/fhir/profile.rs`](../src/fhir/profile.rs)). Every
  rendered resource carries `meta.profile` = a **family-local**
  `StructureDefinition` canonical (`urn:mxi:carepathway:…`), never a US
  Core one — `PlanDefinition` has no US Core profile and the family
  serves R5. `validate_profile` checks must-support elements and
  cardinalities (`title`, `status`, `identifier.system`/`.value`,
  `useContext`, `action.title`, `relatedArtifact.url`) and the `status`
  required binding.
- **Terminology.** Condition codes are validated against the value set
  their system **binds** (ICD-10 / ICD-11 / SNOMED CT), reusing
  `validation::condition_code_issue` — so `"code": "banana"` is an error,
  not merely well-formed JSON. An **unbound** system warns instead of
  failing, because the conversion contract deliberately preserves foreign
  namespaces.
- **`$validate`, SMART, Bulk Data.** `POST /fhir/PlanDefinition/$validate`
  returns `200` with an `OperationOutcome` (an `information` issue when
  clean) and persists nothing. SMART discovery is served **only** when
  `CARE_PATHWAY_SMART_AUTHORIZATION_URL` + `_TOKEN_URL` are set, else
  `404` explaining that this service authenticates with PASETO; the
  `CapabilityStatement`'s `security` block appears on the same condition.
  Bulk Data implements the IG's async shape faithfully, while the
  execution model is in-process and bounded (§12.5).
- **Not certification.** See §12.5.

### 12.4 IEC 62304 / SaMD — lifecycle evidence

Declared in [`compliance/lifecycle.md`](../compliance/lifecycle.md).
Safety classification (`CARE_PATHWAY_SAFETY_CLASS`, default **A** for the
template registry, with the re-classification trigger stated) and build
provenance are reported at `GET /api/compliance`. The **SOUP register**
([`compliance/soup.tsv`](../compliance/soup.tsv)) annotates every direct
dependency; the **SBOM** merges it with the crate's own `Cargo.lock`,
embedded at compile time so it cannot drift from the binary, and is
served at `GET /api/compliance/sbom` (and by `cargo run --bin sbom`).
Rendering is deterministic — no timestamp, no serial number — so a
reproducible build yields a byte-identical SBOM.
[`compliance/traceability.tsv`](../compliance/traceability.tsv) maps each
compliance and safety-relevant requirement to the tests that verify it,
**machine-checked** by [`tests/traceability.rs`](../tests/traceability.rs).
[`scripts/build-reproducible.sh`](../scripts/build-reproducible.sh) pins
the toolchain, derives `SOURCE_DATE_EPOCH` from the commit, and can build
twice and compare hashes; [`scripts/sbom.sh`](../scripts/sbom.sh) gathers
the evidence bundle.

### 12.5 Honest limits

- **Not a certified health-IT module and not a registered medical
  device.** Neither is claimed anywhere in the code or the API. The
  posture endpoint's per-framework `not_claimed` lists are asserted by
  tests, so a future edit cannot quietly turn the report into marketing.
- **Chain scope, now narrowed.** The chain attests to the **audit
  trail**; row-level integrity is a separate, complementary control
  (§12.1). Between them the remaining gap is a row **deleted outright**
  in SQL: the content hash cannot see a row that is not there, and only
  the audit chain — which a legitimate delete writes to — covers that.
  Both verification responses say what they do and do not attest to.
- **Audit-write failure is a deployment choice, now explicit.**
  `CARE_PATHWAY_AUDIT_FAIL_CLOSED` (**default off**) decides what happens
  when a read-audit write fails: off logs and serves the read (the
  family's best-effort posture, and today's availability profile); on
  refuses it with `503`, disclosing nothing the service cannot account
  for. A HIPAA-facing deployment should set it — an accounting of
  disclosures that silently omits disclosures is worse than an outage,
  because the outage is visible. Mutation audits are already fail-closed
  under `CARE_PATHWAY_EVENT_TRANSPORT=outbox`, where the audit row shares
  the mutation's transaction.
- **Bulk export is in-process.** Jobs do not survive a restart, are not
  visible to another replica, are capped at 8 concurrent / 10 000
  resources / 8 MiB, and expire after 15 minutes. A truncated export
  declares itself in the manifest's `error` array. Moving to `bg_pg` +
  an artifact store is the upgrade path (§13 T-10).
- **Signing keys are out of scope** for the build script — a deployment
  secret, signed in the release pipeline.
- **No ISO 14971 risk file, DPIA, Art. 30 record, EHDS data permit, or
  Inferno run.** Organisational or infrastructure artefacts; the service
  supplies the technical controls they cite.

## 13. Tasks (live work queue)

- [x] Name search — `GET /search?q=` Postgres `ILIKE` on the
  denormalised `name` (cap 50, wildcards escaped). Tantivy full-text /
  fuzzy search over the JSONB payload remains deferred.
- [x] Event streaming + audit log on CRUD — `audit_logs` table +
  best-effort row per create/update/delete (`models/audit_logs.rs`);
  in-memory event stream (`streaming.rs`); read at
  `/audit/recent`, `/{pid}/audit`, `/events/recent`. **Phase 1** of the
  durable event bus is implemented: the canonical versioned `Envelope`
  (`event_id`, `schema_version` 1, `entity`, `kind`, `pid`, `seq`,
  `actor`, `name`) plus the `EventPublisher` trait seam with an
  `InMemoryPublisher` ring buffer; `/events/recent` returns the frozen
  `EventView` projection (`{kind, pid, name, seq}`), byte-identical to the
  previous wire shape. **Phase 2** (transactional outbox) is implemented:
  `CARE_PATHWAY_EVENT_TRANSPORT=outbox` writes one `event_outbox` row on
  the entity mutation's transaction (`streaming.rs`; default `memory`).
  **Phase 3** (relay + retention) is implemented — see the dedicated
  Phase-3 item below. Only the real Fluvio broker sink remains
  (broker-gated), designed in
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md);
  `actor` is wired through `publish_with_actor`.
- [x] **Durable event bus — Phase 3 (relay + retention).** `src/relay.rs`:
  the `EventSink` trait (the bus seam), a working no-broker **`LoggingSink`**
  default, `drain_once` (`unpublished` → `sink.send` → `mark_published`,
  at-least-once, per-pid order preserved on a send failure), and
  `purge_published` (retention: deletes `published_at < now() -
  INTERVAL '<CARE_PATHWAY_EVENT_RETENTION_DAYS> days'`, default 7). A
  background loop (`relay::spawn`, started in `App::after_routes`) ticks
  every `CARE_PATHWAY_EVENT_RELAY_INTERVAL_SECS` (default 5, floored at 1)
  and purges every N ticks — **gated by `CARE_PATHWAY_EVENT_TRANSPORT=outbox`
  AND `CARE_PATHWAY_EVENT_RELAY`** (truthy `1`/`true`/`yes`/`on`), so it is
  a no-op by default. Tests: DB-free `LoggingSink`/capturing-sink send +
  config defaults; the drain/ack seams (`unpublished`/`mark_published`) are
  DB-gated-tested via the outbox suite. **Broker-gated follow-up:** a real
  **`FluvioSink`** (`impl EventSink` behind a `fluvio` cargo feature) — the
  trait is the seam, so the drain loop is unchanged when it lands
  ([`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §5, §8).
- [ ] Privacy controls if any restricted fields appear.
- [x] Record merge — `POST /merge` folds a duplicate into a survivor
  (union fields, former-title alias, soft-delete, `merge_records`
  history + snapshot, `Merged` event); pure `src/merge.rs`;
  `/merges/recent`. Front-end merge action is a follow-up.
- [x] OpenAPI/Swagger — hand-written `src/openapi.rs` (matcher DTO is
  dependency-light, so no utoipa, matching the organization service)
  served at `/api-docs/openapi.json` + `/swagger-ui` by
  `controllers/docs.rs`.
- [x] Prometheus metrics — `GET /metrics.prom` (root path,
  `text/plain; version=0.0.4`) for parity with the older Axum services.
  Process-wide `OnceLock` registry in `src/metrics.rs`
  (`care_pathway_created_total` / `_updated_total` / `_deleted_total` /
  `_merged_total` counters + `http_requests_total` `IntCounterVec`);
  handler in `controllers/metrics.rs`, mounted at the root like
  `controllers/docs.rs` and added to `auth::is_public_path` so it stays
  open under blanket enforcement. The CRUD/merge controllers
  increment one counter per success path.
- [x] Richer validation (ICD/SNOMED code formats, identifier shapes,
  language tags) — `src/validation.rs` format-checks `condition_codes`
  per `system` (ICD-10, ICD-11, SNOMED CT SCTID Verhoeff), `identifiers`
  per `scheme` (canonical UUID for `Uuid`, `10.…/…` shape for `Doi`,
  non-blank for the rest), and `in_language` for BCP-47 syntax; `422`
  with all problems. Terminology-server / IANA-registry existence checks
  remain out of scope.
- [x] Request-level integration tests (Postgres) — landed
  `#[ignore]`-gated (entity spec §13 T-4), and the CI `test` job now runs
  them via a dedicated `cargo test ... -- --ignored` step against the
  provisioned Postgres service (`.github/workflows/ci.yaml`). Coverage
  includes unknown-pid `404` on GET / PUT / DELETE and the merge `404`.
- [x] Offline token verification — `src/auth.rs` embeds
  `authentication-verifier`; offline verification via a process-wide
  `Verifier` (env-configured keys/issuer/audience);
  `AuthUser`/`MaybeAuthUser` extractors; `/whoami` protected; audit
  `actor` stamped from the token. (Originally RS256-JWT against the
  auth-service JWKS; the credential has since been switched to PASETO —
  below.)
  - [x] Switch the credential RS256-JWT → **PASETO v4 public** per
    [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
    (supersedes the RS256-JWT + JWKS model). **Done:** `Verifier` verifies
    `v4.public.…` tokens against the auth-service's published Ed25519
    key; `from_paseto_keys_value` / `from_paseto_keys_url` replaced
    `from_jwks_*`; same `Claims` shape (`kid`/`iss`/`aud`/`exp`, `kid`
    in the footer); env vars `CARE_PATHWAY_PASETO_KEYS` /
    `CARE_PATHWAY_TOKEN_ISSUER` / `CARE_PATHWAY_TOKEN_AUDIENCE`.
  - [x] Blanket `/api/*` enforcement — pure `auth::enforce(require_auth,
    path, headers, verifier)` + an `axum::middleware::from_fn` layer in
    `app.rs after_routes`, wired unconditionally and gated per-request by
    `CARE_PATHWAY_REQUIRE_AUTH` (`auth::require_auth`, off by default;
    `1`/`true`/`yes`/`on` ⇒ on). Public paths (`/_health`, `/_ping`,
    `/api-docs/openapi.json`, `/swagger-ui*`) stay open. Family contract:
    `agents/share/jwt-enforcement.md` (credential now PASETO, semantics
    unchanged). Activation is an operations decision once the SSO token
    flow is live.
  - [x] paseto-keys-over-HTTP fetch from the auth service at boot.
    **Done 2026-07-04:** new `CARE_PATHWAY_PASETO_KEYS_URL` env var (§9);
    when set, `auth::init_from_env` (called from `App::after_routes`
    before serving) fetches the key set once via
    `Verifier::from_paseto_keys_url` (`authentication-verifier` `fetch`
    feature) and seeds the process-wide verifier — fetched set wins
    (`tracing::info!`); on fetch failure it warns and falls back to the
    `CARE_PATHWAY_PASETO_KEYS` env path, so the service always boots.
    Unset/blank URL ⇒ prior env-injection behaviour exactly. Fetch-once
    only; a periodic refresh loop on key rotation stays future work
    (§16). Tests: local ephemeral-port HTTP listener serving the test
    key set (fetched verifier accepts a token signed by that key) +
    fast-failing URL fallback (no panic) + no-URL env path (§11).
- [ ] Bulk import/export — `bulk_jobs` migration (shared doc §3 schema,
  `UNIQUE (entity, kind, idempotency_key)`); the five endpoints
  (§6.13: `POST`/`GET /api/care-pathways/import`,
  `POST`/`GET /api/care-pathways/export`,
  `GET /api/care-pathways/bulk-jobs`); `bg_pg` worker draining
  `queued → running → completed | completed_with_errors | failed`;
  JSONL/CSV/Parquet codecs (CSV flattening per entity spec §9.4 —
  every repeated/nested field a JSON-in-cell; Parquet export-only,
  feature-gated); per-row pipeline reusing `src/validation.rs` +
  the matcher + the review queue (upsert by a deterministic scheme-scoped
  identifier / `(provider_id, pathway_code)` / `pid`; keyless rows →
  duplicate detection → review queue, `provenance = import`; events +
  audit not bypassed); downloadable per-row error report
  (`row_number, source_line, field, code, message`); export masking
  (`masking_profile`, masked default) + `include_soft_deleted` gating +
  per-export audit (even zero-row). Uniform contract:
  [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md);
  entity-level detail: entity spec §9.4 / §10.4 / §13 T-10. Tests:
  idempotent re-import, per-row error report, keyless dedupe-to-review,
  masked vs full export, zero-row export still audited.
- [x] **FHIR R5 API** (`PlanDefinition`) — **Done** (`src/fhir/{mod,resources,search}.rs`
  + mounted `src/controllers/fhir.rs`, `routes()` in `app.rs`; 15 DB-free tests,
  `cargo test --lib` + `cargo clippy --lib` clean). Gaps: the DTO has no `status`
  field (the record's `active` flag is the source of truth — FHIR `status`/`type`
  are emitted but not carried back inbound); instantiated `CarePlan` remains
  roadmap. Original task text follows for reference. — adopt the family contract
  ([`agents/share/fhir.md`](../../../agents/share/fhir.md)). Map the stored
  `care_pathway_matcher::CarePathway` DTO to a FHIR **`PlanDefinition`**
  (§3, `medium` fidelity — a clinical pathway *template*): `name` →
  `title`, provider-scoped `pathway_code` (with `provider_id` /
  `provider_name`) → `identifier`, `condition_codes` (ICD/SNOMED) →
  `useContext` / action `condition`, `care_setting` → `useContext`,
  `interventions` → `action`, `keywords` → `useContext` / topic,
  `identifiers` / `same_as` → `identifier` / `relatedArtifact`, status →
  `status`, `type` = clinical protocol. An instantiated `CarePlan` is a
  roadmap alternative. New `src/fhir/` module (resource structs,
  `to_fhir_plan_definition` / `from_fhir_plan_definition`,
  `FhirOperationOutcome`, searchset `Bundle`, search-param parsing) + a
  mounted `src/controllers/fhir.rs` (`routes()` in `app.rs`) exposing
  read/create/update/delete/search at
  `/fhir/PlanDefinition{,/{id}}` + `GET /fhir/metadata`
  `CapabilityStatement`. Reuses the native model helpers,
  `src/validation.rs`, the event/audit path, and the blanket auth + ABAC
  guard (§8; `/fhir/*` guarded, action from HTTP method). Supported search
  params: `_id`, `_lastUpdated`, `_count`, `identifier`, `name`, `status`.
  Tests: DTO↔`PlanDefinition` round-trip, each interaction, search→Bundle,
  `OperationOutcome` on 404/400/422, `CapabilityStatement` matches routes.
- [x] **Input-size caps (SEC-M1).** `src/validation.rs` rejects oversized
  payloads before storage/matching (the O(n·m) matcher over unbounded
  text/arrays is a DoS, amplified by `check-duplicates`): `MAX_TEXT_LEN`
  1024 per free-text field, `MAX_ARRAY_LEN` 256 per array, `MAX_ITEM_LEN`
  512 per string array entry — all collected as `422` problems. DB-free
  tests for oversized field/array/entry + a within-caps large record.

- [x] **Fix: fresh-Postgres `db migrate` failed in the `event_outbox`
  migration (2026-07-18).** The loco `create_table` helper pluralizes
  table names (`cruet::to_plural`: `event_outbox` → `event_outboxes`),
  so the migration's own index DDL (`ON event_outbox`) failed and
  rolled the whole fresh migrate back — no tables were ever created.
  The migration is now explicit SQL creating exactly `event_outbox`
  (matching the `SeaORM` entity), `IF NOT EXISTS`-guarded; same
  migration name (the old form could never have applied anywhere).
  Found and fixed family-wide from the patient-flow implementation
  round; verified by a live fresh-database migrate. Every other table
  this crate creates via the helper is already plural (no-op).

- [x] **2026-07-20 — Registry insight views.** Five read-only
  derived views (`controllers/insights.rs`, prefix
  `/api/care-pathways/insights`) over the stored `CarePathway`
  templates, for the provider / setting / coverage lenses:
  `GET /directory` (faceted by `care_setting` + the `specialty:<x>`
  keyword convention), `/coverage` (per condition code, which settings
  have a pathway + disclosed gap rules: no primary-care / no emergency
  pathway), `/variants` (a condition offered by ≥2 providers, with the
  `jurisdiction:<x>` facet — a comparison directory, never a match
  signal), `/providers` (pathways per issuing provider by setting),
  `/languages` (per-language counts + the single-language-condition
  equity lens). No migration, no matcher change: facets come from
  existing DTO fields plus two disclosed keyword conventions.
  **Acceptance:** the seeded five-view request round-trip green first
  run — full `--ignored` suite 23/23 vs Postgres 18; clippy pedantic
  clean.

- [x] **2026-07-20 — Instance layer (operational pathways).** A new
  operational layer over the registry: a patient **enrolled** on a
  pathway template. Migration `m20260720_000005_instances`
  (`pathway_instances` referencing a `person:` URN + the template pid,
  `instance_steps`, `instance_team`, `instance_events`). Deliberately
  **not** in the matcher payload — the registry owns pathway
  identities. `controllers/instances.rs`: enrol (copies declared
  steps), the `active`↔`on_hold`→terminal lifecycle (pure machine in
  `src/instances.rs`; closing stamps `closed_on`), the review cadence
  (`POST /review` reschedules `next_review_on` + logs a review event),
  urgency escalation (routine/urgent/emergency; logs escalation /
  de-escalation), step completion, the care-team roster (worker /
  person / organization URNs + roles), and free events. Derived views:
  `GET /api/instances/caseload` (open by setting + urgency),
  `/overdue-reviews` (chronic review register), `/care-team-load`
  (open load per member), and `GET /api/care-pathways/{pid}/cohort`
  (chronic cohort by status/urgency + step completion). **Acceptance:**
  lifecycle pure pins; the seeded instance round-trip green first run —
  full `--ignored` suite 24/24 vs Postgres 18; clippy pedantic clean.

- [x] **2026-07-20 — Instance outcomes.** Migration
  `m20260720_000006_outcomes` (`pathway_instances.outcome` recorded at
  close; `instance_measures` for recorded clinical / PROM measures).
  Closing an instance now accepts a validated `outcome`
  (`rules::OUTCOMES`); `POST /api/instances/{pid}/measures` records a
  numeric or text measure; `GET /api/care-pathways/{pid}/outcomes`
  serves the closed-instance outcome distribution (declared outcomes
  only, unrecorded counted separately) + per-measure latest-value
  averages. The honest, record-only basis for outcome analytics.
  **Acceptance:** the extended instance round-trip green — full
  `--ignored` suite green vs Postgres 18; clippy pedantic clean.

- [x] **2026-07-25 — T-11 HIPAA: tamper-evident history + read/disclosure
  auditing.** Migration `m20260725_000007_compliance` adds
  `prev_hash`/`hash`/`context`/`disclosure`/`redacted_at` plus an
  `entity_pid` index. `src/compliance/audit_chain.rs` (pure SHA-256
  chain, time hashed as epoch microseconds, JSON canonicalised so a
  JSONB round-trip cannot change the digest, redaction-tolerant
  verification); `src/models/audit_logs.rs` chains every append under
  `pg_advisory_xact_lock`; `src/compliance/disclosure.rs` (purpose-of-use
  vocabulary, header sanitisation, access-vs-disclosure classification,
  best-effort recording gated by `CARE_PATHWAY_AUDIT_READS`, default
  off). Read-auditing wired into `get_one` / `list` / `search` and the
  FHIR `read` / `search`. New endpoints
  `GET /api/compliance/audit/verify` and
  `GET /api/care-pathways/{pid}/audit/disclosures` (which states its own
  completeness). **Acceptance:** DB-gated
  `chain_survives_a_jsonb_round_trip` and
  `tampering_with_a_row_breaks_verification` green vs Postgres 18, plus
  11 chain unit tests covering edit / delete / reorder / redact.

- [x] **2026-07-25 — T-12 GDPR / EHDS: erasure, residency, lawful basis.**
  `src/compliance/erasure.rs` — `POST /api/care-pathways/{pid}/erase`
  tombstones the payload, soft-deletes, redacts audit content while
  preserving chain linkage, and appends a chained `erased` row;
  irreversible and idempotent (an unknown or already-erased `pid` still
  sweeps its audit content). Added to `DESTRUCTIVE_POST_SUFFIXES`, so
  `access=write` cannot reach it. Deployment declarations (residency,
  lawful basis, Art. 9 condition, transfer safeguard) default to
  `undeclared`, are reported at `GET /api/compliance`, and are stamped
  into every audit `context`; an access naming a destination outside the
  declared region is recorded as a Ch. V transfer (conservatively — never
  asserted without both). EHDS primary/secondary use separated by the
  purpose vocabulary. **Acceptance:**
  `erasure_destroys_content_but_keeps_the_chain_verifiable` and
  `erasure_is_idempotent` green vs Postgres 18.

- [x] **2026-07-25 — T-13 ONC / HTI: profile + terminology conformance.**
  `src/fhir/profile.rs` — a family-local declared profile stamped into
  `meta.profile` on every rendered resource, must-support / cardinality
  checks, the `status` required binding, and terminology validation
  against the **bound** condition-code systems (unbound systems warn
  rather than fail). `POST /fhir/PlanDefinition/$validate`;
  `GET /fhir/.well-known/smart-configuration` served only when the
  deployment configures an authorization server (else `404` explaining
  the PASETO credential); `CapabilityStatement` extended with the
  profile, the operations, and a conditional SMART `security` block;
  FHIR Bulk Data `$export` → `$export-status/{id}` → NDJSON `$export-file`
  + cancel (`src/compliance/bulk.rs`, in-process and bounded). Discovery
  paths added to the public allow-list. **Acceptance:**
  `fhir_validate_checks_profile_and_terminology`,
  `capability_statement_declares_profile_and_operations`,
  `smart_discovery_is_absent_unless_configured`, and
  `bulk_export_kickoff_status_and_output` green vs Postgres 18.

- [x] **2026-07-25 — T-14 IEC 62304 / SaMD: lifecycle evidence.**
  `compliance/lifecycle.md` (development-plan record, safety
  classification with its re-classification trigger, clause→artefact
  index, and an explicit "what is not here"); `compliance/soup.tsv` (the
  §8.1.2 register, one annotated row per direct dependency);
  `src/compliance/soup.rs` merging it with the crate's own `Cargo.lock`,
  embedded at compile time so the SBOM cannot drift, served at
  `GET /api/compliance/sbom` and by `cargo run --bin sbom` (deterministic
  — no timestamp, no serial number); `compliance/traceability.tsv` +
  `tests/traceability.rs` (machine-checked requirement→test mapping,
  un-gated); `scripts/sbom.sh` and `scripts/build-reproducible.sh`.
  `GET /api/compliance` reports software identification, build
  provenance, live control state, the data-protection declarations, and
  per-framework **not-claimed** lines. **Acceptance:** adding a
  dependency without annotating it fails the build
  (`every_direct_dependency_is_annotated`); a renamed test orphaning a
  requirement fails the build (`every_named_test_exists`); 572-component
  SBOM rendered from the real lockfile with 26 annotated direct
  dependencies.

- [ ] **T-15 — Compliance follow-ups (deferred, honest).**
  - [x] **Row-level integrity hashing over `care_pathways`.** **Done
    (2026-07-25):** migration `m20260726_000008_record_integrity` adds
    `content_hash`; `src/compliance/record_integrity.rs` holds the pure
    hash + verify; the digest is set inside the model write helpers and
    the erasure path so no caller can omit it;
    `GET /api/compliance/records/verify` reports mismatches. Pinned by 11
    unit tests and, DB-gated, by `out_of_band_record_edit_is_detected`
    (raw-SQL edit is caught) and `every_write_path_rehashes`
    (create/update/delete/merge/erase all stay verifiable).
  - [ ] Move Bulk Data `$export` onto the `bg_pg` worker + an artifact
    store, so jobs survive a restart and are visible across replicas
    (§12.5); folds into T-10.
  - [x] **Decide whether a read-audit write may fail open.** **Done
    (2026-07-25):** resolved as a deployment switch rather than a library
    default — `CARE_PATHWAY_AUDIT_FAIL_CLOSED`, default off (behaviour
    neutral), with fail-closed refusing the read as `503` on both the
    native and FHIR surfaces. `record_access` now returns
    `Result<(), AuditWriteRefused>` so the choice is explicit at every
    call site rather than swallowed. §12.5 records the recommendation.
  - [x] **Wire `cargo deny`, the SBOM, and the traceability check into
    CI.** **Done (2026-07-25):** the repository had no CI at all;
    `.github/workflows/ci.yml` + `.woodpecker.yml` now run fmt, clippy
    (`-D warnings`), test, DB-gated test, `cargo deny` and the SBOM
    render on both remotes, driven by `scripts/ci-check.sh` so the two
    platforms run identical commands. The traceability and SOUP gates
    are ordinary tests, so the `test` stage already enforces them. See
    the repository `AGENTS.md` §Continuous integration.
  - [ ] Lift the `compliance/` artefacts to the repository root once a
    second crate adopts them
    ([`spec/compliance` §8.5](../../../spec/compliance/index.md)).
  - [ ] Run an Inferno-style conformance suite against `/fhir`.

## 14. Implementation status

Done: loco boot; care_pathways table + migration; CRUD with `422`
validation on create/update (blank `name`; ICD-10 / ICD-11 / SNOMED CT
`condition_codes` format checks; UUID / DOI `identifiers` shapes; BCP-47
`in_language` syntax — all problems reported together);
`ILIKE` name search; `/match`, `/check-duplicates`, and `/merge`
(record merge + history)
embedding care-pathway-matcher; audit log + in-memory event streaming on
every CRUD/merge (`/audit/recent`, `/{pid}/audit`, `/events/recent`,
`/merges/recent`) — Phase 1 of the durable event bus (canonical
`Envelope` + `EventPublisher` seam + `InMemoryPublisher`; frozen
`EventView` projection on `/events/recent`); offline **PASETO v4 public**
verification (`AuthUser`/`MaybeAuthUser`, `/whoami`, audit `actor` from
the token — credential switched from RS256-JWT per §13), including the
boot-time paseto-keys-over-HTTP fetch (`CARE_PATHWAY_PASETO_KEYS_URL`,
fetch-once, env fallback; §9/§13); OpenAPI 3 doc
+ Swagger UI (`/api-docs/openapi.json`, `/swagger-ui`); a root-level
Prometheus `/metrics.prom` endpoint (CRUD/merge counters +
`http_requests_total`, public under enforcement); blanket `/api/*`
enforcement middleware (`auth::enforce` + `after_routes` layer,
off by default via `CARE_PATHWAY_REQUIRE_AUTH`); DB-free tests +
gated request-level tests; green build + clippy.

## 15. Roadmap

All of the scope below shipped together in the still-unreleased `0.1.0`
line (Cargo.toml is `0.1.0`; CHANGELOG keeps it under `[Unreleased]`):
the CRUD + matching MVP, then `ILIKE` search + audit + in-memory
streaming, then record merge + OpenAPI/Swagger + Prometheus + offline
bearer-token verification + blanket `/api/*` enforcement middleware. The
original v0.2 / v0.3 milestone split was never cut as a tagged release.
The credential switch RS256-JWT → PASETO v4 public per
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
has since landed (§13), as has the boot-time paseto-keys-over-HTTP fetch
(`CARE_PATHWAY_PASETO_KEYS_URL`, fetch-once, env fallback; §9/§13). Next
(deferred, §13): Tantivy full-text/fuzzy search, the durable event bus's
real Fluvio broker sink (Phases 2–3 — outbox + relay/retention — are done;
only the broker-gated `FluvioSink` remains), a PASETO key-set refresh loop,
privacy, front-end merge action.

## 16. Open questions

- Normalise condition codes / interventions into their own tables once
  search lands?
- Real-time duplicate check on create (409) vs the explicit endpoint?
- Periodic re-fetch of the PASETO key set (key rotation) — the boot
  fetch (§9 `CARE_PATHWAY_PASETO_KEYS_URL`) runs once; is a refresh
  loop (or refetch-on-`UnknownKid`) needed before rotation goes live?

## 17. References

- The care-pathway-matcher spec; loco.rs; ICD-10 / SNOMED CT.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
