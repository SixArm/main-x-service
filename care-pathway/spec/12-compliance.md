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

### 12.4 Extended frameworks

Four frameworks impose obligations beyond the §12.2 table, and this
entity is the family's **reference implementation** of all four (see
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2 for the regime detail and
[`spec/compliance` §8](../../spec/compliance/index.md) for the
repository-wide status). The service-side design and task breakdown
live in the service spec
[§12 / §13 T-11–T-14](../care-pathway-service-with-loco/spec/index.md).

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — audit controls §164.312(b), integrity §164.312(c), transmission security §164.312(e), accounting of disclosures §164.528 | Pathway *definitions* are not PHI, but **who consulted which pathway, when, and why** is system activity over a clinical artefact, and the **instance layer** (a named patient on a pathway) is squarely ePHI. | **Read-auditing** — an audit row for reads / searches / exports / FHIR fetches, carrying purpose-of-use and a disclosure flag — and **tamper-evident history**: a SHA-256 chain over `audit_logs` so the trail can prove it was not rewritten, plus a per-record accounting of disclosures. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | Fully engaged for audit-trail personal data (operator identities) and for the instance layer. EHDS's **primary vs. secondary use** split matters directly: pathway data is exactly the registry material its Ch. IV routes to research and policy use. | An **erasure path that survives the immutable chain** (redact the content, keep the linkage); a declared **data residency** and **lawful basis** stamped on every audit row; an `X-Purpose-Of-Use` marker separating care delivery from secondary use; and an export crossing the declared region recorded as a **transfer** event. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | Partial by construction: the service serves FHIR **R5** `PlanDefinition`, which has **no US Core profile**, so certification itself is out of reach (§12.5). The conformance machinery is not. | **Profile and terminology validation** — a declared `meta.profile`, must-support and cardinality checks, and condition codes validated against the value set their element is *bound* to (ICD-10 / ICD-11 / SNOMED CT) rather than merely being well-formed — plus `$validate`, SMART discovery, and Bulk Data `$export`. |
| **IEC 62304 / SaMD** (with ISO 14971, EU MDR Rule 11) | The template registry alone is not a device; the **instance layer crosses the line**, because tracking an individual patient's progress through a pathway informs care decisions. This entity therefore declares a safety classification and carries the evidence artefacts. | A **SOUP register + CycloneDX SBOM** from the real dependency graph; **machine-checked requirement→test traceability**; **signed, reproducible builds** with recorded provenance; and a runtime software-identification surface stating version, build, classification, and which controls are live. |

### 12.5 Honest limits

- **Not a certified health-IT module.** ONC certification targets FHIR
  **R4 + US Core**; this service is **R5** and `PlanDefinition` has no
  US Core profile. The service implements profile / terminology
  validation, `$validate`, SMART *discovery*, and Bulk Data because each
  is independently worth having — it does **not** claim certification,
  and the SMART discovery document advertises the deployment's actual
  authorisation server rather than implying the family's PASETO
  credential is SMART OAuth.
- **Not a registered medical device.** Declaring an IEC 62304 safety
  classification and shipping the lifecycle evidence is preparation, not
  conformity assessment. A deployment that surfaces pathway steps as
  clinical decision support must complete the qualification (MDCG
  2019-11) and the ISO 14971 risk file itself; those are organisational
  artefacts (§12.3, [`spec/compliance` §6.3](../../spec/compliance/index.md)).
- **Chain scope.** The hash chain proves the **audit trail** was not
  rewritten. It does not prove the `care_pathways` rows were not —
  row-level integrity hashing over the entity table is not built.
- **Read-auditing is default-off.** `CARE_PATHWAY_AUDIT_READS` defaults
  off so adopting the change is behaviour-neutral; a HIPAA-facing
  deployment MUST turn it on, alongside `CARE_PATHWAY_REQUIRE_AUTH`
  (§12.3, and the activation gate in
  [`agents/share/security.md`](../../agents/share/security.md) §4).
- **EHDS data permits and secure processing environments** are not
  built — they are an operating-organisation and infrastructure
  concern. The service contributes the purpose-of-use marking and the
  export audit a permit regime consumes.
