## 3. Stakeholders and Users

| Stakeholder | Interest |
|---|---|
| API integrators | Stable REST + FHIR surface for person CRUD, match, merge |
| Operations / DBA | PostgreSQL schema + migration discipline; backups |
| Compliance officer | HIPAA audit trail, GDPR export, consent records |
| Other Main X Index crates | Cross-references via `person_id` |

