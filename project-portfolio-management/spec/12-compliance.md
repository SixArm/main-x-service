## 12. Compliance

The Main X Index targets worldwide public governmental systems.
Plans are **operational / business data** rather than clinical or
patient data, so the technology-compliance posture leads, with the
data-protection posture engaged by the **people references** a plan
carries. Frameworks:
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
and
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md).

### 12.1 Data classification

A plan's thin record (name, goals, code, optional `kind` label, parent
plan, timeframe, identifiers) is **business data**, not personal data.
Two angles keep the data-protection posture in force:

- **People references are personal data.** `lead_ref` and task
  `assignee_ref`s are `EntityRef`s to **people** (person / worker /
  authentication identities). Who leads or staffs a plan — and the
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
  the `case ↔ person`-style sensitivity does **not** apply (plans
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
  supersedes the RS256-JWT model) and TLS. **ABAC** authorisation over
  the token's `attrs` claim gates write vs read vs destructive actions
  (per
  [`agents/share/authorization-attributes.md`](../../agents/share/authorization-attributes.md);
  delivered, supersedes the earlier role-based sketch); default-off
  until the coordinated SSO activation (§15).
- **Explainability for accountability.** Per-component match
  breakdowns give auditors a replayable rationale for every duplicate
  decision; keep this property (NFR-8).

### 12.4 Extended frameworks

Four frameworks impose obligations beyond §12.2. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md). Consistent with
§12.1 — plans are business data, and there is no patient data anywhere in
the model — two of the four are **not engaged**, and the table says so
rather than manufacturing relevance.

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity | **Not engaged** — no PHI, consistent with §12.2. | Nothing HIPAA-specific. The **tamper-evident audit chain** (a SHA-256 chain over `audit_logs`) is still adopted on governmental-information-governance and ISO/IEC 27001 grounds — the same reasoning §12.2 already uses to keep an audit-grade trail without a HIPAA trigger. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | **GDPR engaged**, via the people references §12.1 names (`lead_ref`, task `assignee_ref`) and the operator identities in audit data. **EHDS not engaged** — no health data. | An **erasure path that survives immutable history** (redact content, keep chain linkage) — which here means erasing a person's assignment history without destroying the plan's own record; a declared **data residency** and **lawful basis**; and bulk export of people references beyond the region recorded as a **Ch. V transfer**, reinforcing §9.6's masked-by-default rule. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | **Not engaged.** Portfolio is explicitly out of FHIR scope ([`agents/share/fhir.md`](../../agents/share/fhir.md) §3) — no meaningful resource models a plan, and the service mounts no `/fhir` surface. | Nothing. Do **not** add a FHIR surface, a profile claim, or SMART discovery to this entity; there is nothing to conform to and advertising conformance would be misleading. |
| **IEC 62304 / SaMD** (with ISO 14971) | Not a device, and no clinical hazard. The engagement is **supply-chain and configuration evidence** on general engineering grounds. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability**, notably over the containment cycle check and the date arithmetic (SEC-M4's overflow panic is the archetype of what traceability catches); and **signed, reproducible builds** — all feeding the ISO/IEC 27001 configuration-management controls §12.2 claims. |

### 12.5 Honest limits

- **Two of the four do not apply, and that is the finding** — not an
  oversight to be filled in later. Recording "not engaged" with the
  reason is the useful output.
- **Read-auditing is optional here.** Without PHI, the §164.312(b)
  argument does not carry; audit reads only if a deployment's own
  governance rules require it.
- **Every extended control is unimplemented in this service today.** The
  reference implementation is the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/).
