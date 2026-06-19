## 12. Compliance

The Main X Index targets worldwide public governmental systems.
Work items are **operational / business data** rather than clinical or
patient data, so the technology-compliance posture leads, with the
data-protection posture engaged by the **people references** a work
item carries. Frameworks:
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
and
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md).

### 12.1 Data classification

A work item's thin record (name, goals, code, kind, parent portfolio,
timeframe, identifiers) is **business data**, not personal data. Two
angles keep the data-protection posture in force:

- **People references are personal data.** `lead_ref` and task
  `assignee_ref`s are `EntityRef`s to **people** (person / worker /
  authentication identities). Who leads or staffs a work item — and the
  audit trail of who changed what — is personal data about identifiable
  individuals. **GDPR / UK DPA 2018 apply** to these references and to
  the audit log.
- **Free-text fields can leak.** `keywords`, `tags`, goal titles, and
  issue / task text MUST never carry special-category or unnecessary
  personal data; this is an operator-training point and is why export
  defaults to masked (§9.6).

There is **no patient data** anywhere in the model; the NHS / HIPAA
clinical posture does not lead, but the audit-grade trail it implies
is retained for governmental information governance.

### 12.2 Frameworks

| Framework | Application to this entity |
|---|---|
| UK DPA 2018 / UK GDPR / EU GDPR | Apply to the people references (`lead_ref`, task assignees) and operator identities in audit data; lawful-basis and accountability documentation deployment-side; soft delete supports retention policy; bulk export of these references is a compliance event (§9.6) |
| US HIPAA | Not engaged — no PHI; HIPAA-grade audit trails (who/what/when) are still the bar for registry + sub-resource operations as a governance baseline |
| UK NHS Act 2006 s251 / Common Law Duty of Confidentiality | Not engaged — no confidential patient information by design |
| ISO/IEC 27001 | ISMS operational controls (deployment-side): access control, backups, logging |
| ISO/IEC 42001:2023 | AIMS controls if matcher weights/thresholds are ever ML-tuned (today they are hand-set constants, §7 NFR-8) |

### 12.3 Information-governance posture

- **People-reference minimisation.** The model stores `EntityRef`s
  (opaque URNs), not duplicated personal detail; the personal data
  itself lives in the person / worker / authentication entities. This
  keeps the portfolio entity a *referrer*, not a second copy of
  personal data — re-assess masking the moment any denormalised
  personal field appears.
- **Masking on export.** Bulk export defaults to masked, with the
  people references revealed only under elevated authorisation (§9.6);
  the `case ↔ person`-style sensitivity does **not** apply (work items
  are not case data), but lead / assignee references are still personal
  data and masked by default.
- **Auditability.** Full audit log + event stream of every CRUD **and
  sub-resource write** per
  [`agents/share/auditability.md`](../../agents/share/auditability.md)
  — including who was assigned what — is the production bar; the
  durable event bus is roadmap (§15).
- **Access control.** Production deployments MUST sit behind SSO
  (central authentication entity, PASETO v4 public token verification;
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md),
  supersedes the RS256-JWT model) and TLS (role-based write
  authorisation over a work item's sub-resources is roadmap once auth
  is enforced, §15).
- **Explainability for accountability.** Per-component match
  breakdowns — and the kind gate — give auditors a replayable rationale
  for every duplicate decision; keep this property (NFR-8).
