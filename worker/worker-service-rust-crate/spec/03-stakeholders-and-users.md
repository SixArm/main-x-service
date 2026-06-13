## 3. Stakeholders and Users

| Stakeholder | Interest |
|---|---|
| HR / credentialing officers | Authoritative worker record + credential history |
| API integrators | Stable REST + FHIR surface for worker CRUD, match, merge |
| Operations / DBA | PostgreSQL schema + migration discipline; backups |
| Compliance officer | HIPAA audit trail, GDPR export, consent records |
| Other Main X Index crates | Cross-references via `worker_id` |

