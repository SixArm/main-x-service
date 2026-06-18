## 3. Stakeholders and Users

The plan entity targets deployment inside a worldwide public
governmental system. Stakeholders span the trio:

| Stakeholder | Interest | Primary surface |
|---|---|---|
| Government departments / agencies (portfolio owners) | One canonical registry of initiatives across the system; dedupe of overlapping or duplicated programmes | Service `/api/plans/*` |
| Programme / portfolio managers (PMO) | Register portfolios and programmes; see parent/child hierarchy; spot duplicate or dependent initiatives | Front-end routes + check-duplicates + relationships |
| Project leads | Own a plan's goals, tasks, issues, posts; assign work; track timeline / burndown | Front-end sub-resource workspaces |
| Team members / contributors | Pick up tasks, raise issues, comment, post updates | Front-end sub-resource workspaces (membership-scoped) |
| Sponsoring organisations | Their initiatives registered once, linked to the owning org (`owner_org_id` → [organization entity](../../organization/)) | Service create + identifiers + cross-service links |
| Integrators (Jira / Asana / MS Project / Linear / GitHub Projects) | Register / sync a project via its external id; stable REST surface; deterministic-id linkage | Service `/api/*` + bulk import/export |
| Auditors / information-governance officers | Who changed what, when; explainable match decisions; who was on which plan | Audit endpoints + event stream + soft-delete timestamps |
| Operations / DBA / SRE | Schema + migration discipline, backups, health checks, scaling | Service deployment artefacts, `/_health` |
| Developers / AI agents | Clear SDD contracts; which spec governs what; the matchable/operational partition | This spec + the three crate specs |
| Other Main X Index entities | Cross-references via `EntityRef` (lead, assignees, authors, members → person / worker / authentication; sponsor → organization); cross-service links to any entity | Service REST API + the link aggregator |

Per-subproject stakeholder detail: service
[spec §3](../plan-service-with-loco/spec/index.md), matcher
[spec §3](../plan-matcher-rust-crate/spec/index.md), front-end
[spec §3](../plan-front-end-with-svelte/spec/index.md).
