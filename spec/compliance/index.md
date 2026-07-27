# Compliance

Monorepo-wide specification for **regulatory compliance** across the
**Main X Index** family. This is the cross-cutting source of truth that
ties the implemented technical controls — privacy, auditability,
authentication, persistence — to the legal regimes the family targets.
The two short regime briefs it expands are
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
and
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md).

It is deliberately **honest about maturity**: each control is marked
*implemented*, *deployment-gated* (the code is ready but the protection
is provided by how the service is deployed), or *deferred* (not yet
built in some services). Compliance is a property of the running
deployment, not only of the code — this document keeps that distinction
explicit.

## Contents

1. [Target regimes](#1-target-regimes)
2. [Obligation → control → location](#2-obligation--control--location)
3. [Per-entity data-sensitivity classification](#3-per-entity-data-sensitivity-classification)
4. [AI management (ISO/IEC 42001) — explainable matching](#4-ai-management-isoiec-42001--explainable-matching)
5. [Auditability & accountability](#5-auditability--accountability)
6. [Gaps & deferred controls (honest)](#6-gaps--deferred-controls-honest)
7. [Cross-references](#7-cross-references)
8. [Extended frameworks — HIPAA, GDPR/EHDS, ONC/HTI, IEC 62304](#8-extended-frameworks--hipaa-gdprehds-onchti-iec-62304)

---

## 1. Target regimes

The family targets two overlapping families of regime: **healthcare**
(for the entities that carry clinical or patient-linked data) and
**technology / data-protection** (for every service). The two share UK
DPA 2018 and UK/EU GDPR.

### 1.1 Healthcare

Source: [`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md).

| Regime | One-line scope |
|---|---|
| **US HIPAA** | US health-information privacy & security — the who/what/when audit trail and safeguards over protected health information. |
| **UK DPA 2018** | UK statute implementing GDPR — lawful processing, data-subject rights, special-category (health) data. |
| **UK Common Law Duty of Confidentiality** | Obligation to keep patient information confidential and disclose only with a lawful basis. |
| **UK NHS Act 2006, Section 251** | The legal gateway permitting use of confidential patient information for defined purposes without consent. |
| **UK GDPR** | The retained-EU GDPR as it applies in the UK post-Brexit — access, erasure, minimisation, lawful basis. |
| **EU GDPR** | EU-wide data protection — same rights and principles for EU data subjects. |
| **EU EHDS** — Reg. (EU) 2025/327 | European Health Data Space — primary use (care delivery, EHR exchange) vs. secondary use (research, policy) via health-data access bodies, data permits, and secure processing environments. Applies in stages; a deployment confirms which chapter is live for it. |
| **US ASTP/ONC certification** — 45 CFR Part 170 | Health IT Certification Program (the HTI rules). §170.315(g)(10) standardized API: US Core profile conformance, SMART App Launch, FHIR Bulk Data `$export`, tested by Inferno; §170.315(d)(2)/(3)/(10) auditable events and audit reports. |
| **IEC 62304** (+ ISO 14971, IEC 82304-1) | Medical device software life-cycle processes — safety classification, SOUP register, traceability, configuration management. Engages only where a deployment crosses into clinical decision support (see §8.4). |

### 1.2 Technology / data protection

Source: [`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md).

| Regime | One-line scope |
|---|---|
| **UK DPA 2018** | UK data-protection statute (shared with §1.1). |
| **UK GDPR** | UK retained GDPR (shared with §1.1). |
| **EU GDPR** | EU data protection (shared with §1.1). |
| **ISO/IEC 27001 (ISMS)** | Information Security Management System — access control, audit, change management, secret handling. |
| **ISO/IEC 42001:2023 (AIMS)** | AI Management System — governance, transparency, and explainability of algorithmic decisioning (see §4). |

---

## 2. Obligation → control → location

Every obligation below maps to an **implemented** control in the
repository (or is flagged where it is deployment-gated or deferred). The
"Where" column names the module / endpoint and links the cross-cutting
spec that owns the detail.

| Obligation (regime) | Implemented control | Where |
|---|---|---|
| **Audit trail — who / what / when** (HIPAA; ISO 27001) | Durable append-only `audit_logs` / `audit_log` table; best-effort write on every CRUD/merge, stamped with the bearer-token `sub` actor + before/after JSON snapshot. | `src/models/audit_logs.rs` (Loco) / `src/db/audit.rs` (MPI); see [auditability](../auditability/index.md). |
| **Right of access — GDPR Art. 15** | Entity export `GET /<plural>/{id}/export` (full unmasked record); account export `GET /api/auth/account/export` (users + sessions + auth_events); per-subject `GET /api/auth/account/audit`. | `src/privacy/mod.rs` (`export_<entity>_data`); `controllers/auth.rs` (`export_account`); see [privacy §2](../privacy/index.md). |
| **Right to erasure — GDPR Art. 17** | Two semantics: **soft-delete** tombstone on every MPI `DELETE` + merge loser (reversible, preserves audit); **anonymisation** on `DELETE /api/auth/account` (soft-delete + overwrite `email`/`name` + revoke sessions + audit, irreversible). | `models/users.rs` (`Model::erase`); `controllers/auth.rs` (`erase_account`); see [privacy §3](../privacy/index.md). |
| **Consent / lawful basis** (GDPR; DPA) | First-class `Consent` model (`ConsentType` = DataProcessing / DataSharing / Marketing / Research / EmergencyAccess; `ConsentStatus` = Active / Revoked / Expired) + `has_active_consent` checker. | `src/models/consent.rs`, `src/privacy/mod.rs`; see [privacy §4](../privacy/index.md). |
| **Data minimisation / masking** (GDPR; HIPAA) | View-time masking `mask_value(value, visible_chars)` (keeps last N chars, char-safe for multibyte); masked-view endpoint `GET /<plural>/{id}/masked`; inline `mask_sensitive=true` on search. | `src/privacy/mod.rs` (`mask_person`/`mask_worker`/`mask_place`); see [privacy §1](../privacy/index.md). |
| **Access control / authentication** (ISO 27001; GDPR) | Central SSO: passwordless magic-link establishing a server-side **Postgres cookie session** (opaque httpOnly cookie, no token in the browser); cross-service auth via short-lived **PASETO v4.public** tokens (Ed25519 — peers hold only the public key) verified **offline** against the published key at `/.well-known/paseto-keys`; CSRF protection on mutating cookie requests; and **blanket `/api/*` enforcement** middleware (default-off, env-gated per service). Replaces the prior RS256 JWT + JWKS model. | `authentication-verifier` crate embedded in `src/auth.rs`; see [authentication §4, §7](../authentication/index.md) and [../../agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md). |
| **Anti-enumeration** (confidentiality; GDPR) | Passwordless `signup` / `magic-link` **always return `200`** regardless of account existence; rate limiter keys on request volume, not existence; tokens never logged. | `controllers/auth.rs`, `src/rate_limit.rs`; see [authentication §8](../authentication/index.md), [privacy §5](../privacy/index.md). |
| **Encryption in transit** (ISO 27001; GDPR) | SeaORM uses the `runtime-tokio-rustls` runtime; DB connections support TLS. Public HTTP TLS is terminated at the deployment edge. | **Deployment-gated** — see [postgresql](../postgresql/index.md) and §6. |
| **Encryption at rest** (ISO 27001; GDPR) | `pgcrypto` available for hashing/encryption; volume/disk encryption is an infrastructure concern. | **Deployment-gated** — see §6. |
| **No secrets in audit / logs** (ISO 27001; HIPAA) | Audit rows + event envelopes + GDPR export carry **identity and outcome only** — never bearer tokens, magic-link tokens, signing keys, or password hashes; magic-link tokens are never logged. | [auditability §2.6, §4.1](../auditability/index.md); [authentication §9](../authentication/index.md). |

---

## 3. Per-entity data-sensitivity classification

Compliance obligations attach where personal data actually lives. Not
every entity in the index is a data subject; classification scopes the
controls. This mirrors [privacy §6](../privacy/index.md).

| Entity | Classification | Notes |
|---|---|---|
| **Person** | Personal / special-category | Names, DOB, tax/national IDs, identity documents, contacts, addresses. Full masking + export + consent. |
| **Worker** | Personal / special-category | Workforce / professional identities; same controls as person. |
| **Case** | Personal / special-category | Governmental case data (benefits, legal, social-services, complaints) **is** personal data — most consequential of the deferred-masking services (§6). |
| **Care pathway** | Mixed | Clinical pathway **templates** are largely reference data; patient linkage is personal / special-category (healthcare regimes apply). |
| **User (authentication)** | Personal | Email + name only — minimal by design (passwordless). |
| **Organization** | Largely public | Legal name, jurisdiction, registry IDs (LEI / DUNS / ROR) are public; **contact fields** (telecom, contact address) are personal. |
| **Place** | Largely public | Geographic places are public; contact / occupant fields are personal. |
| **Thing / Event / Course** | Largely non-personal | Asset / time-window / course-template registries; contact fields, where present, are personal. |
| **case-folder subjects** | Opaque identifiers | The NHS case-note-folder tracker stores subjects as **opaque IDs** referencing person / place / worker by ID, not embedded PII — the move audit trail carries no demographic data of its own. |

---

## 4. AI management (ISO/IEC 42001) — explainable matching

The family's only algorithmic decisioning is **record matching**, and it
is built to be **explainable**, which is the property ISO/IEC 42001
(AIMS) cares about.

- **Deterministic + probabilistic, not opaque ML.** Matching is
  rule-based short-circuiting (exact tax-ID / LEI / DUNS / GLN / docket
  matches pin the score) plus weighted fuzzy scoring (Jaro-Winkler,
  Levenshtein, Soundex, Haversine). There is **no trained model**, no
  black-box inference, and no learned weights — the weights are
  configured and inspectable.
- **Score breakdown is returned.** Match responses include the
  per-component scores, so a human can see *why* two records were judged
  similar. Confidence is classified into certain / probable / possible /
  unlikely against configurable thresholds.
- **Human-in-the-loop merge.** High-confidence pairs may auto-merge;
  everything else lands in a review queue with status tracking
  (Pending / Confirmed / Rejected / AutoMerged) — a human confirms before
  a merge is finalised. Merges are themselves audited (§5).

Because the decisioning is transparent, deterministic where it matters,
and accompanied by a per-component rationale, it satisfies the AIMS
expectations of explainability and traceability without requiring a
model-governance regime. See [matching](../matching/index.md) and
[merge](../merge/index.md).

---

## 5. Auditability & accountability

Auditability is the backbone of the family's compliance posture; it is
specified in full in [auditability](../auditability/index.md). The
compliance-relevant guarantees:

- **Immutable, append-only.** The durable `audit_logs` / `audit_log`
  tables have **no update or delete path** — helpers only `insert` and
  query. The trail cannot be silently rewritten.
- **Actor attribution.** Every audit row and event envelope is stamped
  with the bearer-token `sub` (the caller's user pid) when a verified
  cross-service token was presented (target: PASETO v4.public;
  RS256/JWKS decommissioned); `NULL` otherwise. The same actor flows into
  both mechanisms, so they agree on identity.
- **Soft-delete preserves history.** "Deletion" is a soft delete
  (`active = false` / `deleted_at`); the record **and all its audit
  rows survive**, so the forensic trail outlives the record.
- **Durable system of record.** The audit log lives in Postgres and is
  the compliance system of record; the operational event stream
  (in-memory ring buffer, Phase 1) is explicitly **not** relied upon for
  compliance.
- **No secret leakage.** Audit rows carry identity and outcome only —
  never tokens, keys, or password hashes — and authentication outcomes
  never leak account existence at the wire.

---

## 6. Gaps & deferred controls (honest)

Compliance maturity is uneven by design, and some controls are properly
the responsibility of the deployment or the operating organisation, not
the code. This section states those gaps plainly.

### 6.1 Deferred in code (some services)

| Gap | Detail |
|---|---|
| **Per-field masking + GDPR export** | The MPI services (person / worker / place) ship the full privacy stack. The newer **loco services — organization, care-pathway, case — defer per-field masking and the `/export` GDPR surface**; they ship CRUD + matching + audit + event streaming first. Because **case data is personal data**, its masking/export gap is the most consequential and is flagged in its `spec.md`. When implemented, these MUST follow the `mask_value` / `/{id}/masked` / `/{id}/export` shape so behaviour stays uniform. See [privacy §7](../privacy/index.md). |
| **Blanket `/api/*` enforcement** | Implemented but **default-off**, gated per service (`ORGANIZATION_REQUIRE_AUTH`, `CARE_PATHWAY_REQUIRE_AUTH`, `CASE_REQUIRE_AUTH`); the older Axum services (person / worker / place) are a separate follow-up. The guard requires a valid PASETO token (service-to-service) or session (BFF); RS256/JWKS decommissioned. Activation is an ops decision (turning it on before the front-end attaches a credential would `401` every request). See [authentication §7](../authentication/index.md). |
| **Durable event bus** | The event stream is a Phase-1 in-memory ring buffer; the transactional outbox → Fluvio sink is planned (the audit log, not the stream, is the durable compliance record). See [auditability §6](../auditability/index.md). |

### 6.2 Deployment-gated (code ready, protection comes from the deployment)

| Control | Why it is deployment-gated |
|---|---|
| **Encryption in transit (TLS)** | SeaORM runs on `runtime-tokio-rustls` and DB TLS is supported; public HTTP TLS is terminated at the ingress / load-balancer edge. The protection is real only when the deployment configures it. |
| **Encryption at rest** | `pgcrypto` is available for field-level hashing/encryption, but full disk/volume encryption is provided by the database host / storage layer. |
| **Secrets management** | Production signing keys (the PASETO v4.public Ed25519 keypair; RS256/JWKS decommissioned) and DB credentials come from the environment edges; the committed dev keys are **dev-only**. A production secrets manager is an infrastructure concern. See [authentication §3.2](../authentication/index.md). |

### 6.3 Organisational process (not code)

A **DPIA** (Data Protection Impact Assessment), **Records of Processing
Activities** (GDPR Art. 30), an **ISMS** certification (ISO 27001),
controller/processor agreements, and the NHS Act s.251 legal-basis
determination are **organisational-process artefacts**. The codebase
provides the technical controls those processes rely on (audit trail,
access control, minimisation, erasure), but the assessments and records
themselves are produced and maintained by the operating organisation,
not generated by the service.

---

## 7. Cross-references

| Topic | Where |
|---|---|
| Healthcare regime brief | [`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md) |
| Technology regime brief | [`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md) |
| Privacy / masking / GDPR access & erasure / consent | [privacy](../privacy/index.md) |
| Audit log & event streaming | [auditability](../auditability/index.md) |
| Authentication / access control / anti-enumeration / secrets | [authentication](../authentication/index.md) |
| Persistence / TLS runtime / `pgcrypto` / soft-delete columns | [postgresql](../postgresql/index.md) |
| Explainable matching (ISO 42001 angle) | [matching](../matching/index.md) · [merge](../merge/index.md) |
| Extended frameworks (HIPAA detail, EHDS, ONC/HTI, IEC 62304) | §8 below · [`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md) §2 |

---

## 8. Extended frameworks — HIPAA, GDPR/EHDS, ONC/HTI, IEC 62304

§2 maps the obligations the family already satisfies. This section covers
the four frameworks that impose **further** technical obligations, and is
deliberately blunt about how far the code has actually come. The regime
detail lives in
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; this section is the repository-side status.

**Reference implementation.** All four are implemented first in the
[care-pathway service](../../care-pathway/care-pathway-service-with-loco/)
— `src/compliance/` (audit hash chain, disclosure accounting, erasure,
posture, SOUP/SBOM), `src/fhir/profile.rs` + `src/controllers/fhir.rs`
(profile/terminology validation, SMART discovery, Bulk Data), and the
repository-level `compliance/` artefacts. **Every other service is
unchanged**, so the rows below say "care-pathway" where that is the truth.

### 8.1 HIPAA — read-auditing and tamper-evident history

| Obligation | Control | Where | Status |
|---|---|---|---|
| §164.312(b) audit controls over **reads**, not just writes | Audit rows for read / search / export / FHIR-read, recording the accessed record, actor, purpose-of-use, and whether the access was a **disclosure** | `src/compliance/disclosure.rs`; env-gated by `CARE_PATHWAY_AUDIT_READS` | care-pathway ✔ (default-off); other services **deferred** |
| §164.312(c)(1)–(2) integrity / corroboration | SHA-256 **hash chain** over `audit_logs` — each row binds its content and its predecessor's hash, so any insertion, deletion or edit breaks verification | `src/compliance/audit_chain.rs`; verified at `GET /api/compliance/audit/verify` | care-pathway ✔; other services **deferred** |
| §164.528 accounting of disclosures | Disclosure-flagged audit rows, queryable per record over the statutory 6-year window | `GET /api/care-pathways/{pid}/disclosures` | care-pathway ✔ |
| §164.312(a)(1), (d) access control / authentication | Blanket guard + ABAC, offline PASETO verification | §2 rows; unchanged | family-wide, **default-off** (§6.1) |
| §164.312(e) transmission security | TLS at the edge; `runtime-tokio-rustls` for DB | §6.2 | **deployment-gated** |

The honest gap: the audit chain proves *the trail* has not been rewritten;
it does not prove *the entity rows* have not been. Row-level integrity
hashing over `care_pathways` is not built.

### 8.2 GDPR / EU EHDS — erasure vs. history, residency, lawful basis

| Obligation | Control | Where | Status |
|---|---|---|---|
| Art. 17 erasure **against an append-only trail** | **Redaction**: the audit row's content is destroyed and stamped `redacted_at`, while its stored hash and chain linkage survive — the trail stays verifiable and still proves an event occurred, but the personal data is gone. Irreversible. | `src/compliance/erasure.rs`; `POST /api/care-pathways/{pid}/erase` (a destructive action under ABAC) | care-pathway ✔ |
| Art. 6 / 9 lawful basis + Art. 30 records of processing | Declared deployment lawful basis and Art. 9 condition, stamped into every audit row's `context` and reported by the posture endpoint | `src/compliance/mod.rs` (`CARE_PATHWAY_LAWFUL_BASIS`, `CARE_PATHWAY_ART9_CONDITION`) | care-pathway ✔ |
| Ch. V cross-border transfer | Declared `CARE_PATHWAY_DATA_RESIDENCY` region + transfer-safeguard marker, surfaced at `GET /api/compliance`; an export destined outside the declared region is recorded as a **transfer** audit event | `src/compliance/mod.rs`, `src/compliance/disclosure.rs` | care-pathway ✔ (declaration + audit); **enforcement** of a transfer block is deployment-side |
| EHDS primary vs. secondary use | `X-Purpose-Of-Use` request header captured per access (`care`, `research`, `policy`, `statistics`, …) and recorded on the audit row, so secondary-use access is separable from care delivery | `src/compliance/disclosure.rs` | care-pathway ✔ |
| EHDS data permits / secure processing environments | **Not built** — permits and SPEs are an operating-organisation and infrastructure concern; the service contributes the purpose-of-use marking and the export audit that a permit regime consumes. | — | **organisational / deferred** |
| Art. 15 access, Art. 32 security | §2 rows (export endpoints, masking, encryption) | §2 | mixed — see §6.1 |

### 8.3 ONC / HTI — profile and terminology validation

| Obligation | Control | Where | Status |
|---|---|---|---|
| Profile conformance (US Core–style must-support + cardinality) | A declared profile URL in `meta.profile`, plus structural validation producing one `OperationOutcome` issue per violation | `src/fhir/profile.rs` | care-pathway ✔ |
| Terminology binding validation | Codes validated against the value set their element is **bound** to (system URI recognised, code well-formed for that system) rather than merely non-blank; unbound systems warn instead of failing | `src/fhir/profile.rs` + `src/validation.rs` | care-pathway ✔ |
| `$validate` operation | `POST /fhir/PlanDefinition/$validate` returning an `OperationOutcome` without persisting | `src/controllers/fhir.rs` | care-pathway ✔ |
| SMART App Launch discovery | `GET /fhir/.well-known/smart-configuration` + a `CapabilityStatement.rest.security` SMART declaration | `src/controllers/fhir.rs` | care-pathway ✔ (**discovery only** — the family's credential is PASETO, not SMART OAuth; see the caveat below) |
| FHIR Bulk Data `$export` | Async kickoff → status poll → NDJSON manifest and files | `src/controllers/fhir.rs`, `src/compliance/bulk.rs` | care-pathway ✔ |
| Inferno test suite | Not run in CI. The conformance points Inferno checks are pinned by unit tests instead (profile validation, terminology binding, capability/route agreement, Bulk Data flow). | `tests/` | **deferred** |

**Certification caveat, stated plainly.** ONC certification targets **FHIR
R4 + US Core**; this family serves **FHIR R5**, and `PlanDefinition`,
`Task`, `Basic` and `Appointment` have no US Core profile. **No crate here
is a certifiable health-IT module and none claims to be.** What is
implemented is the conformance *machinery* — declared profiles, structural
and terminology validation, `$validate`, SMART discovery, Bulk Data — which
is independently useful and is the precondition for ever running an
Inferno-style suite against us.

### 8.4 IEC 62304 / SaMD — lifecycle, SBOM, traceability, reproducible builds

| Obligation | Control | Where | Status |
|---|---|---|---|
| §5.1 development plan + declared **safety classification** | Classification declared per service and reported at runtime | `compliance/lifecycle.md`, `GET /api/compliance` | care-pathway ✔ |
| §5.3.3 / §8.1.2 **SOUP register** | Every third-party dependency listed with identity, version, purpose, and its safety-relevant notes | `compliance/soup.md`, served at `GET /api/compliance/sbom` | care-pathway ✔ |
| Supply-chain evidence / SBOM | CycloneDX SBOM generated from the real dependency graph; advisory + licence gating already in place | `scripts/sbom.sh`, `deny.toml` (`cargo-deny`, SEC-I1); CI stages in `.github/workflows/ci.yml` + `.woodpecker.yml` | care-pathway ✔; **CI wired** (fmt / clippy / test / test-db / deny / SBOM on both remotes) |
| §5.2–5.5 **traceability requirement → test** | A checked-in matrix mapping every functional requirement to the tests that verify it, **machine-checked** so a requirement cannot silently lose its verification | `compliance/traceability.tsv`, enforced by `tests/traceability.rs` | care-pathway ✔ |
| §8 configuration management — reconstructible releases | Pinned toolchain, `SOURCE_DATE_EPOCH`, recorded commit + build inputs, optional signing | `scripts/build-reproducible.sh`; build provenance at `GET /api/compliance` | care-pathway ✔ (script + provenance); **signing key custody is deployment-side** |
| §7 risk management (ISO 14971), §9 problem resolution | Hazard analysis, risk controls, and the defect-to-fix trail are **organisational-process artefacts** the operating organisation maintains. | — | **organisational** (§6.3) |

**Device-qualification caveat.** A registry of pathway *templates* is
generally not a medical device. The line is crossed when a deployment
surfaces pathway steps as clinical decision support, or when an individual
patient is tracked through a pathway — which the care-pathway service's
**instance layer** (`pathway_instances`, `instance_steps`) does. That is
why care-pathway carries the evidence artefacts and declares a
classification; the non-clinical registries build the same artefacts as
engineering practice without the device framing.

### 8.5 Rollout

1. **Contracts.** `agents/share/compliance-for-healthcare.md` §2 + this
   section + the per-entity `spec/12-compliance.md` sections. ✔
2. **Reference implementation.** care-pathway service — all four. ✔
3. **Copy the audit chain + read-auditing** to the personal-data services
   first (person, worker, case), where HIPAA and GDPR bite hardest.
   **case ✔ (2026-07-25)** — chain + read/disclosure auditing +
   `/audit/verify` + the §164.528 accounting, which is gated behind the
   same record-level authorization as reading the case, so the
   disclosure history cannot be more open than the record it describes.
   case adopts the audit half only; the GDPR residency/lawful-basis
   declarations, FHIR conformance and the SBOM bundle follow at steps
   4–5. **person ✔ (2026-07-26, chain + read/disclosure auditing)** — ported rather than
   copied: person's `audit_log` has a UUID primary key, so the chain
   orders on a new `seq BIGSERIAL` rather than the PK, and the digest
   binds the old/new value pair plus request provenance
   (`ip_address`, `user_agent`) so *who* acted cannot be rewritten
   while *what* they did stays intact. Read/disclosure auditing covers
   `get` / `masked` / `search` / `export`, and the dedicated §164.528
   accounting endpoint (`GET /api/persons/{id}/audit/disclosures`) is
   built, gated by the same record-level authorization as reading the
   record.
   **worker ✔ (2026-07-26, chain + read/disclosure auditing +
   `/audit/verify`)** — same port as person, whose `audit_log` it
   shares; read auditing covers `get` / `masked` / `search` / `export`,
   and the verification endpoint is mounted and pinned by an
   end-to-end test; it carries the same §164.528 accounting endpoint as
   person (`GET /api/workers/{id}/audit/disclosures`).
   Both are now **enrolled in CI's DB suites** (2026-07-26), so their
   chains are verified against Postgres on every run. Getting there
   meant fixing pre-existing defects that had nothing to do with
   compliance: a migration in each crate that built a `gin_trgm_ops`
   index on a `text[]` column and created `pg_trgm` after first using
   it (on worker this halted the whole migration chain, so `audit_log.seq`
   never existed); person's rename migration leaving `patient_id` columns
   the entities call `person_id`; a NUL byte in person's bulk-import
   advisory-lock key, which Postgres `text` cannot carry, so every
   identifier-keyed import row had always failed; a Tantivy index path no
   fixture created in both crates; and worker's loss of the
   `#[serde(default)]` attributes person still had, which turned an
   omitted optional field into a `422`. CI's DB stage now also drops and
   recreates each database and applies the crate's SQL migrations before
   running, and runs `--test-threads=1` — these suites assert on
   whole-table state (an entire verified `audit_log`, its row count, its
   `MIN(seq)`), so a leftover database or a concurrent writer produces
   failures that look like chain defects but are not.
   **Step 3 is now complete for all three.** As of 2026-07-26 case,
   person, and worker each carry the chain, read/disclosure auditing,
   `/audit/verify`, the §164.528 accounting endpoint, and **GDPR Art. 17
   erasure by redaction** — the audit trail's content is destroyed while
   each row's `hash` and `prev_hash` survive, so the chain keeps
   verifying and Art. 17 and §164.312(c) are satisfied together rather
   than traded off. Two entity-specific extensions were needed beyond
   the care-pathway reference: **case** also withdraws its cross-service
   links, because a surviving `subject_of` edge would erase the details
   of a proceeding while preserving the accusation; and **person and
   worker**, being relational rather than single-JSONB, delete their
   child rows and scrub the parent (worker including
   `worker_assessments`, the psychometric results that are the most
   sensitive data it holds) inside one transaction, writing back a
   single tombstone name so an erased record degrades cleanly instead of
   breaking every read path that assumes a name exists.
> **Resolved (2026-07-26) — the database audit triggers are dropped in
> person and worker.** `m20260726_000003_drop_audit_triggers` removes
> `audit_patients_changes` / `audit_workers_changes` and
> `audit_organizations_changes`, which appended rows to `audit_log` from
> the database — where the application's hashing and advisory lock are
> unreachable — so those rows carried a NULL `hash` and verification
> skipped them (16 of 28 rows on a person run). They went rather than
> stayed because they were **a log, not evidence**: a tolerated unchained
> row can be inserted without registering as a break and deleted without
> breaking linkage, so a trigger row was as forgeable as the edit it
> claimed to witness. They also carried **worse provenance** than the
> application's own row (`user_id` from the row's `created_by` column
> rather than the authenticated caller; no `ip_address` or `user_agent`),
> **duplicated** what the repository already chains in the same
> transaction, and were **narrower than they looked** — they existed only
> on the parent tables, never on the child tables where most personal data
> lives, which corrects the earlier claim here that they covered changes
> the application did not audit separately. The genuine gap they gesture
> at — a **raw-SQL edit to an entity row**, invisible to any
> application-level audit — is properly served by row-level **record
> integrity** (a per-row content hash, as in care-pathway's
> `src/compliance/record_integrity.rs`), which remains open for person and
> worker. Existing trigger rows are left in place: deleting them would
> destroy history, and rewriting them would achieve nothing since they
> carry no digest to invalidate. The verify report keeps reporting
> `unchained` so the historical gap stays visible, and a DB-gated test
> pins that on a fresh database a full CRUD cycle leaves **zero**
> unchained rows — verified to fail when the triggers are restored. The
> loco-idiomatic services (care-pathway, case) never had such triggers.

> **Fixed (2026-07-26) — the `entity_type` vocabulary had split in
> person and worker.** Mutation rows were written as `"Person"` /
> `"Worker"`, but the read-auditing path added with the audit chain
> wrote `"person"` / `"worker"`. Every per-entity audit query filters on
> one spelling, so it silently returned none of the other's rows: an
> accounting of disclosures would have read as empty while disclosures
> were being recorded, and the pre-existing `GET
> /api/<plural>/{id}/audit` endpoint has been missing read rows since
> read-auditing landed. New rows use the capitalised spelling
> throughout, and the queries accept both via `IN` so rows already
> written are not orphaned — `IN` also keeps the `(entity_type,
> entity_id)` index usable, which a case-insensitive comparison would
> not. Reads go through **one shared list** per crate
> (`ENTITY_TYPE_SPELLINGS` / `entity_type_spellings`), applied to both
> `get_logs_for_entity` and `disclosures_for_entity` so the two cannot
> drift apart again, and covering the trigger spellings (`"patient"` /
> `"worker"`) as well. Only the canonical name expands, so an unrelated
> type such as `"PersonBulkExport"` is not silently widened. Historical
> rows are deliberately **not** rewritten: `entity_type` is bound into the
> chain's row digest, so an `UPDATE` normalising it would make every
> affected chained row fail verification — the chain would correctly
> report that someone had edited the audit trail, because someone had.
> Tolerating the spelling on read is the only option that keeps both the
> history and its integrity.

> **Row-level record integrity reached person and worker (2026-07-26).**
> Both now carry a `content_hash` over the record, recomputed on every
> write, with a `GET /api/records/verify` endpoint — the complement to
> `/api/audit/verify`, and the proper answer to the raw-SQL entity edit
> that the dropped database triggers only pretended to witness. Unlike the
> care-pathway reference, which hashes one JSONB column, these hash the
> **assembled** record: a person's or worker's identifiers, names, and
> addresses live in child tables, and a parent-row digest would have
> repeated exactly the narrowness that made the triggers worthless.
> Existing rows are not back-filled — computing a hash from current
> content would certify whatever an attacker had already changed — so they
> report as `unhashed` rather than verified. The failure mode is a false
> accusation rather than a missed one, since a write path that forgets to
> rehash flags an untouched record; every path is covered by a DB-gated
> test verified to fail when a rehash is removed. **case adopted it on
> 2026-07-27**, closing the last gap among the four chain-carrying
> services — until then an out-of-band edit to a stored case was
> undetectable there. worker's
> `worker_assessments` sub-resource carries its own digest rather than
> being folded into the parent's — an assessment is written through its
> own endpoints, and a per-row hash names *which* assessment was tampered
> with instead of only that something about the worker changed.

4. **Copy the FHIR conformance machinery** to the services that already
   mount `/fhir` (organization, place, thing, person, worker, case, event).
5. **Lift the evidence artefacts** (`compliance/`, `scripts/`) to the
   repository root once a second crate needs them, and wire SBOM + `cargo
   deny` + the traceability check into CI.
</content>
</invoke>
