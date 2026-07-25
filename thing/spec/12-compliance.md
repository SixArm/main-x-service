## 12. Compliance

A public governmental deployment must satisfy the technology
compliance set in
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md):
EU GDPR, UK GDPR, UK Data Protection Act 2018, ISO/IEC 27001 (ISMS),
ISO/IEC 42001:2023 (AIMS).

**Things can carry personal data.** A Thing is "just an object", but
a thing linked to an individual — a personally-owned device, a
registered item with an `owner`, a serial number traceable to a
person — is personal data under GDPR / UK DPA. The entity therefore
ships the full privacy toolchain even though most records are
impersonal.

| Standard | Mechanism | Owner |
|---|---|---|
| GDPR Art. 15 (access / portability) | `GET /api/things/{id}/export` — full JSON export | service |
| GDPR Art. 17 (erasure) | Soft delete + consent revocation; hard-erasure policy is an operational control | service |
| GDPR / UK DPA minimisation | Per-field masking (`owner` withheld, identifier values truncated); masked view endpoint; optional masking in search results | service |
| GDPR lawful basis / consent | Consent model (`DataProcessing` / `DataSharing` / `Marketing` / `Research`; `Active` / `Revoked` / `Expired`) | service |
| UK DPA 2018 accountability | Full audit trail — old/new JSON, user ID, IP, user agent, timestamp on every mutation | service |
| ISO/IEC 27001 | Operational controls (deployment-side): non-root containers, env-based secrets, CORS, health checks, least-privilege DB access | service + ops |
| ISO/IEC 42001 | Match scoring is an automated decision aid: deterministic, explainable (per-field breakdown is mandatory, never discarded), tunable thresholds, human review queue for non-certain matches | matcher + service |
| No PII leakage | Matcher never logs record data (no-IO posture); front-end never `console.log`s Thing values; service logs exclude payload PII | all three |

### Front-end specifics

Soft-delete requires an explicit confirm; GDPR-export download UI and
masked-view toggle are deferred to roadmap (front-end §13 T-19 /
T-20). User attribution in the audit view becomes meaningful once SSO
is enforced (§8.3, §15).

### Healthcare note

The healthcare compliance set
([`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md))
applies to this entity only when a deployment registers
health-related things; the audit-trail design is already HIPAA-style.

### Extended frameworks

Four frameworks impose obligations beyond the table above. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md).

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity, §164.528 accounting of disclosures | **Conditional** — engaged where a Thing is a medical device or an owner-linked item, matching the "things can carry personal data" premise above. A device serial number traceable to a patient is ePHI-adjacent. | **Read-auditing** on `get` / `list` / `search` / `export` / FHIR reads with purpose-of-use and a disclosure flag, and **tamper-evident history** (a SHA-256 chain over `audit_log`) so the trail's integrity is provable — which also makes the audit view meaningful before SSO attribution lands. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | Engaged for owner-linked records; EHDS only indirectly (an implanted device linked to a patient is health data in the receiving system, not here). | An **erasure path that survives immutable history** (redact content, keep chain linkage) — a concrete answer to the "hard-erasure policy is an operational control" hand-wave above; a declared **data residency** and **lawful basis**; and export beyond the region recorded as a **Ch. V transfer**. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | Narrow: thing maps to **`Device`**, and US Core profiles only the *implantable* device case. A general asset registry falls outside that profile, so conformance applies to the subset, not the whole. | **Profile and terminology validation** for the device subset — `identifier` (UDI carrier and its components) and `type` validated against their bound value sets rather than merely non-blank — plus `$validate`, SMART discovery, and Bulk Data `$export`. |
| **IEC 62304 / SaMD** (with ISO 14971) | The registry is not a device — but it may **register** devices, and a mismatched identifier can misattribute a recall or a UDI. That misattribution is the hazard the evidence artefacts exist to control. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability**, notably over the identifier validators (DOI / ISBN / GTIN check digits), where a silent regression creates false identity; and **signed, reproducible builds**. |

### Honest limits

- **Not a certified health-IT module.** ONC certification targets FHIR
  **R4 + US Core**; the family serves **R5**, and only the implantable
  subset of `Device` is profiled at all.
- **Every extended control is unimplemented in this service today.** The
  reference implementation is the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/);
  thing adopts the FHIR conformance machinery at step 4 of the rollout
  ([`spec/compliance` §8.5](../../spec/compliance/index.md)).
