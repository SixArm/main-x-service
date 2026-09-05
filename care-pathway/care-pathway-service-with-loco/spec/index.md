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

MVP: CRUD + Tantivy full-text/fuzzy/phonetic search + matching + record merge + audit log +
in-memory event streaming (durable-bus Phase 1) + OpenAPI/Swagger +
Prometheus metrics + offline PASETO v4 public token verification + blanket
`/api/*` enforcement (off by default) + rich payload validation
(ICD/SNOMED/UUID/DOI/BCP-47) + field masking / GDPR export wired to the
ABAC `mask` obligation (§13 2026-08-02; thin by design — a pathway
*template* names no patient, see §12.2) + the durable event bus's real
`FluvioSink` broker sink (BUS-3, §13 2026-08-03; all three phases —
transactional outbox, relay/retention, and the real-broker sink — are
now done). Also in scope, landed later: **time-based analysis (TBA)** —
elapsed-calendar-time measurement of a patient's journey through the
pathway (value-adding ratio, cohort NHS-access-standard compliance,
ranked constraints, Little's-Law flow; §6.18, §13 T-13 2026-08-23) — and
the cross-service **`continues_as` journey edge**, which lets TBA follow
a journey across a service boundary instead of stopping at it (§6.19,
§13 2026-08-24 through 2026-08-27). Full design:
[`agents/share/time-based-analysis.md`](../../../agents/share/time-based-analysis.md)
and
[`../../spec/time-based-analysis.md`](../../spec/time-based-analysis.md).
Deferred (§13):
instance-layer masking/authz for `pathway_instances.subject_ref` (the
patient-identifying linkage — see §16),
terminology-server code-existence checks, gRPC, and the native
(non-FHIR) bulk import/export API (§13). The PASETO key-set refresh loop
and ABAC policy hot-reload are **done** (§9/§13, 2026-08-01) — a rotated
key or an edited policy reaches a running process without a restart.
Also done: keyed HMAC integrity MACs and external-witness chain
checkpoints (§12.1, 2026-07-27) closing the tail-truncation gap the hash
chain alone cannot see, and `?limit=`/`?offset=` pagination on list and
search (§6.2, 2026-08-01). Token
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
   `Doi`; other schemes non-blank), `in_language` checked for BCP-47
   syntax, and each `same_as` entry checked for an `http(s)://` scheme;
   `422` on any problem, all reported together — also enforced on
   update. Rules in [`src/validation.rs`](../src/validation.rs).
2. `GET /api/care-pathways` — list active, `{pid, name}`.
   `GET /api/care-pathways/search?q=` — Tantivy full-text search over
   name, alternate names, provider, identifiers, keywords, condition
   codes, and interventions (`?fuzzy=true` for typo tolerance,
   `?phonetic=true` for Soundex; blank `q` → `400`, an unavailable index
   → `503`). Both take `?limit=`/`?offset=` and report
   `X-Total-Count`/`X-Limit`/`X-Offset` response headers, per the family
   convention in
   [`agents/share/restful.md`](../../../agents/share/restful.md):
   defaults reproduce the old hard caps (100 list / 50 search), `limit`
   clamps to 500, and an `offset` past 10 000 is `400` (SEC-G7). The
   search total is a `COUNT(*)` over the predicate, not the page length.
3. `GET /api/care-pathways/{pid}` — return the stored `CarePathway`,
   unless the record-level ABAC decision carries the **`mask`
   obligation**, in which case the masked view (§6.16) is returned
   instead — see [`agents/share/authorization-attributes.md`](../../../agents/share/authorization-attributes.md) §9/§11.
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
   break. `GET /api/compliance/records/verify` — re-verifies row-level
   record content integrity (§12.1), naming any row changed outside the
   service. `GET /api/compliance/checkpoint` — takes a MAC'd
   external-witness statement of the chain's current head, position, and
   row count (§12.1); `POST /api/compliance/checkpoint/verify` takes one
   back and reports whether the chain still honours it, distinguishing
   an anchor row deleted from its content changed from earlier history
   shrunk. `GET /api/care-pathways/{pid}/audit/disclosures` — HIPAA
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
16. `GET /api/care-pathways/{pid}/masked` — the **masked view**:
   `provider_name` and `provider_id` masked to their tail; every clinical
   field (`name`, `condition_codes`, `interventions`, `keywords`,
   `identifiers`) untouched — redacting those would defeat the registry
   for no privacy gain, since a pathway *template* names no patient
   (§12.2; see §16 for the patient-identifying linkage this does **not**
   cover). `404` for an unknown `pid`.
17. `GET /api/care-pathways/{pid}/export` — **GDPR right-of-access**
   export: an envelope of `{entity, pid, exported_at, masked, record,
   note}`. **Every export is audited** as a disclosure
   ([`disclosure::action::EXPORT`](../src/compliance/disclosure.rs),
   HIPAA §164.528) whether or not it is masked, because extracting
   clinical data is itself a compliance event. A caller whose
   record-level ABAC decision carries the `mask` obligation gets the
   redacted record and `masked: true` — an access request answered with
   redactions must never look like a complete answer. `404` for an
   unknown `pid`; `503` when the export could not be recorded on the
   audit trail (`CARE_PATHWAY_AUDIT_FAIL_CLOSED`).
18. **Time-based analysis (TBA).** `POST`/`GET
   /api/instances/{pid}/segments` (+ `/segments/{seg}/close`, `/clock`)
   record a journey's segments (classified `value_adding` /
   `necessary_non_value_adding` / `unnecessary_non_value_adding`, with a
   stage and an optional VSM waste type) and the pathway's explicit
   clock (`start`/`stop`; deliberately **no** `pause` — a patient-caused
   delay is a visible, subtractable segment instead, per §12.3 of the
   umbrella doc below). `GET /api/instances/{pid}/{time-analysis,timeline}`
   derive, on every read (nothing is stored), the per-journey lead time,
   value-adding ratio, coverage, gaps and handoffs — the denominator is
   **elapsed calendar time**, never the sum of recorded activity.
   `GET /api/care-pathways/{pid}/{time-analysis,constraints}` roll a
   pathway's instances into nearest-rank lead-time percentiles against
   an NHS access standard (small cohorts, `<5`, withhold percentile
   detail) and a ranked list of constraint findings.
   `GET /api/instances/{flow,time-standards}` serve Little's-Law flow
   (λ/μ/ρ/κ/τ) and the access-standard catalogue. Never per-clinician:
   see [`agents/share/time-based-analysis.md`](../../../agents/share/time-based-analysis.md)
   §7 for the family-wide refusal and §6.x above for this crate's one
   permitted, not-yet-buildable exception (utilisation). Pure
   computation: [`src/tba.rs`](../src/tba.rs); HTTP surface:
   [`src/controllers/tba.rs`](../src/controllers/tba.rs). Full design:
   [`agents/share/time-based-analysis.md`](../../../agents/share/time-based-analysis.md),
   [`../../spec/time-based-analysis.md`](../../spec/time-based-analysis.md).
   Landed §13 T-13, 2026-08-23.
19. **Cross-service journey links (`continues_as`).** `POST`/`GET`/`DELETE
   /api/instances/{pid}/links` (+ `/{id}`) assert, list, and withdraw the
   outbound `continues_as` edge from a pathway **instance** (never a
   template — a journey belongs to an enrolment, and the template is a
   document many journeys share) into the next episode: another pathway
   instance, a `patient_flow_stay`, or a `case`. `GET
   /api/instances/links[?since=]` is the aggregator's bulk reconciliation
   pull. `GET /api/instances/{pid}/journey` walks the chain
   breadth-first, resolving each leg — local legs from this database,
   remote legs from the far service under **the caller's own credential**
   (never a service identity, which would make this a confused deputy) —
   and withholds the combined totals unless every leg resolved (a
   stitched total missing a leg is a wrong number, not an imprecise one).
   `continues_as` is a **high**-sensitivity kind alongside `subject_of`
   ([`agents/share/cross-service-linking.md`](../../../agents/share/cross-service-linking.md)
   §10.2): edges are authorised at the read-the-journey level against the
   instance's own pathway template (including its `sensitive_setting`
   flag), every write is audited, and the bulk pull is gated as a
   privileged (`destructive`) read. **See §6.20 for the denial rule.**
   Write side: [`src/controllers/links.rs`](../src/controllers/links.rs);
   stitching read: [`src/journey.rs`](../src/journey.rs). Landed §13
   2026-08-24 through 2026-08-27 (`0.2.0`, CHANGELOG.md).

### 6.20 Rule: a denied journey-link request is `404`, not `403`

On every `/api/instances/{pid}/links*` and `/api/instances/{pid}/journey`
endpoint (§6.19), a record-level authorization denial is reported as
**`404 Not Found`**, never `403 Forbidden`. This is a deliberate
governance decision, distinct from the rest of the API, not an
inconsistency:

- A `403` answers a question the caller was not allowed to ask in the
  first place. "This journey exists, and you may not see it" is itself
  a disclosure — and on a mental-health or palliative pathway, it *is*
  the disclosure that matters.
- An empty list and a denied read are therefore made **deliberately
  indistinguishable**. The stitched `/journey` traversal collapses a
  peer's `403` and `404` into one leg status for the same reason, so the
  leak cannot reopen from the far side of a cross-service hop.
- `401` is left alone: "you sent no credential" discloses nothing about
  what exists, and folding it into `404` would leave an unauthenticated
  client retrying forever against a URL it should be authenticating to.

The cost is real and stated rather than hidden: a misconfigured operator
sees `404` where the true answer is "your policy denies this," which is
harder to debug. The **audit trail** carries the denial, so the
information is moved somewhere the caller cannot read, not lost. This
trade is deliberately **not** made on the pathway *record* endpoints
(`/api/care-pathways/*`), which still return `403`: a care-pathway
template is a document, not a person, and knowing one exists discloses
nothing about anybody. Enforced in
[`src/controllers/links.rs::record_rejection`](../src/controllers/links.rs)
and pinned by `a_denied_request_is_reported_as_not_found` and
`a_missing_credential_stays_unauthorized`.

### 6.x Per-clinician utilisation (permitted, not yet buildable)

**Decision 2026-08-25.** Per-person utilisation — recorded effort
against declared available capacity — is **permitted** in this service,
extending the exception in
[`agents/share/time-based-analysis.md` §7.1](../../../agents/share/time-based-analysis.md).
The family refusal to compute **per-clinician cycle time, throughput or
efficiency** is **unchanged** and still binds here: this reverses one
narrow thing, and utilisation does not reach the others.

**It cannot be computed today, and that is a data gap rather than a
policy one.** This service records a **care team with roles**
(`lead_clinician`, `gp`, `specialist`, `nurse`, `mental_health`,
`coordinator`) on a pathway instance, which is an attribution point —
but it holds **no recorded effort and no declared capacity**. Both are
new: a roster (who is available, for how much of a week) and an effort
source (time against a pathway instance, or a sessions/job-plan feed).
Until both exist there is no numerator and no denominator, and the
figure is absent rather than zero.

When built, it adopts all **five obligations** in §7.1 — declared
denominator returned with the number; non-working time (leave, study
leave, non-clinical duty) excluded from the denominator rather than
counted as idle; small denominators suppressed; never the sole ranking
key, and shipped beside the same period's queue and wait figures;
effort labelled **asserted** — plus two that bind harder here than in a
project portfolio:

- **Suppression is a privacy control, not only a statistical one.** A
  ward of four nurses makes a per-person figure identifying at almost
  any aggregation, so the §8 re-identification rules of the family doc
  govern the floor.
- **A high reading is a warning, not an achievement.** Utilisation near
  100% is what a queueing system looks like just before it stops coping,
  which in a clinical setting is a safety observation.

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
back to the env key set, so it always boots. **A background loop then
keeps polling** (`auth::spawn_key_refresh`, AU-2, 2026-08-01): every
`CARE_PATHWAY_PASETO_KEYS_REFRESH_SECS` (default 3600; `0` disables; a
no-op when the URL is unset) it re-fetches and swaps the key set into
the live `ReloadableVerifier` that the guard and the bearer extractors
read per request, so a key rotation reaches a running process without a
restart. A failed refresh keeps the current key set rather than locking
every caller out on a transient auth-service outage. See the family contract
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
supersedes the earlier per-crate roles/RBAC sketch. **Hot-reloadable**
(AU-2, 2026-08-01): the policy lives in a `ReloadablePolicy` the guard
reads per request, and — when `CARE_PATHWAY_ABAC_POLICY_FILE` is set — a
background loop (`auth::spawn_policy_watcher`) polls the file's mtime
every 15 s and calls `reload_policy()` on a change, so an operator can
edit the policy without a restart; a malformed edit falls back to the
built-in default rather than leaving the service unprotected.

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
endpoints, `$validate`, the SMART 404, the `CapabilityStatement`, the
full Bulk Data kickoff → status → NDJSON → cancel flow, and — the
truncation blind spot itself — a checkpoint catching a wholesale
`audit_logs` deletion the chain alone reports as `verified: true`.

Two further DB-gated request files exercise operational surfaces not
covered above: [`tests/requests/instances.rs`](../tests/requests/instances.rs)
(enrolment, lifecycle transitions, review cadence, urgency, care team,
step completion, outcomes, and the derived caseload/overdue-review/
care-team-load/cohort views) and
[`tests/requests/insights.rs`](../tests/requests/insights.rs) (the five
registry lenses in §13's 2026-07-20 entry).

**Time-based analysis and journey links.** [`src/tba.rs`](../src/tba.rs)
carries its own DB-free unit suite — interval union/subtraction, the
four-category clock partition summing to the lead time under
overlapping and out-of-window segments, gaps, handoffs, nearest-rank
percentiles, cohort rollup, constraint ranking, Little's Law — with no
I/O and no clock read (`as_of` is always a parameter), so all of it runs
without a database; the calendar-time-not-recorded-activity property
(§6.18) is pinned by a dedicated regression test. Exercised end-to-end,
DB-gated, by [`tests/requests/tba.rs`](../tests/requests/tba.rs)
(segment/clock recording `422`s, small-cohort percentile suppression,
`401` under `CARE_PATHWAY_REQUIRE_AUTH`).
[`src/controllers/links.rs`](../src/controllers/links.rs) unit-tests the
`continues_as` accept/reject matrix (permitted vs. refused far-end
types, self-link refusal, URN canonicalisation) and — the property this
crate treats as load-bearing — that a record-level `403` is remapped to
`404` while a `401` is left alone (§6.20:
`a_denied_request_is_reported_as_not_found`,
`a_missing_credential_stays_unauthorized`).
[`tests/requests/links.rs`](../tests/requests/links.rs) covers the
write/list/delete/bulk-pull endpoints against Postgres, and
[`src/journey.rs`](../src/journey.rs) the stitched-journey combine logic
(withheld totals unless every leg resolved).
[`tests/requests/event_outbox.rs`](../tests/requests/event_outbox.rs)
and [`tests/outbox_audit.rs`](../tests/outbox_audit.rs) cover the
durable-bus outbox path and its audit-in-same-transaction property;
[`tests/enforcement.rs`](../tests/enforcement.rs) is the blanket-guard
activation proof described above by behaviour rather than by name.

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

**Keyed integrity MAC** ([`src/compliance/mac.rs`](../src/compliance/mac.rs),
embedding the shared [`integrity-mac`](../../../integrity/integrity-mac-rust-crate/index.md)
crate). A SHA-256 digest alone proves nothing against an attacker with
database write access — its format is published, so a forged row can
carry a matching hash. `CARE_PATHWAY_INTEGRITY_MAC_KEY` (or
`_KEY_FILE`, which takes precedence) configures an HMAC-SHA256 root key;
HKDF derives a distinct subkey per purpose (`audit-chain`, `record`,
`checkpoint`, below) so a tag produced for one domain can never verify
as another. No key configured ⇒ no MAC is written, and rows are reported
`mac_absent` rather than as mismatches — adopting the control must not
manufacture a wall of false accusations. `cargo loco task
integrity_key` generates/checks/reports the key without ever logging it
(§ no-secret-in-logs, `agents/share/security.md`); `cargo loco task
integrity_resign` re-signs existing rows after a key rotation. Landed
2026-07-27; the family-wide activation order and env vars are
[`agents/share/runbooks/integrity-activation.md`](../../../agents/share/runbooks/integrity-activation.md).

**External-witness checkpoints** ([`src/compliance/checkpoint.rs`](../src/compliance/checkpoint.rs)).
The hash chain cannot see the chain's own tail deleted: removing the
newest N rows leaves no successor to break, so the shortened chain
verifies perfectly, and deleting every row verifies vacuously. A
checkpoint closes that blind spot by recording, outside this database,
what the chain looked like at a known moment. `GET
/api/compliance/checkpoint` returns a MAC'd statement of the chain's
current head hash, anchor row id, and row count; `POST
/api/compliance/checkpoint/verify` takes a previously-taken checkpoint
back and reports whether the chain still honours it — distinguishing
the anchor row missing (rows deleted) from its hash changed (content
altered) from fewer rows standing at or before it (earlier history
deleted, even though the anchor itself survived). **The control is the
off-box storage, not this code**: a checkpoint kept in this database is
worthless, since whoever can delete audit rows can delete a checkpoint
row in the same transaction. Each checkpoint is also logged at `INFO`,
so a deployment that already ships logs off-host has a witness without
building anything further. Landed 2026-07-27.

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
- **Right of access — field masking + export**
  ([`src/privacy.rs`](../src/privacy.rs), §6.16–17). `mask_pathway`
  redacts `provider_name` / `provider_id`; every clinical field is left
  alone, since a pathway *template* names no patient and redacting
  `condition_codes` would defeat the registry for no privacy gain.
  `export_pathway` builds the `{entity, pid, exported_at, masked,
  record, note}` envelope, wired to the ABAC `mask` obligation via
  `auth::authorize_record` — a caller granted a masked read gets a
  masked export too, and the envelope says so. Every export is audited
  as a disclosure regardless of whether it is masked. **Not** covered:
  the patient-identifying `pathway_instances.subject_ref` linkage (§16).

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
- **Bulk export is durable** (since 2026-07-26). Job state is a
  `bulk_jobs` row, the work runs on the `bg_pg` worker queue, and the
  output NDJSON goes to an artifact store — so a poll from any replica,
  at any point in the retention window, answers correctly. Remaining
  bounds, which are caps rather than defects: 10 000 resources / 8 MiB
  per export, a 15-minute retention TTL, and a **local-filesystem**
  artifact backend (`CARE_PATHWAY_BULK_ARTIFACT_DIR`) — the S3-compatible
  backend with short-lived access-controlled URLs is a later step, as it
  is for person. Cancelling drops the output reference at once rather
  than at TTL, so a cancelled export stops serving clinical data
  immediately.
- **Signing keys are out of scope** for the build script — a deployment
  secret, signed in the release pipeline.
- **No ISO 14971 risk file, DPIA, Art. 30 record, EHDS data permit, or
  Inferno run.** Organisational or infrastructure artefacts; the service
  supplies the technical controls they cite.

## 13. Tasks (live work queue)

- [ ] **UTIL-1 — Per-clinician utilisation (§6.x).** Permitted by the
  2026-08-25 decision; **blocked on two absent inputs**, so this task is
  the inputs before it is the figure.
  - [ ] A **roster / capacity source**: who is available to this
    service, for how much of a period, with a working-time
    configuration and recorded non-working time (leave, study leave,
    non-clinical duty) that **subtracts from the denominator**.
  - [ ] An **effort source**: time recorded against a pathway instance,
    or a sessions / job-plan feed. Labelled **asserted**, never inferred
    from status changes.
  - [ ] The figure itself, with the five §7.1 obligations.
  - **Acceptance:** a clinician on leave for the whole window reports
    `null` with a reason, **not 0%**; a window below the suppression
    floor is `null` — and the floor is justified as a
    re-identification control, not only a statistical one; the response
    carries effective time, available capacity, and the configuration
    that produced them; **no endpoint returns per-clinician cycle time,
    throughput or flow efficiency**, and none can be derived by
    arithmetic from what is returned.

- [x] **2026-09-01 — PRO-H12 slice 5: OpenTelemetry OTLP export.** This
  crate carried no `src/observability` module at all before this
  change. Added it as a port of organization-service's slice 4 (repo
  `tasks.md` PRO-H12) — `src/observability.rs` (real OTLP/gRPC span +
  metric export, on by default at `OTLP_ENDPOINT`, disabled by setting
  it to the empty string), `App::init_logger`/`on_shutdown` wired in
  `src/app.rs`, and `observability::trace_mw` layered as the outermost
  middleware in `after_routes` — this crate's **only**
  router-construction surface, confirmed (not assumed) by grepping
  `src/`/`tests/` for a second `Router::new()`/`create_router`: the
  one hit is a unit test for the auth middleware, not an app-level
  router. No `otlp-test-tonic` rename was needed (this crate declares
  no `tonic` dependency of its own). See
  [`agents/share/rust-tracing-opentelemetry-stack.md`](../../../agents/share/rust-tracing-opentelemetry-stack.md)
  for the full rollout status. Because this crate is the family's IEC
  62304 SOUP-register reference implementation, the port also required
  8 new `compliance/soup.tsv` rows (found by running
  `every_direct_dependency_is_annotated`, not anticipated up front).
  Verified: `cargo fmt --check`, `cargo clippy --all-targets -D
  warnings`, `cargo deny check`, `cargo bench --no-run`, and the MSRV
  check (`cargo +1.96 check --all-targets`) all clean; `cargo test
  --lib` 316/316 (was 308, +8 new `src/observability.rs` unit tests);
  `cargo test --test otlp_export --test otlp_middleware` 4/4 (real
  protobuf crossing a real in-process gRPC socket).

- [x] **2026-08-28 — PRO-P13: backport TBA + journey-links into this
  spec, and close the instances/insights OpenAPI gap.** This spec
  (§1–§18) had zero coverage of time-based analysis and the
  `continues_as` journey/links surface even though both had shipped —
  the umbrella `spec/time-based-analysis.md`, the CHANGELOG, and
  AGENTS.md were the only places that said so. Backported into §2, §6
  (new §6.18–§6.20), §11, §13 (this entry plus the two below), §14, and
  §15. Also closed `src/openapi.rs`'s own pre-existing,
  spec-acknowledged gap (its module doc used to read "the sibling
  instance and insight endpoints are not documented here yet"): the
  operational instance layer (`src/controllers/instances.rs`) and the
  five registry-insight lenses (`src/controllers/insights.rs`) are now
  in `openapi.json`, pinned by
  `spec_documents_the_instance_and_insight_surface`. Swept the stale
  "front-end merge action deferred" claim (the two-step merge UI has
  been live in `care-pathway-front-end-with-svelte`'s `[pid]/+page.svelte`
  since before this spec last touched it) from this file, `AGENTS.md`,
  and `README.md`, and corrected `AGENTS.md`'s stale
  `authentication-verifier` version citation (`0.2` → `0.9.0`, this
  crate's actual `Cargo.toml` dependency).

- [x] **2026-08-23 — T-13: Time-based analysis (TBA-1 … TBA-7, §6.18).**
  The time dimension of the pathway — of the calendar time a patient
  spent on it, how much was care? Unifies Barker's time-based analysis
  (the value-adding ratio; published NHS journeys measure 8–14%), value
  stream mapping (VA/NNVA/UNVA classification, VT/PT/LT/%A/#HO metric
  names), and queueing theory (λ/μ/ρ/κ/τ, Little's Law). New primitive
  `instance_segments` (migration
  `m20260823_000014_time_based_analysis`) plus an explicit
  `clock_start_at`/`clock_stop_at` on `pathway_instances`, backfilled
  from `enrolled_on`/`closed_on` so pre-migration instances are
  analysable at day resolution immediately. Pure analysis in
  `src/tba.rs` (interval union/subtraction, the four-bucket clock
  partition, gaps, handoffs, nearest-rank percentiles, the NHS
  access-standard catalogue, cohort rollup, constraint ranking, Little's
  Law — no I/O, `as_of` always a parameter); HTTP surface in
  `src/controllers/tba.rs`; documented in `openapi.json`. Full contract:
  [`agents/share/time-based-analysis.md`](../../../agents/share/time-based-analysis.md),
  [`../../spec/time-based-analysis.md`](../../spec/time-based-analysis.md).
  **Acceptance:** a 100-day journey with 14 value-adding days reports
  0.14 — and the same journey with only its value-adding segments
  *recorded* still reports 0.14, the regression pin against
  "simplifying" the denominator into a sum of recorded activity; the
  four category buckets sum to the lead time over a generated sweep;
  ratios stay in `[0, 1]` under overlapping/out-of-window segments;
  degenerate clocks return a stated `null` rather than panic; every
  §5.1 invariant is a `422`; `401` under `CARE_PATHWAY_REQUIRE_AUTH`.
  48/48 `--ignored` request tests green vs Postgres 18; 279 unit tests;
  clippy pedantic clean. TBA-9 (cross-service stitching) followed as the
  `continues_as` journey work below; TBA-11 (flow gauges) followed with
  it.

- [x] **2026-08-24 through 2026-08-27 — Cross-service journey links
  (`continues_as`, §6.19) + stitched journeys + TBA flow gauges
  (TBA-11).** This service's first originated cross-service edge:
  `entity_links` write-side (migration
  `m20260824_000015_entity_links`), `POST`/`GET`/`DELETE
  /api/instances/{pid}/links` (+ `/{id}`), and the aggregator's bulk
  reconciliation pull `GET /api/instances/links[?since=]`
  (`src/controllers/links.rs`); the read side,
  `GET /api/instances/{pid}/journey`, follows the chain breadth-first
  under the *caller's* credential and withholds combined totals unless
  every leg resolved (`src/journey.rs`). The shared `entity-ref`
  registry gained `continues_as` and two entity types,
  `care_pathway_instance` and `patient_flow_stay` (the second the first
  type owned by a consumer application rather than an index registry);
  the `Envelope` gained `Linked`/`Unlinked` kinds plus an additive
  `data` field (`skip_serializing_if`, so existing CRUD envelopes stay
  byte-identical). **A denied link/journey request is reported as
  `404`, not `403`** — the named rule at §6.20. A default-off Prometheus
  gauge family (`care_pathway_flow_*`, TBA-11) reports cohort
  value-adding ratio / p90 lead time / coverage / instance count per
  pathway on a background refresh loop
  (`CARE_PATHWAY_FLOW_METRICS_SECS`), with the same small-cohort
  suppression the API itself applies and a per-pathway series cap
  (default 50) so one metric family cannot take a Prometheus install
  down. **Acceptance:** the `continues_as` accept/reject matrix for
  permitted far-end types is unit-pinned, as are the self-link refusal
  and the idempotent-reassert-on-`(from_pid, kind, to_ref, valid_from)`
  case; `a_denied_request_is_reported_as_not_found` and
  `a_missing_credential_stays_unauthorized` pin the §6.20 rule;
  `tests/requests/links.rs` green vs Postgres 18. Released as `0.2.0`
  (CHANGELOG.md, tag `care-pathway-service-v0.2.0`).

- [x] **2026-08-21 — FUZZ-2: cargo-fuzz harness for the request-path
  logic.** Three coverage-guided targets over the pure, total code that
  faces the network: `validate_json` (bytes → `serde_json` → `CarePathway` →
  `validation::problems`), `validate_built` (the validator driven from
  raw bytes, so the fuzzer controls array cardinality directly rather
  than having to learn JSON first), and `merge_pathways` (the merge fold over
  two arbitrary payloads). Invariants pinned: never-panic; validation is
  deterministic and its **problem report is bounded** independent of
  payload size (the generalisation of SEC-M8, which the unit tests pin
  only for one hand-written payload); merge keeps the survivor's
  `name` and is **absorbing**, so a retried merge cannot inflate
  the record. The sub-crate declares an empty `[workspace]` table
  because this crate is a workspace root, and no `rust-version` because
  cargo-fuzz is nightly-only. Verified: all three build and run clean,
  and the bounded-report assertion was confirmed live by lowering its
  ceiling until it fired.

- [x] **2026-08-21 — SEC-M8b: the `422` report is bounded, not just the
  work.** `validation::problems` reported an over-long array's
  cardinality violation once and then still walked every entry, so a
  payload with ten thousand blank/malformed entries returned ten thousand
  problem strings in one `422` body — a small request buying a large
  response. Worse here than a blank check: each entry also ran an ICD-10 /
  ICD-11 / SNOMED CT validation, SNOMED including a Verhoeff check
  digit. Every per-entry loop now walks the new `inspected()`
  helper (at most `MAX_ARRAY_LEN` entries, index preserved): the
  cardinality problem already rejects the payload, so inspecting the tail
  decides nothing, and bounding the **report** is the same input-bounding
  rule as bounding the work (SEC-M1). Named rather than inlined so a loop
  added later without it reads as different. Pinned by a test, which was
  confirmed to fail without the cap. Measured with `benches/service_bench.rs`: the oversized-array
  rejection path went from **112 µs to 4.9 µs**, ~96% less. Case was the reference
  (repo `tasks.md` SEC-M8/SEC-M8b).

- [x] **2026-08-20 — Search writer held for the process; Criterion
  benchmarks; declared MSRV.** `SearchEngine::index_pathway` (and the
  delete/clear paths) built a **new Tantivy `IndexWriter` per call**.
  That allocates the whole 50 MB `WRITER_HEAP_MB` arena and spawns merge
  threads on construction, so every create / update / merge /
  soft-delete paid it synchronously on the request path — ~155 ms per
  indexed document, measured against a fresh index. It was also a
  concurrency hazard: an `IndexWriter` holds the index directory's
  exclusive lock, so taking and releasing it per call left two
  simultaneous writes able to collide on it. The engine now holds one
  `Mutex<IndexWriter>` created in `new()` (~78 ms per document; the
  remainder is the durable `commit()` plus reader `reload()` that
  read-after-write indexing inherently costs — see repo `tasks.md`
  PERF-2 for the open design question). Found by `benches/service_bench.rs`,
  new in the same pass: validation, merge, and search groups over the
  CPU-bound halves of a request, compiled in CI by the new
  `scripts/ci-check.sh bench` stage. `Cargo.toml` also declares
  `rust-version = "1.95"` — the repository's current-stable-minus-three
  floor (`spec/rust-msrv-n-minus-2/index.md`), enforced by
  `scripts/ci-check.sh msrv`.

- [x] **2026-08-02 — Privacy: field masking + GDPR export (repo
  tasks.md P-2, as P-1/organization).** `src/privacy.rs`:
  `mask_pathway` + `export_pathway`. Unlike organization, a
  `CarePathway` is a **template** that names no patient — its clinical
  content (`name`, `condition_codes`, `interventions`, `keywords`,
  `identifiers`) is left untouched (masking it would defeat the
  registry for no privacy gain, exactly as masking an LEI would for
  organization); only `provider_name` / `provider_id` (institutional,
  not personal, but worth redacting from a cross-department reader) are
  masked. `src/auth.rs` gains `authorize_record` +
  `care_pathway_resource_attrs` (`care_setting`, and a
  `sensitive_setting` flag for `mental_health` / `palliative` — the two
  settings that carry special-category treatment under UK Common Law
  Duty of Confidentiality even though the template names no one). `GET
  /{pid}` honours the `mask` obligation; new `GET /{pid}/masked` and
  `GET /{pid}/export` endpoints (§6.16–17), the export audited as a
  disclosure via the existing `disclosure::action::EXPORT`. **Explicitly
  out of scope** (§16): `pathway_instances.subject_ref`, the actual
  patient-identifying enrolment linkage, which lives outside this
  module entirely.
  *Verified:* 48 DB-gated (46 request-suite + 2 dedicated `tests/masking.rs`,
  including a record-level-decision proof that a non-sensitive-setting
  read is *not* masked by the same policy) + 1 enforcement + 1
  outbox-audit green vs Postgres 18; 246 lib tests; fmt + clippy clean.
- [x] **2026-08-01 — Pagination on list and search.** `GET
  /api/care-pathways` and `GET /api/care-pathways/search` take
  `?limit=`/`?offset=` and report `X-Total-Count`/`X-Limit`/`X-Offset`
  (§6.2), per the family convention in `agents/share/restful.md`
  (first implemented in organization). Defaults reproduce the old hard
  caps (100 list / 50 search); `limit` clamps to 500 rather than
  erroring; `offset` past 10 000 is `400` (SEC-G7). The search total is
  a `COUNT(*)` over the predicate, not the page length. *Verified:* a
  DB-gated request test walks a window and checks the total, the clamp,
  and the `400`.
- [x] **2026-08-01 — AU-2: key rotation and ABAC policy hot-reload
  without a restart.** The loco-style half of the family rollout (case
  was the reference). `src/auth.rs`: the verifier and the ABAC policy
  became **reloadable holders** (`ReloadableVerifier` /
  `ReloadablePolicy`) the blanket guard and the bearer extractors read
  per request — previously boot-only `OnceLock` snapshots that a
  rotated key or an edited policy could never reach. `spawn_key_refresh`
  re-fetches `CARE_PATHWAY_PASETO_KEYS_URL` every
  `CARE_PATHWAY_PASETO_KEYS_REFRESH_SECS` (new env var, default 3600;
  `0` disables; no-op when the URL is unset) and swaps the fetched set
  in; a failed fetch keeps the current keys. `spawn_policy_watcher`
  polls `CARE_PATHWAY_ABAC_POLICY_FILE`'s mtime every 15 s and calls
  `reload_policy()`; a malformed edit falls back to the built-in
  default rather than leaving the service unprotected. Resolves the §16
  "periodic re-fetch" open question. *Verified:* the existing
  `tests/enforcement.rs` activation proof needed no change — it now
  exercises the reloadable holders by construction.
- [x] Name search — `GET /search?q=` Tantivy full-text/fuzzy/phonetic
  search (§13 2026-08-01), replacing the earlier Postgres `ILIKE`.
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
  Phase-3 item below, now joined by the real-broker sink; designed in
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
  DB-gated-tested via the outbox suite.
- [x] **Durable event bus — real broker sink (`FluvioSink`, BUS-3,
  2026-08-03).** Ported from case-service's BUS-1 reference
  (`case/case-service-with-loco/src/relay.rs`). `FluvioSink` (`impl
  EventSink`) lives in `src/relay.rs` behind this crate's own `fluvio`
  Cargo feature, off by default, so a default build's dependency tree
  and behaviour are unchanged. `spawn()` selects it over `LoggingSink`
  when `CARE_PATHWAY_FLUVIO_ENDPOINT` is set (default topic
  `mxi.care_pathway.events`, overridable via `CARE_PATHWAY_EVENT_TOPIC`);
  an endpoint configured **without** the `fluvio` feature compiled in is
  a clean refusal to start the relay (logged at `error`), never a
  silent `LoggingSink` fallback that would mark outbox rows published
  without ever reaching a real broker. The initial broker connection
  retries indefinitely rather than falling back. `compose.fluvio.yaml`
  + `Dockerfile.fluvio-cli` provision a local broker for opt-in manual
  runs (not part of any automated CI stage); `tests/fluvio_relay.rs` is
  a `fluvio`-feature-gated, `#[ignore]`d live round-trip test, verified
  today only by compiling under `--features fluvio` (no broker is stood
  up by any automated run in this repo). `compliance/soup.tsv` carries
  the `fluvio` SOUP row. See
  [`agents/share/event-bus.md`](../../../agents/share/event-bus.md) §5,
  §8.
- [ ] Privacy controls if any restricted fields appear.
- [x] Record merge — `POST /merge` folds a duplicate into a survivor
  (union fields, former-title alias, soft-delete, `merge_records`
  history + snapshot, `Merged` event); pure `src/merge.rs`;
  `/merges/recent`. Front-end merge action shipped in
  care-pathway-front-end-with-svelte's `[pid]/+page.svelte` (inline
  two-step "merge into this record" → "confirm merge" flow after a
  duplicate check).
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
  - [x] **Move Bulk Data `$export` onto the `bg_pg` worker + an artifact
    store.** **Done (2026-07-26):** migration
    `m20260726_000009_bulk_jobs` creates the family `bulk_jobs` table
    (shared doc §3, matching person's so the two do not drift);
    `src/models/bulk_jobs.rs` owns the queued→running→terminal lifecycle;
    `src/workers/bulk_export.rs` materialises the NDJSON off the request
    path; `src/bulk/store.rs` is the `ArtifactStore` trait plus its
    local-filesystem dev backend, ported from person including its
    SEC-B4 path confinement. The four `$export` endpoints read the row
    rather than a process-local registry, so a queued job now really does
    return `202` + `X-Progress`. Pinned by four DB-gated tests, including
    `export_state_is_durable_not_in_process`, which reads the job and its
    artifact straight from the database and store — the way a second
    replica would.
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

- [x] **2026-07-27 — Keyed integrity MAC + external-witness checkpoints
  (§12.1).** A SHA-256 digest alone is forgeable by anyone with database
  write access, since its format is published; this closes that. New
  `src/compliance/mac.rs` embeds the shared
  [`integrity-mac`](../../../integrity/integrity-mac-rust-crate/index.md)
  crate: `CARE_PATHWAY_INTEGRITY_MAC_KEY` (or `_KEY_FILE`, which wins) +
  HKDF-derived per-domain subkeys (`audit-chain` / `record` /
  `checkpoint`) so a tag from one domain can never verify as another; no
  key ⇒ no MAC written, rows report `mac_absent` rather than a
  mismatch. `cargo loco task integrity_key` (`op:generate` /
  `op:check` / `op:status`, key never logged) and `integrity_resign`
  (re-sign existing rows after rotation). New `src/compliance/checkpoint.rs`
  + `GET`/`POST /api/compliance/checkpoint{,/verify}` (§6.13): a MAC'd
  statement of the chain's head/anchor/row-count, closing the one gap
  the hash chain cannot see on its own — deletion of the chain's own
  **tail**, which leaves no successor to break and so verifies
  perfectly (or, if every row is gone, vacuously). The control is the
  off-box storage of the checkpoint, not the code — a checkpoint kept in
  this database is exactly as deletable as the rows it witnesses.
  Family-wide activation order:
  [`agents/share/runbooks/integrity-activation.md`](../../../agents/share/runbooks/integrity-activation.md).
  *Verified:* a DB-gated test empties `audit_logs`, asserts
  `/audit/verify` still reports `verified: true` (stating the blind spot
  outright), then shows a checkpoint catches it; unit tests for every
  domain having a distinct label and `Domain::ALL` covering every
  variant.

- [ ] **CP-T1 (M) Batch `/deduplicate` + persisted `review_queue`.**
  Unlike person/worker/place/thing/organization
  (`agents/share/match-search-merge.md`: "Review queue persisted... —
  person / worker / place / thing / organization"), this crate has only
  real-time `check-duplicates`; there is no batch scan endpoint and no
  stored review queue at all. `auth::DESTRUCTIVE_POST_SUFFIXES` already
  reserves `/deduplicate` as a destructive action "ahead of the
  dedup-scan... features", confirming this was planned but never
  built. *(Verified: `grep -n '"/check-duplicates"\|"/deduplicate"\|"/review-queue"'
  src/controllers/care_pathways.rs` shows only `/check-duplicates`
  routed; no `review_queue` table/migration/model exists.)* Three-part
  change: a `review_queue` migration (mirroring case-service's, which
  also started from zero — `provenance` column from day one), `POST
  /deduplicate` (pairwise scan persisting candidates), `GET
  /review-queue` (`?status=&limit=`), `POST /review-queue/{id}/decision`.
  **Acceptance:** DB-gated round-trip (scan → list → decide → `422` on
  re-decide → `404` unknown) green; the front-end's `/review` route (a
  Kanban already exists for the *service's* stored review model in the
  sibling crates) can be pointed at it in a follow-up.

- [ ] **CP-T2 (M) Extend the ABAC `mask` obligation to `list`, `search`,
  and `check-duplicates`.** Only the single-record `GET /{pid}` and
  `GET /{pid}/export` handlers call `crate::auth::authorize_record`; the
  `list`, `search`, and `check_duplicates` handlers return unmasked
  provider name/id even under a `mask`-obligated policy. This is the
  same gap as the family's SEC-G3 (partial for person) and violates
  `agents/share/security.md` invariant 5 ("masking on every read
  path"). *(Verified: `grep -n "async fn list\|async fn search\|async fn check_duplicates\|authorize_record"
  src/controllers/care_pathways.rs` shows `authorize_record` only at the
  two lines that also match `get`/`get_export`.)* Three-part change.
  **Acceptance:** a DB-gated test with a `mask`-obligation policy proves
  `list`/`search`/`check-duplicates` responses redact `provider_name`/
  `provider_id` exactly as `GET /{pid}/masked` does.

- [x] **CP-T3 (S) `same_as` URL well-formedness validation.** *(resolved
  2026-09-04.)*
  `src/validation.rs`'s `string_array_caps("same_as", …)` only bounds
  length/cardinality; it never checks the entries parse as URLs. Since
  `same_as` also drives the matcher's R-2 deterministic short-circuit
  (case-folded string overlap, empty-guarded), a garbage non-URL value
  is silently accepted and stored, and two pathways sharing the same
  garbage string would still deterministically short-circuit to a
  match. *(Verified: `grep -n "same_as" src/validation.rs` shows only
  the cardinality/length call, no format check.)* Three-part change.
  **Acceptance:** an `422` with a field-scoped reason for a `same_as`
  entry that doesn't parse as a URL; existing valid records unaffected;
  unit + request-level tests.
  - **Resolved.** Added `is_valid_url` (the same lightweight
    `http://`/`https://` scheme check every sibling entity crate
    applies) and wired it into `problems()` as a per-entry `same_as[i]`
    check, mirroring the existing `in_language`/identifier per-entry
    loops. New unit tests
    (`same_as_entries_must_be_http_urls`,
    `malformed_same_as_url_is_a_problem`) plus a DB-gated request-level
    test (`malformed_same_as_url_on_create_returns_422`,
    `tests/requests/care_pathways.rs`) verified green against a real
    Postgres (`scripts/ci-check.sh test-db …`).

- [x] **CP-T4 (S) Wire FHIR `GET /fhir/PlanDefinition` search onto the
  Tantivy index.** `controllers/fhir.rs::search` calls
  `PathwayModel::list(&ctx.db, FHIR_SEARCH_SCAN_CAP)` — a capped
  Postgres scan — rather than `crate::search::SearchEngine`, even
  though this crate indexes via Tantivy for the native `/search`
  endpoint. *(Verified: `grep -n "async fn search" -A 15
  src/controllers/fhir.rs` shows the handler reading
  `PathwayModel::list` directly.)* Three-part change, mirroring the
  identical open item already recommended for organization-service
  (ORG-T5, landed first — this is its twin, same pattern ported).
  **Resolution (2026-09-05):** `search()` now derives a `query_text`
  from whichever of `FhirPlanSearchParams::name` /
  `FhirPlanSearchParams::identifier` is present (the crate's only
  text-bearing FHIR search params — both are indexed Tantivy `TEXT`
  fields, `src/search/index.rs`); when present, candidates resolve via
  `crate::search::engine().search(q, …)` +
  `PathwayModel::find_by_pids`, mirroring the native `/api/care-pathways/search`
  handler's retrieval exactly. A query with neither param (e.g. bare
  `_id`) falls back to the original `PathwayModel::list` capped scan,
  since there is nothing to search on. `FhirPlanSearchParams::matches`
  is unchanged and stays the authoritative, field-precise filter in
  both branches — only retrieval changed. `parse_pids` in
  `controllers/care_pathways.rs` bumped `pub(crate)` for reuse (no
  duplicated hit-parsing logic). The task's own acceptance text names a
  `?title=` param that does not exist on this resource (the field is
  `name`, per `FhirPlanSearchParams`) — corrected in the implementation
  rather than followed literally. As in ORG-T5, literally exceeding the
  1000-row scan cap in a test is impractical (it would mean creating
  1000+ live, Tantivy-indexed rows); the substituted test proves
  retrieval no longer depends on `PathwayModel::list`'s recency
  ordering — the target pathway is created first (oldest/lowest id,
  the position a newest-first scan reaches last) and is still found
  ahead of five newer distractors
  (`tests/requests/fhir.rs::fhir_search_by_name_resolves_through_the_index`);
  a second test pins the bare-`_id` fallback
  (`fhir_search_by_id_alone_still_works`). The `CapabilityStatement`'s
  declared search params are unchanged (only retrieval changed, not
  the supported-param list). Verified: `cargo build --lib`, `cargo
  clippy --all-targets -- -D warnings`, `cargo test --lib` (318
  passed), `cargo test -- --ignored` against a real Postgres (54
  passed, including both new tests), `cargo fmt --check`.

## 14. Implementation status

Done: loco boot; care_pathways table + migration; CRUD with `422`
validation on create/update (blank `name`; ICD-10 / ICD-11 / SNOMED CT
`condition_codes` format checks; UUID / DOI `identifiers` shapes; BCP-47
`in_language` syntax — all problems reported together); paginated
Tantivy full-text/fuzzy/phonetic search + paginated list
(`?limit=`/`?offset=`, `X-Total-Count`/`X-Limit`/`X-Offset`); field
masking + audited GDPR export wired to the ABAC `mask` obligation;
`/match`, `/check-duplicates`, and `/merge`
(record merge + history)
embedding care-pathway-matcher; audit log + in-memory event streaming on
every CRUD/merge (`/audit/recent`, `/{pid}/audit`, `/events/recent`,
`/merges/recent`) — all three phases of the durable event bus (canonical
`Envelope` + `EventPublisher`/`EventSink` seams, transactional outbox,
relay + retention, and the `FluvioSink` real-broker sink behind the
`fluvio` feature); offline **PASETO v4 public**
verification (`AuthUser`/`MaybeAuthUser`, `/whoami`, audit `actor` from
the token — credential switched from RS256-JWT per §13), including the
boot-time paseto-keys-over-HTTP fetch **and** a background refresh loop
(`CARE_PATHWAY_PASETO_KEYS_URL`/`_REFRESH_SECS`; §9/§13) and ABAC
policy hot-reload (`CARE_PATHWAY_ABAC_POLICY_FILE` watcher; §9/§13);
OpenAPI 3 doc
+ Swagger UI (`/api-docs/openapi.json`, `/swagger-ui`); a root-level
Prometheus `/metrics.prom` endpoint (CRUD/merge counters +
`http_requests_total`, public under enforcement); blanket `/api/*`
enforcement middleware (`auth::enforce` + `after_routes` layer,
off by default via `CARE_PATHWAY_REQUIRE_AUTH`); the operational
instance layer (enrolment, lifecycle, review cadence, urgency, care
team, outcomes — §13 2026-07-20) and the registry insight views
(directory/coverage/variants/providers/languages); the full compliance
surface (§12) — tamper-evident audit hash chain, keyed MAC + external-
witness checkpoints, row-level record integrity, read/disclosure
auditing, GDPR Art. 17 erasure, SOUP register + CycloneDX SBOM,
machine-checked requirement→test traceability; the FHIR R5
`PlanDefinition` surface (§12.3) with `$validate`, conditional SMART
discovery, and Bulk Data `$export` (durable, on the `bg_pg` worker);
**time-based analysis** (§6.18) — segment/clock recording, per-instance
and cohort TBA, ranked constraints, Little's-Law flow, a default-off
Prometheus flow-gauge family; **cross-service journey links** (§6.19) —
the `continues_as` edge write side, the bulk reconciliation pull, and
the stitched `GET /api/instances/{pid}/journey` read (each leg fetched
under the caller's own credential; combined totals withheld unless
every leg resolved) — with the §6.20 rule that a denial is reported as
`404`, never `403`; the operational instance layer and the five
registry-insight lenses are now also in `openapi.json` (closing that
crate's own pre-existing gap); DB-free tests + gated request-level
tests; green build + clippy.

## 15. Roadmap

All of the scope below shipped as `0.1.0`, tagged
`care-pathway-service-v0.1.0` on 2026-08-04 (CHANGELOG.md): the CRUD +
matching MVP, then `ILIKE` search (since replaced by
Tantivy, 2026-08-01) + audit + in-memory
streaming, then record merge + OpenAPI/Swagger + Prometheus + offline
bearer-token verification + blanket `/api/*` enforcement middleware. The
original v0.2 / v0.3 milestone split was never cut as a separate
release — it landed inside the same `0.1.0` line. The credential switch
RS256-JWT → PASETO v4 public per
[`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
landed (§13), as did the boot-time paseto-keys-over-HTTP fetch plus a
background refresh loop (`CARE_PATHWAY_PASETO_KEYS_URL`/`_REFRESH_SECS`;
§9/§13), ABAC policy hot-reload (§9/§13), field masking + audited GDPR
export wired to the ABAC `mask` obligation
(2026-08-02; §13), keyed integrity MACs + external-witness checkpoints
(2026-07-27; §12.1/§13), pagination on list/search (2026-08-01; §13),
and the durable event bus's real `FluvioSink` broker
sink (BUS-3, 2026-08-03; §13) — all three bus phases are now done. Time-based
analysis (§6.18, §13 T-13 2026-08-23) and the cross-service `continues_as`
journey edge + stitched journeys + flow gauges (§6.19, §13
2026-08-24 through 2026-08-27, `0.2.0`) have also since landed. Next
(deferred, §13): instance-layer privacy for
`pathway_instances.subject_ref` (§16), and the
native (non-FHIR) bulk import/export API.

## 16. Open questions

- ~~Normalise condition codes / interventions into their own tables once
  search lands?~~ RESOLVED (2026-08-01): search landed on the JSONB
  payload via Tantivy (§13); no normalisation needed.
- Real-time duplicate check on create (409) vs the explicit endpoint?
- ~~Periodic re-fetch of the PASETO key set (key rotation)~~ RESOLVED
  (2026-08-01, AU-2): `auth::spawn_key_refresh` polls
  `CARE_PATHWAY_PASETO_KEYS_URL` every
  `CARE_PATHWAY_PASETO_KEYS_REFRESH_SECS` and swaps the fetched set into
  the live `ReloadableVerifier` (§9/§13); a refetch-on-`UnknownKid` fast
  path was not additionally needed.
- **Instance-layer privacy for `pathway_instances.subject_ref`.** The
  privacy module landed 2026-08-02 (§13) masks the `CarePathway`
  *template* — which names no patient — but the actual
  patient-identifying fact is a specific person's *enrolment*, recorded
  as `subject_ref` (a `person:<uuid>` `EntityRef`) on the instance layer
  (`src/instances.rs`, `models/_entities/pathway_instances.rs`). That
  linkage is the clinical analogue of the `case ↔ person` `subject_of`
  edge (`agents/share/cross-service-linking.md` §10) — high-governance,
  and not yet covered by any masking or record-level ABAC. Deferred
  rather than silently left undone; needs its own resource-attribute
  design (likely keyed on the parent pathway's `care_setting` /
  `sensitive_setting`) before the instance controllers can honour a
  `mask` obligation the way `care_pathways.rs` now does.

## 17. References

- The care-pathway-matcher spec; loco.rs; ICD-10 / SNOMED CT.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
