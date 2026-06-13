## 3. Stakeholders and Users

The organization entity targets deployment inside a worldwide public
governmental system. Stakeholders span the trio:

| Stakeholder | Interest | Primary surface |
|---|---|---|
| Registry operators (data stewards) | Day-to-day CRUD, duplicate-check before create, record curation | Front-end routes (`/`, `/new`, `/[pid]`, `/[pid]/edit`) |
| Business / company registers | Authoritative source feeds; deterministic linkage via LEI / DUNS / GLN / VAT / tax ID | Service `/api/organizations` + identifier schemes (§5.2) |
| Government agencies / integrating departments | Stable REST surface to resolve, match, and reference organizations by `pid` | Service `/api/organizations/*` |
| Procurement / grants / licensing systems | Supplier and counterparty identity resolution; duplicate detection before onboarding | `/check-duplicates`, `/match`, `/search` |
| Data subjects (sole traders, named contacts) | Accuracy of records that are personal data; GDPR / UK DPA rights | Service privacy endpoints (deferred — §13, §12) |
| Auditors / regulators | Who changed what, when; explainable match decisions; compliance evidence | `/api/organizations/audit/*` + `audit_logs` table; per-component score breakdowns |
| Operations / DBA / SRE | Schema + migration discipline, backups, health checks, scaling | loco `/_health`, `/_ping`; `migration/` |
| Compliance / privacy officers | Soft-delete retention, audit trail; masking + export when the privacy layer lands | Service audit endpoints; §12 |
| Developers / AI agents | Clear SDD contracts; which spec governs what | This spec + the three subproject specs |
| Other Main X Index entities | Cross-references via organization `pid` (e.g. a person's managing organization, an event's organizer) | Service REST API |

Per-subproject stakeholder detail: service
[spec §3](../organization-service-rust-crate/spec/index.md), matcher
[spec §1–§2](../organization-matcher-rust-crate/spec/index.md),
front-end [spec §3](../organization-front-end-with-svelte/spec/index.md).
