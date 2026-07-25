## 12. Compliance

The entity targets a worldwide public governmental deployment, so the
technology-compliance baseline of
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
applies entity-wide, plus healthcare overlays
([`compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md))
when Events carry clinical context (`EncounterId` identifiers,
encounter-type Events).

### 12.1 Personal data in this entity

Event records look impersonal but are not: **attendee and party data
is personal data.** A `Party` carries a name, optionally an email,
and optionally an external ID into the person / worker /
organization entities. Identifier values (booking numbers,
confirmation codes, ticket numbers) often double as access tokens
and are treated as sensitive. An Event's time + place + attendee
list can reveal health, political, or religious information
(special-category data under GDPR Art. 9) — e.g. attendance at a
clinic encounter or a political hearing.

### 12.2 Mechanisms

| Standard | Mechanism |
|---|---|
| GDPR Art. 5 (minimisation) | Adapter drops party emails / attendee lists from matcher projection; matcher never scores `local_id`; no party data in logs (matcher forbids `Debug`-formatting records into traces) |
| GDPR Art. 9 (special category) | Masking on by default for public-facing reads; consent records per Event (`DataProcessing`, `DataSharing`, `Marketing`, `Research`) |
| GDPR Art. 15 (access) | `GET /api/events/{id}/export` |
| GDPR Art. 17 (erasure) | Soft delete + consent revocation; erasure-vs-audit-retention tension is EOQ-4 |
| GDPR Art. 30 (records of processing) | `audit_log` with old/new JSON, user ID, IP, user agent, timestamp |
| UK DPA 2018 | Same controls as GDPR rows above; UK-specific lawful-basis documentation is a deployment-side artefact |
| ISO/IEC 27001 | Operational ISMS controls (deployment-side): non-root containers, health checks, encryption at rest, CI security scans (`security.yml`) |
| ISO/IEC 42001 | AI-management scope: matching is deterministic and explainable (per-field breakdowns, FR-17, NFR-13) — no opaque model; any future ML-based scoring (service roadmap) MUST enter via an AIMS impact assessment |
| HIPAA (overlay, when PHI present) | Audit trail, access tracking, soft delete, encryption at rest |

### 12.3 Per-subproject responsibilities

- **Service** — enforcement point for masking, export, consent,
  audit. All compliance-relevant behaviour is specified in
  [service spec §12](../event-service-with-loco/spec/12-compliance.md).
- **Matcher** — by construction cannot leak: no IO, no logging, no
  network. Test fixtures use synthetic personal data only.
- **Front-end** — must not cache or persist personal data
  client-side; masked-view toggle and GDPR-export download are open
  UI tasks (front-end §13 T-19 / T-20). Until SSO lands (ET-5), the
  UI must be deployed only on trusted operator networks.

### 12.4 Extended frameworks

Four frameworks impose obligations beyond §12.2. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md).

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity, §164.528 accounting of disclosures | Engaged wherever an Event carries clinical context (`EncounterId` identifiers, encounter-type Events). §12.1's point sharpens it: **the mere fact of attendance can reveal health, political, or religious information**, so a *read* of an attendee list is itself a disclosure — the case §164.312(b) is written for. | **Read-auditing** on `get` / `list` / `search` / `export` / FHIR reads, with purpose-of-use and a disclosure flag distinguishing an internal access from an outward disclosure; and **tamper-evident history** (a SHA-256 chain over `audit_log`) so the Art. 30 record §12.2 already claims can prove it was not rewritten. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | Fully engaged, including Art. 9 special category (§12.1). This is the framework that resolves **EOQ-4** — the erasure-versus-audit-retention tension the spec currently leaves open. | An **erasure path that survives immutable history**: redact the row's content and stamp it, while preserving the hash linkage and the fact an event occurred. That is the answer to EOQ-4 — neither "delete the audit trail" nor "refuse erasure". Plus a declared **data residency** and **lawful basis**, and export beyond the region recorded as a **Ch. V transfer**. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | **Weak.** Event maps to `Appointment` (with `Encounter` on the roadmap), and neither is a headline US Core profile; the family's FHIR fidelity for this entity is already labelled best-effort. | The conformance *machinery* only: a declared `meta.profile`, structural validation, and **terminology validation** of `status` and participant codes against their bound value sets — plus `$validate`, SMART discovery, and Bulk Data `$export`. Conformance to a specific certification profile is **not** a goal here. |
| **IEC 62304 / SaMD** (with ISO 14971) | Not a device. The engagement is **supply-chain and configuration evidence** — worth having on engineering grounds and feeding the ISO/IEC 27001 controls §12.2 already claims. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability** over the window-overlap and identifier logic; and **signed, reproducible builds**. |

### 12.5 Honest limits

- **Not a certified health-IT module,** and for this entity not even a
  candidate: `Appointment` is a best-effort mapping
  ([`agents/share/fhir.md`](../../agents/share/fhir.md) §3), and the
  family serves **R5** against a certification that targets **R4 + US
  Core**.
- **Every extended control is unimplemented in this service today.** The
  reference implementation is the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/);
  event adopts the FHIR conformance machinery at step 4 of the rollout
  ([`spec/compliance` §8.5](../../spec/compliance/index.md)).
