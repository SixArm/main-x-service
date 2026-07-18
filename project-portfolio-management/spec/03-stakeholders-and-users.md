## 3. Stakeholders and Users

The portfolio entity targets deployment inside a worldwide public
governmental system. Stakeholders span the trio:

| Stakeholder | Interest | Primary surface |
|---|---|---|
| Government departments / agencies (portfolio owners) | One canonical registry of portfolios and the projects / products / programs under them; dedupe of overlapping or duplicated initiatives | Service `/api/portfolios/*` + child collections |
| Programme / portfolio managers (PMO) | Register portfolios and roll up their child projects / products / programs; see parent/child hierarchy; spot duplicate or dependent initiatives | Front-end routes + check-duplicates + `portfolio_ref` roll-up |
| Project / product / program leads | Own a work item's goals, tasks, issues; assign work; track timeline / burndown | Front-end sub-resource workspaces |
| Team members / contributors | Pick up tasks, raise issues, track goals | Front-end sub-resource workspaces |
| Sponsoring organisations | Their work items registered once, linked to the owning org (`owner_org_id` → [organization entity](../../organization/)) | Service create + identifiers + cross-service links |
| Integrators (Jira / Asana / MS Project / Linear / GitHub Projects) | Register / sync a project via its external id; stable REST surface; deterministic-id linkage | Service `/api/*` + bulk import/export |
| Auditors / information-governance officers | Who changed what, when; explainable match decisions; who was on which work item | Audit endpoints + event stream + soft-delete timestamps |
| Operations / DBA / SRE | Schema + migration discipline, backups, health checks, scaling | Service deployment artefacts, `/_health` |
| Developers / AI agents | Clear SDD contracts; which spec governs what; the four-kind model + the matchable/operational partition | This spec + the three crate specs |
| Other Main X Index entities | Cross-references via `EntityRef` (lead, assignees, members → person / worker / authentication; sponsor → organization); cross-service links to any entity | Service REST API + the link aggregator |

Per-subproject stakeholder detail: service
[spec §3](../project-portfolio-management-service-with-loco/spec/index.md), matcher
[spec §3](../project-portfolio-management-matcher-rust-crate/spec/index.md), front-end
[spec §3](../project-portfolio-management-front-end-with-svelte/spec/index.md).
