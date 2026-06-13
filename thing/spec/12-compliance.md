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
