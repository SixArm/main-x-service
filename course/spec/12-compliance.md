## 12. Compliance

The entity targets governmental deployments and follows the
project-wide regimes in
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md)
and, where education touches health-adjacent confidentiality rules,
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md).

### 12.1 What is personal data here

Course-template data (name, code, syllabus, keywords) is mostly
**non-personal**. The personal-data surface is narrow but real:

- `CourseInstance.instructor_names` / `instructor_ids` — identifiable
  natural persons.
- `provider_id` and `Personal Data`-tagged identifier values — may be
  identifying in small-provider contexts.
- Audit-log user context (`user_id`, IP, agent) — operator personal
  data.

The masked view (`GET /api/courses/{id}/masked`, service FR-16)
clears / masks exactly these fields.

### 12.2 Regimes

| Regime | How the entity addresses it |
|---|---|
| **EU / UK GDPR, UK DPA 2018** | Right of access / portability via `GET /api/courses/{id}/export` (Article 15 envelope); erasure via soft delete + masking; accountability via the audit log recording who accessed and changed each record |
| **FERPA** (where instances carry instructor / student-adjacent data) | Masked view conceals personal identifiers; audit log preserves the access trail (service [spec §12](../course-service-with-loco/spec/12-compliance.md)) |
| **ISO/IEC 27001** | Audit trail, soft-delete retention, health checks, observability, and (roadmap) JWT-enforced access control supply ISMS evidence; full conformance is an operational/organisational programme, not a crate feature |
| **ISO/IEC 42001** | The matcher is deterministic and fully explainable (per-component breakdowns, no ML) — match decisions are reviewable by design; if ML scoring ever lands (explicitly out of scope today), it triggers AIMS review |
| **WCAG** | Front-end concern; Lily Headless primitives are the accessibility path (front-end [spec §12](../course-front-end-with-svelte/spec/12-compliance.md)) |

### 12.3 Gaps

- **Access control is not yet enforced** (service T-15, entity §13) —
  until JWT verification lands, GDPR/ISO 27001 claims about access
  restriction do not hold in deployment.
- Erasure is soft-delete + masking; whether a regulator-grade
  crypto-shredding or hard-purge path is needed for instructor data
  is undecided (§16 OQ-5 territory).

### 12.4 Extended frameworks

Four frameworks impose obligations beyond §12.2. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md). This is the
**least engaged** entity in the family — most of a course template is
non-personal reference data (§12.1) — and the table says so.

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(b) audit controls, §164.312(c) integrity | **Not engaged** — a course template holds no PHI. The one adjacency is a clinical-training deployment where instance enrolment identifies practitioners, and that is worker/person data held elsewhere. | Nothing HIPAA-specific. The **tamper-evident audit chain** is still worth adopting on ISO/IEC 27001 grounds, because the audit log is the evidence base §12.2 already relies on. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | GDPR engaged for the narrow personal-data surface §12.1 names (instructor identities, small-provider identifiers, operator audit context). **EHDS not engaged** — course data is not health data and does not enter its secondary-use pipeline. | An **erasure path that survives immutable history** (redact content, keep chain linkage) — the concrete answer to §12.3's open question about whether instructor data needs crypto-shredding or a hard purge: **neither**, redaction suffices; plus a declared **data residency** and **lawful basis**. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | **Not engaged.** No FHIR R5 resource models an educational course; the service wraps one in `Basic`, which is explicitly non-standard ([`agents/share/fhir.md`](../../agents/share/fhir.md) §3). There is no profile to conform to. | Only the generic half of the machinery: structural validation and `$validate` for well-formedness. **Do not** advertise a US Core profile or SMART conformance here — claiming either would be misleading. |
| **IEC 62304 / SaMD** (with ISO 14971) | Not a device, and no clinical hazard even indirectly. The engagement is **supply-chain and configuration evidence** on general engineering grounds. | A **SOUP register + CycloneDX SBOM**; **machine-checked requirement→test traceability**; and **signed, reproducible builds** — all feeding the ISO/IEC 27001 configuration-management controls §12.2 claims. |

### 12.5 Honest limits

- **The `Basic` wrapper is non-standard and stays labelled as such.** It
  exists so the entity has *a* FHIR surface, not because a course is a
  FHIR concept.
- **The access-control gap in §12.3 dominates.** Read-auditing and chain
  integrity add little while every endpoint is open; enforcement (service
  T-15) is the higher-value item and should land first.
- **Every extended control is unimplemented in this service today.** The
  reference implementation is the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/).
