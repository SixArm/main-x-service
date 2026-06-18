## 3. Stakeholders and Users

| Stakeholder | Interest |
|---|---|
| Scheduling / EHR integrators | Stable REST surface for create / read / search |
| Operations / DBA | PostgreSQL schema + migration discipline |
| Compliance officer | Audit trail, GDPR export, consent records |
| Other Main X Index crates | Cross-references via `event_id` |

