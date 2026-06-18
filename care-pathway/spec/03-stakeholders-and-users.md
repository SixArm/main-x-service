## 3. Stakeholders and Users

The care-pathway entity targets deployment inside a worldwide public
governmental health system. Stakeholders span the trio:

| Stakeholder | Interest | Primary surface |
|---|---|---|
| Health ministries / national agencies (e.g. NHS, national guideline bodies) | One canonical registry of published pathways across the system; dedupe of overlapping guidance | Service `/api/care-pathways/*` |
| Guideline publishers (e.g. NICE-style institutes) | Their pathways registered once, referenced by guideline-registry id / DOI / URI | Service create + identifiers |
| Hospital / trust pathway teams | Register local pathways (provider-scoped `pathway_code`); discover whether a national or sibling pathway already exists | Front-end routes + check-duplicates |
| Clinical informaticians (registry operators) | Day-to-day CRUD, duplicate review, curation | Front-end (`/`, `/new`, `/[pid]`, `/[pid]/edit`) |
| Clinicians (consumers) | Find the canonical pathway for a condition / care setting via downstream catalogues that resolve against this registry | Service read API (via integrators) |
| Integrators (EHR, CDS, pathway-execution platforms) | Stable REST surface; linkage via deterministic identifiers to FHIR `PlanDefinition` / BPM+ Health artefacts | Service `/api/*` |
| Auditors / information-governance officers | Who changed what, when; explainable match decisions; compliance evidence | Soft-delete timestamps today; audit endpoints on the roadmap (§15) |
| Operations / DBA / SRE | Schema + migration discipline, backups, health checks, scaling | Service deployment artefacts, `/_health` |
| Developers / AI agents | Clear SDD contracts; which spec governs what | This spec + the three crate specs |
| Other Main X Index entities | Cross-references via `pid`; provider organisations resolve in the [organization entity](../../organization/) | Service REST API |

Per-subproject stakeholder detail: service
[spec §3](../care-pathway-service-with-loco/spec/index.md), matcher
[spec §3](../care-pathway-matcher-rust-crate/spec/index.md),
front-end
[spec §3](../care-pathway-front-end-with-svelte/spec/index.md).
