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
</content>
</invoke>
