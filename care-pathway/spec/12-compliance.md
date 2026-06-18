## 12. Compliance

The Main X Index targets worldwide public governmental health
systems, so the full healthcare-compliance posture applies to this
entity. Frameworks:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
and
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md).

### 12.1 Data classification

Care-pathway records are **clinical artefacts, not patient data**: a
pathway definition (name, condition codes, interventions,
identifiers) contains no patient identifiers and is usually published
material. Two caveats keep the healthcare posture in force:

- **Audit trails and linked usage are personal data.** Who created,
  edited, reviewed, or merged a record — and, in downstream systems,
  which patients were placed on which pathway — is personal data
  about operators and patients respectively. The audit log (roadmap)
  MUST be treated as personal data under GDPR / UK DPA 2018.
- **Free-text fields can leak.** `keywords`, `interventions`, and
  `alternate_names` MUST never carry patient-level detail; this is a
  shared invariant (§5.5) and an operator-training point.

### 12.2 Frameworks

| Framework | Application to this entity |
|---|---|
| US HIPAA | Pathway definitions are not PHI; HIPAA-grade audit trails (who/what/when) still required for registry operations once audit logging lands (§15); soft delete preserves history |
| UK DPA 2018 / UK GDPR / EU GDPR | Applies to operator identities in audit data and to any personal data that strays into free text; lawful-basis and accountability documentation deployment-side; soft delete supports retention policy |
| UK Common Law Duty of Confidentiality | Engaged only if confidential patient information ever enters the registry — by design it MUST NOT (§5.5) |
| UK NHS Act 2006 s251 | Not engaged for pathway definitions (no confidential patient information); becomes relevant only for downstream linkage analyses performed outside this entity |
| ISO/IEC 27001 | ISMS operational controls (deployment-side): access control, backups, logging |
| ISO/IEC 42001:2023 | AIMS controls if matcher weights/thresholds are ever ML-tuned (today they are hand-set constants) |

### 12.3 Information-governance posture

- **Minimisation by design.** The domain model has no fields for
  patient data; there is nothing to mask, so the family's privacy
  controls (masking, GDPR export, consent) are deferred rather than
  required — re-assess the moment any restricted field appears
  (service spec §13).
- **Auditability.** MVP ships soft delete with timestamps; the
  compliance bar for production in a national health system is the
  full audit log + event stream of
  [`agents/share/auditability.md`](../../agents/share/auditability.md)
  — tracked as §13 T-3 and roadmap §15.
- **Access control.** Production deployments MUST sit behind SSO
  (central authentication entity, PASETO v4 public token verification
  per [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  — roadmap) and TLS; the registry is read-mostly and writes are
  restricted to registry operators.
- **Explainability for accountability.** Per-component match
  breakdowns give auditors a replayable rationale for every
  duplicate decision — keep this property (NFR-8).
