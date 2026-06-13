## 3. Stakeholders and Users

The person entity targets deployment inside a worldwide public
governmental system. Stakeholders span the trio:

| Stakeholder | Interest | Primary surface |
|---|---|---|
| Registry operators (data stewards) | Day-to-day CRUD, duplicate review, merge, audit lookup | Front-end routes (`/persons/*`) |
| Government agencies / integrating departments | Stable REST + FHIR surface to resolve, match, and reference persons by `person_id` | Service `/api/*`, `/fhir/Person` |
| Data subjects (citizens / residents) | Accuracy of their record; GDPR / UK DPA rights — access, export, erasure, consent | Service privacy endpoints (via agency channels) |
| Auditors / regulators | Who changed what, when, and why; explainable match decisions; compliance evidence | Audit endpoints + audit-log table; match score breakdowns |
| Operations / DBA / SRE | Schema + migration discipline, backups, health checks, scaling | Service deployment artefacts |
| Compliance / privacy officers | Masking, consent records, export, retention via soft delete | Service privacy + consent model |
| Developers / AI agents | Clear SDD contracts; which spec governs what | This spec + the three crate specs |
| Other Main X Index entities | Cross-references via `person_id`; the worker entity refines persons into workforce records | Service REST API |

Per-subproject stakeholder detail: service
[spec §3](../person-service-rust-crate/spec/03-stakeholders-and-users.md),
matcher
[spec §3](../person-matcher-rust-crate/spec/03-stakeholders-and-users.md),
front-end
[spec §3](../person-front-end-with-svelte/spec/03-stakeholders-and-users.md).
