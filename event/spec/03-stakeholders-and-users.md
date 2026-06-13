## 3. Stakeholders and Users

The entity serves a worldwide public governmental deployment with
millions of users; stakeholders are correspondingly broad.

| Stakeholder | Interest | Primary touchpoint |
|---|---|---|
| Registry operators | Day-to-day CRUD, duplicate review, merging, audit lookups | event-front-end-with-svelte |
| Event-publishing agencies | Register hearings, consultations, civic programmes; keep one canonical record per occurrence | Service REST API |
| Scheduling / EHR / CRM integrators | Stable REST surface for create / read / search / match; stable cross-system event ID | Service REST API, OpenAPI |
| The public (data consumers) | Accurate, locale-aware, privacy-masked event data via downstream portals | Service search + masked views |
| Auditors / regulators | Complete who/what/when trail; explainable match decisions; GDPR / UK DPA evidence | Audit endpoints, score breakdowns |
| Compliance officers | GDPR export, consent records, soft delete, masking of party data | Service privacy layer |
| Operations / DBA | PostgreSQL schema + migration discipline; availability and scaling | Service persistence + deployment |
| Other Main X Index entities | Cross-references via `event_id`; `Party.id` into person / worker / organization; `Place.id` into place | Service models + REST API |
| Developers / AI agents | Spec-driven workflow, per-subproject AGENTS docs, three-part PRs | This spec, [`AGENTS/`](../AGENTS/) |

### 3.1 Privacy posture

Attendee and party records (`Party` with name, optional email,
optional external person ID) are **personal data** under GDPR / UK
DPA. The public consumes event data only through masked or
aggregated views; raw party emails and identifier values (which often
double as access tokens) are masked by default in exports intended
for the public. See [§12 Compliance](12-compliance.md).
