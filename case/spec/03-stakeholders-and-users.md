## 3. Stakeholders and Users

The case entity targets deployment inside a worldwide public
governmental system handling cases at population scale. Stakeholders
span the trio:

| Stakeholder | Interest | Primary surface |
|---|---|---|
| Government agencies / departments (benefits, courts, social services, licensing, tax, immigration, …) | One canonical registry of cases across systems; dedupe of the same matter held in multiple places | Service `/api/cases/*` |
| Caseworkers / registry operators | Day-to-day CRUD, duplicate review, curation, merging | Front-end (`/`, `/new`, `/[pid]`, `/[pid]/edit`) |
| Data subjects (the people / organisations the cases concern) | That their cases are not duplicated or confused; their data-subject rights (access, rectification, erasure) are honoured | Indirect — via GDPR export (roadmap §15) and operator action |
| Oversight bodies / regulators / ombudsmen | Who changed what, when; explainable match decisions; compliance evidence | Audit endpoints (`/audit/recent`, `/{pid}/audit`), soft-delete history |
| Integrators (court systems, EHR, benefits platforms, downstream case-handling systems) | Stable REST surface; linkage via deterministic identifiers (docket, external case id, URI) | Service `/api/*`, OpenAPI / Swagger |
| Auditors / information-governance officers | HIPAA/GDPR-grade audit trail; replayable match rationale; lawful-basis evidence | `audit_logs`, event stream, soft-delete timestamps |
| Operations / DBA / SRE | Schema + migration discipline, backups, health checks, scaling | Service deployment artefacts, `/_health`, `/_ping` |
| Developers / AI agents | Clear SDD contracts; which spec governs what | This spec + the three crate specs |
| Other Main X Index entities | Cross-references via `pid`; subjects resolve in the [person entity](../../person/) / [organization entity](../../organization/); handling agencies resolve in the organization entity | Service REST API |

Per-subproject stakeholder detail: service
[spec §3](../case-service-rust-crate/spec/index.md), matcher
[spec §3](../case-matcher-rust-crate/spec/index.md),
front-end
[spec §3](../case-front-end-with-svelte/spec/index.md).
