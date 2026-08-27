# AGENTS.md — Care Pathway Service

Entry point for AI coding agents working in the `care-pathway-service`
crate: a registry of **clinical care-pathway** records.

> Read [`spec/index.md`](./spec/index.md) first — the living spec.

## What this is

A **loco.rs** service for care-pathway records: CRUD + matching,
embedding the canonical [`care-pathway-matcher`](../care-pathway-matcher-rust-crate).
The API DTO **is** `care_pathway_matcher::CarePathway` — stored verbatim
(JSONB) and matched with the same type, so there is no separate model or
adapter to drift.

| Question | Answer |
|---|---|
| Framework | loco.rs 1.0.1 (`Hooks`/`AppContext`/CLI, loco config, `sea-orm-migration` 2.0). |
| Build / test | `cargo build` · `cargo test` (DB-free: `tests/matching.rs` + controller 422 pin) · `cargo test -- --ignored` (request tests, need Postgres). |
| Run | `cargo loco start` (needs Postgres). |
| Persistence | One `care_pathways` table: `pid`, `name`, `data` (JSONB CarePathway), `active`, soft-delete. |

## API surface

API URLs are version-free; select the version with the `Accepts-version` header (default `1.0`) — see [`agents/share/api-versioning.md`](../../agents/share/api-versioning.md).

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/care-pathways` | Create (body: `CarePathway`; blank `name` → `422`) → `{pid, name}` |
| GET | `/api/care-pathways` | List active (capped 100) |
| GET | `/api/care-pathways/search?q=` | Tantivy full-text search (`?fuzzy=true`, `?phonetic=true`) |
| GET | `/api/care-pathways/{pid}` | Fetch the stored `CarePathway` (record-level ABAC; a `mask`-obligation allow returns the redacted view) |
| GET | `/api/care-pathways/{pid}/masked` | The masked view: provider name / provider id redacted |
| GET | `/api/care-pathways/{pid}/export` | GDPR right-of-access export (audited as a disclosure; masked when the policy says so) |
| PUT | `/api/care-pathways/{pid}` | Replace payload |
| DELETE | `/api/care-pathways/{pid}` | Soft-delete |
| POST | `/api/care-pathways/match` | Rank a `{query, candidates}` set |
| POST | `/api/care-pathways/check-duplicates` | Match a query against stored pathways |
| POST | `/api/care-pathways/merge` | Merge a duplicate into a survivor (`422` equal pids, `404` unknown) |
| GET | `/api/care-pathways/merges/recent` | Merge-history records |
| GET | `/api/care-pathways/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api/care-pathways/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/care-pathways/events/recent` | In-memory event stream |
| GET | `/api/care-pathways/insights/{directory,coverage,variants,providers,languages}` | Registry lenses: setting/specialty facets, condition-coverage gaps, cross-provider variants, provider directory, language coverage |
| POST/GET | `/api/care-pathways/{pid}/instances` · `/{pid}/cohort` | Enrol a `person:` URN on a pathway; the chronic cohort view |
| — | `/api/instances/{pid}` (+ `/status` `/review` `/urgency` `/team` `/events` `/steps/{s}/complete`) | Instance lifecycle, review cadence, urgency, care team, steps |
| GET | `/api/instances/{caseload,overdue-reviews,care-team-load}` | Derived operational views |
| POST/GET | `/api/instances/{pid}/segments` (+ `/segments/{seg}/close`, `/clock`) | **Time-based analysis**: record a journey segment (VA / NNVA / UNVA + stage + waste), close a running one, set the pathway clock (no pause, by design) |
| GET | `/api/instances/{pid}/{time-analysis,timeline}` | Per-journey TBA: value-adding ratio, coverage, gaps, handoffs; and the segment/gap wall |
| GET | `/api/care-pathways/{pid}/{time-analysis,constraints}` | Cohort TBA: nearest-rank lead-time percentiles vs an NHS access standard; ranked constraints |
| GET | `/api/instances/{flow,time-standards}` | Little's Law flow (λ/μ/ρ/κ/τ) and the access-standard catalogue |
| GET | `/api/instances/{pid}/journey` | **Stitched journey**: follows `continues_as` across services, each leg fetched under the *caller's* credential; combined figures withheld unless every leg resolved |
| POST/GET/DELETE | `/api/instances/{pid}/links` (+ `/{id}`) · `GET /api/instances/links` | **Cross-service journey links**: the `continues_as` edge from a pathway instance into the next episode (another instance, a `patient_flow_stay`, or a `case`); high-sensitivity governance, audited; a denial is reported as `404` so it cannot disclose the journey's existence; the bulk pull is the aggregator's reconciliation source and is a privileged read |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (text-exposition; root path, public under auth enforcement) |

Plus loco's default `/_health`, `/_ping`. Every CRUD action writes an
`audit_logs` row and publishes a `created`/`updated`/`deleted` event.
`GET /api/care-pathways` and `GET /api/care-pathways/search` take
`?limit=`/`?offset=` and report `X-Total-Count`/`X-Limit`/`X-Offset`.

This crate is also the family's **reference implementation** of the
compliance controls in
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2 (spec §12) — a full `/api/compliance/*` surface (posture, SBOM, audit
hash-chain verify, row-integrity verify, MAC'd external-witness
checkpoints) plus a mounted FHIR R5 `PlanDefinition` surface (spec
§12.3). See [README.md](./README.md) for both endpoint tables in full;
the tables above cover only the native `/api/care-pathways` surface.

## MVP scope

CRUD + Tantivy full-text/fuzzy/phonetic search + matching, with payload validation
(`condition_codes` ICD-10 / ICD-11 / SNOMED CT SCTID Verhoeff;
`identifiers` UUID / DOI shapes; `in_language` BCP-47 syntax;
`src/validation.rs`), OpenAPI 3 +
Swagger UI (`src/openapi.rs`, `controllers/docs.rs`), an audit log +
in-memory event stream on every CRUD/merge (`models/audit_logs.rs`,
`src/streaming.rs`), record merge (`src/merge.rs` + `models/merge_records.rs`,
`POST /merge`), offline PASETO v4.public verification (`src/auth.rs`,
embeds `authentication-verifier`; `/whoami` + audit `actor`), and blanket
`/api/*` auth enforcement (`auth::enforce` + an `after_routes` middleware
in `app.rs`) wired but **off by default** — gated by
`CARE_PATHWAY_REQUIRE_AUTH`. The durable event bus's
Phase-2 outbox/relay landed (`models/event_outbox.rs`, `src/relay.rs`),
default-off via `CARE_PATHWAY_EVENT_TRANSPORT` (`memory` unless set to
`outbox`). **Tantivy full-text/fuzzy/phonetic search** (`src/search/`)
replaces the `ILIKE` name search and backs search-blocked
`check-duplicates` candidates. **Privacy** (`src/privacy.rs`) provides
field masking (`provider_name` / `provider_id`), the always-masked
`/masked` view, and the audited GDPR `/export`, wired to the ABAC `mask`
obligation via `auth::authorize_record` + `auth::care_pathway_resource_attrs`
(`care_setting`, `sensitive_setting` for the mental-health/palliative
special-category settings). A pathway *template* names no patient, so
the masked field set is thin — see `src/privacy.rs`'s module docs for
why, and for the explicit note that the patient-identifying linkage
(`pathway_instances.subject_ref`) is a separate, not-yet-addressed
surface. The durable bus's real broker sink (BUS-3, `FluvioSink` in
`src/relay.rs`, behind this crate's own `fluvio` Cargo feature, off by
default) landed 2026-08-03, ported from case-service's BUS-1 reference —
see `agents/share/event-bus.md`. Deferred
(spec §13): instance-layer
masking/authz for `subject_ref`, front-end merge
action, terminology-server code-existence checks, and the native
(non-FHIR) bulk import/export API. The published key set
is fetched over HTTP once at boot when `CARE_PATHWAY_PASETO_KEYS_URL` is
set (fetched set wins; warn + env fallback via
`CARE_PATHWAY_PASETO_KEYS` otherwise — the service always boots), **and
then a background loop keeps re-fetching it**
(`CARE_PATHWAY_PASETO_KEYS_REFRESH_SECS`, default 3600), so a rotated
key reaches a running process with no restart; the ABAC policy
(`CARE_PATHWAY_ABAC_POLICY_FILE`) hot-reloads the same way (AU-2,
2026-08-01).

The full **compliance surface** (`src/compliance/`, spec §12) — the
family's reference implementation of
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2 — landed 2026-07-25 through 2026-08-04: a tamper-evident audit hash
chain, keyed HMAC integrity MACs with per-domain HKDF subkeys
(`src/compliance/mac.rs`, embedding the shared `integrity-mac` crate),
MAC'd external-witness chain checkpoints
(`src/compliance/checkpoint.rs` — closes the tail-truncation blind spot
the hash chain alone cannot see), row-level record content hashing,
read/disclosure auditing, GDPR Art. 17 erasure, and a SOUP register +
CycloneDX SBOM + machine-checked requirement→test traceability.
`cargo loco task integrity_key`/`integrity_resign` generate, check, and
re-sign the MAC key without ever logging it. The mounted **FHIR R5**
surface (`src/fhir/`, `controllers/fhir.rs`, spec §12.3) maps the
stored DTO to `PlanDefinition`, with `$validate`, conditional SMART
discovery, and a durable (worker + artifact-store) Bulk Data `$export`.
Family-wide activation order for the compliance controls (all default
off/inert):
[`agents/share/runbooks/integrity-activation.md`](../../agents/share/runbooks/integrity-activation.md).

Auth pivot done in this crate: the family moved from RS256 JWT + JWKS to
cookie sessions + short-lived PASETO v4.public verified offline against a
published Ed25519 key (RS256/JWKS decommissioned); the
`CARE_PATHWAY_REQUIRE_AUTH` flag and enforcement semantics are unchanged,
only the credential changed. See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); `src/auth.rs` verifies PASETO via the
`authentication-verifier` crate (0.2, `from_paseto_keys_*`).

## Golden rules

1. **Spec-first.** Update `spec/index.md` with behavioural changes.
2. **Loco-idiomatic.** Endpoints are loco controllers in `app.rs`; new
   tables are `sea-orm-migration` migrations.
3. **Reuse the matcher type.** Do not fork a `CarePathway` DTO.
4. **Auth** comes from the central
   [authentication-service](../../authentication/authentication-service-with-loco):
   cookie sessions + offline PASETO v4.public verification.

## Layout

```
src/
├── app.rs                 loco Hooks (routes, workers, truncate, key/policy-refresh spawn)
├── bin/main.rs             loco CLI entrypoint
├── bin/sbom.rs             `cargo run --bin sbom` — SBOM to stdout, no server boot
├── controllers/
│   ├── care_pathways.rs   CRUD + match + check-duplicates + merge + masked/export + audit/events/disclosures + erase + whoami
│   ├── compliance.rs      posture, SBOM, audit-chain verify, record verify, checkpoint take/verify
│   ├── fhir.rs             mounted FHIR R5 PlanDefinition CRUD/search + $validate + SMART + $export
│   ├── insights.rs         directory/coverage/variants/providers/languages registry lenses
│   ├── instances.rs        instance lifecycle/review/urgency/team/steps/outcomes + caseload/overdue/care-team-load
│   ├── tba.rs              time-based analysis: segment + clock recording, per-instance and cohort views, constraints, flow
│   ├── docs.rs             OpenAPI JSON + Swagger UI
│   └── metrics.rs          root /metrics.prom Prometheus endpoint
├── compliance/
│   ├── mod.rs              posture assembly, data-protection declarations, safety class
│   ├── audit_chain.rs      SHA-256 tamper-evident hash chain over audit_logs
│   ├── mac.rs               keyed HMAC integrity MAC (embeds shared integrity-mac crate, per-domain HKDF subkeys)
│   ├── checkpoint.rs        MAC'd external-witness chain checkpoints (tail-truncation blind spot)
│   ├── record_integrity.rs  row-level content_hash over care_pathways
│   ├── disclosure.rs        purpose-of-use vocabulary + read/disclosure auditing (HIPAA §164.528)
│   ├── erasure.rs           GDPR Art. 17 erasure against the immutable chain
│   ├── soup.rs               SOUP register + CycloneDX SBOM (embeds compliance/soup.tsv + Cargo.lock at compile time)
│   └── bulk.rs               FHIR Bulk Data $export in-process shape (job orchestration lives in bulk/ + workers/)
├── fhir/
│   ├── mod.rs               to/from PlanDefinition conversions
│   ├── profile.rs           family-local StructureDefinition profile + terminology validation
│   ├── resources.rs         resource structs, OperationOutcome, Bundle
│   └── search.rs             FHIR search-param parsing → searchset Bundle
├── bulk/
│   ├── mod.rs               durable bulk_jobs table + artifact store, shared by FHIR $export (native bulk import/export is future work)
│   └── store.rs              ArtifactStore trait + local-filesystem dev backend
├── workers/
│   └── bulk_export.rs        bg_pg worker materialising FHIR $export NDJSON off the request path
├── tasks/
│   ├── search.rs             `cargo loco task` Tantivy reindex + boot-time reindex-if-empty
│   ├── integrity_key.rs      generate/check/report the MAC root key (never logs the key)
│   └── integrity_resign.rs   re-sign existing rows after a MAC key rotation
├── metrics.rs             process-wide Prometheus registry (CRUD/merge counters + http_requests_total)
├── auth.rs                offline PASETO v4.public verification (AuthUser/MaybeAuthUser) + ABAC, both reloadable (ReloadableVerifier/ReloadablePolicy — AU-2 key/policy hot-reload)
├── version.rs             `Accepts-version` header negotiation middleware (agents/share/api-versioning.md)
├── instances.rs            pure instance lifecycle state machine (active↔on_hold→terminal)
├── tba.rs                 pure time-based analysis: interval union/subtract, the four-bucket
│                          clock partition, gaps, handoffs, nearest-rank percentiles, the NHS
│                          access-standard catalogue, cohort rollup, constraint ranking,
│                          Little's Law. No I/O; `as_of` is a parameter, so it is deterministic
├── merge.rs               pure record-merge logic (merge_pathways)
├── openapi.rs             hand-written OpenAPI 3 document
├── privacy.rs             field masking (provider name/id) + GDPR export envelope
├── relay.rs               durable-bus Phase 3 outbox relay (poll/ack loop) + FluvioSink (BUS-3, `fluvio` feature)
├── search/                Tantivy full-text/fuzzy/phonetic index (index.rs schema + mod.rs engine)
├── streaming.rs           CRUD/merge event stream — Phase 1 durable-bus
│                          envelope (Envelope) + EventPublisher seam +
│                          InMemoryPublisher; frozen EventView projection
├── validation.rs          name + condition-code (ICD/SNOMED) + input-size-cap checks → 422
├── models/
│   ├── care_pathways.rs   CRUD helpers over the stored payload (sets content_hash on every write)
│   ├── audit_logs.rs      audit-trail record/query helpers (chains under pg_advisory_xact_lock)
│   ├── merge_records.rs   merge-history record/query helpers
│   ├── event_outbox.rs    durable-bus Phase 2: OutboxInsert::from_envelope mapping + enqueue (tx-generic) + relay poll/ack
│   ├── bulk_jobs.rs        queued→running→terminal FHIR $export job lifecycle
│   └── _entities/{care_pathways,audit_logs,merge_records,event_outbox,bulk_jobs,pathway_instances,instance_steps,instance_team,instance_events,instance_measures,instance_segments}.rs  SeaORM entities
migration/src/            …care_pathways, …audit_logs, …merge_records, …event_outbox, …instances (m20260720_…), …outcomes, …compliance (m20260725_000007), …record_integrity, …bulk_jobs
config/                   development/production/test yaml
compose.fluvio.yaml        opt-in local Fluvio broker (`fluvio` feature, BUS-3; not part of CI)
Dockerfile.fluvio-cli       support image for compose.fluvio.yaml's sc-setup step
tests/fluvio_relay.rs       `fluvio`-feature-gated, #[ignore]d live-broker round-trip test
compliance/                crate-root IEC 62304 evidence: lifecycle.md, soup.tsv, traceability.tsv
```

## Container image

`Dockerfile` (multi-stage, Debian 13 slim runtime) builds this crate's
production image. **Build context must be the repository root**, not
this directory — this crate's sibling path dependencies
(`integrity-mac`, `authentication-verifier`, `care-pathway-matcher`)
live outside `care-pathway/care-pathway-service-with-loco/`:

```sh
podman build -f care-pathway/care-pathway-service-with-loco/Dockerfile \
  -t care-pathway-service .   # run from the repository root
```

Verified end-to-end (2026-08-03): builds clean, boots against a real
Postgres, and `GET /_health` returns `200`. This exercise found and
fixed two real bugs: (1) `src/compliance/soup.rs` embeds the IEC 62304
SOUP register via a relative `include_str!` from `compliance/soup.tsv`
at the crate root — the Dockerfile now copies that directory
explicitly. (2) `config/production.yaml`'s `mailer.smtp.auth.user`/
`password` used an unquoted Tera `{{ get_env(name="…", default="") }}`
call, which renders as YAML `null` (not `""`) when the env var is
unset — loco's `SmtpAuth` fields are `String`, not `Option<String>`, so
this failed config parsing at boot with "invalid type: unit value,
expected a string". This crate's `.gitignore` also excluded
`config/production.yaml` entirely (a loco scaffold default nobody had
removed), which is why the SMTP bug had never been caught — the file
never left this machine, so no other checkout could exercise it. Both
are fixed (the file is now tracked; see the `.gitignore` for the
reasoning). See `.containerignore` at the repository root (excludes
every crate's `target/`, or the build context would try to copy
hundreds of GB of build artifacts). The wired multi-service
`examples/compose/` stacks (DEP-1) that build on this are not yet
written.
