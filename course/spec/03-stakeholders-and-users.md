## 3. Stakeholders and Users

The course entity targets deployment inside a worldwide public
governmental system — national education systems and public training
programmes. Stakeholders span the trio:

| Stakeholder | Interest | Primary surface |
|---|---|---|
| Education ministries / national agencies | One canonical course identity across school, university, and vocational catalogues; curriculum-feed ingestion with deduplication | Service `/api/*` |
| Accreditation bodies | Stable course identity to attach accreditation and credential shapes to; audit trail of catalogue changes | Service `/api/*`, audit endpoints |
| Course providers (universities, colleges, training organisations) | Their courses correctly identified, provider-scoped codes respected, instances per term tracked | Service `/api/*`; front-end |
| Registry operators (catalogue stewards) | Day-to-day CRUD, instance management, duplicate review, merge, audit lookup | Front-end routes (`/courses/*`) |
| Data integration engineers | Bulk import from LMS / OER repositories with deduplication; stable identifiers (DOI, Wikidata, LMS id) | Service `/api/*`, OpenAPI at `/swagger-ui` |
| Auditors / regulators | Who changed what, when; explainable match decisions; compliance evidence | Audit endpoints + audit-log table; match score breakdowns |
| Compliance / privacy officers | Masking of instructor / provider personal data, GDPR export, retention via soft delete | Service privacy endpoints |
| Operations / DBA / SRE | Migration discipline, backups, health checks, scaling | Service deployment artefacts, loco `/_health` |
| Developers / AI agents | Clear SDD contracts; which spec governs what | This spec + the three crate specs |
| Other Main X Index entities | Cross-references: instances may eventually reference [event](../../event/) records; providers may link to the [organization](../../organization/) registry (§16) | Service REST API |

Per-subproject stakeholder detail: service
[spec §3](../course-service-rust-crate/spec/03-stakeholders-and-users.md),
matcher
[spec §3](../course-matcher-rust-crate/spec/03-glossary.md) (the
matcher's §1–§25 shape folds users into §1–§2),
front-end
[spec §3](../course-front-end-with-svelte/spec/03-stakeholders-and-users.md).
