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
