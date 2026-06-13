## 12. Compliance

The Main X Index targets worldwide public governmental systems, and
**case data is personal data**: a case concerns an identified or
identifiable person or organisation. Privacy and compliance therefore
matter *more* for this entity than for most siblings (a pathway or a
place definition is reference data; a case is about someone). Family
frameworks:
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
and
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
(the latter applies to healthcare / social-care cases).

### 12.1 Data classification

Case records are **personal data**, and some are **special-category**
(health, social-services, immigration, criminal-matter cases). Three
facets:

- **The record itself is personal data.** Title, case type, status,
  agency, opened date, and the involved `subjects` together identify a
  matter about a person — even though `subjects` are opaque references,
  re-identification is trivial for the holding agency. This is GDPR /
  UK DPA personal data; some cases engage GDPR Art. 9 special
  categories.
- **Audit trails are personal data too.** Who created, edited,
  reviewed, or merged a record is personal data about operators; the
  audit log (delivered) MUST be governed accordingly.
- **Free text can leak.** `keywords` and `alternate_titles` MUST never
  carry substantive case content or personal detail; `subjects` carry
  only opaque ids. This is a shared invariant (§5.5) and an
  operator-training point.

### 12.2 Frameworks

| Framework | Application to this entity |
|---|---|
| UK DPA 2018 / UK GDPR / EU GDPR | Fully engaged — case records are personal data. Lawful basis (usually public task / legal obligation), data-subject rights (access, rectification, erasure), and accountability documentation are mandatory. Soft delete supports retention policy; a GDPR-erasure path on top of it is required (§13 T-10). |
| US HIPAA | Engaged for healthcare / social-care cases that touch PHI; HIPAA-grade audit trails (delivered) and access controls required; soft delete preserves history. |
| UK Common Law Duty of Confidentiality | Engaged for cases holding confidential personal information (health, social care). |
| ISO/IEC 27001 | ISMS operational controls (deployment-side): access control, encryption at rest, backups, logging. |
| ISO/IEC 42001:2023 | AIMS controls if matcher weights/thresholds are ever ML-tuned (today they are hand-set constants). |

### 12.3 Information-governance posture

- **Minimisation by design.** `subjects` are opaque references, not
  personal detail; the registry holds identity/routing metadata, not
  the case file (§1.3). This shrinks but does **not** remove the
  personal-data footprint — the record is still about a person.
- **Privacy controls are a priority gap.** Per-field masking (for the
  masked-view endpoint) and GDPR data-subject export are **not yet
  built** and are higher-priority here than for any sibling — tracked
  as §13 T-10 and roadmap §15. Until they land, masking/export are an
  honest gap (§14), and deployments must mitigate operationally
  (access control, need-to-know, data-protection impact assessment).
- **Auditability.** Delivered: soft delete + durable `audit_logs` row
  per create/update/delete/merge + in-memory event stream, per
  [`agents/share/auditability.md`](../../agents/share/auditability.md).
  The remaining gap is a durable cross-replica event bus (roadmap §15).
- **Access control.** Production deployments MUST sit behind SSO
  (central authentication entity, RS256 JWT verification — delivered
  for `whoami` / `actor`; *blanket `/api/*` enforcement is roadmap*)
  and TLS; writes are restricted to caseworkers / registry operators.
- **Explainability for accountability.** Per-component match breakdowns
  give auditors a replayable rationale for every duplicate / merge
  decision — keep this property (NFR-9).
