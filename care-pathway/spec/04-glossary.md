## 4. Glossary

Entity-level terms. Per-subproject vocabularies: service
[spec §4](../care-pathway-service-with-loco/spec/index.md), matcher
[spec §3](../care-pathway-matcher-rust-crate/spec/index.md),
front-end
[spec §4](../care-pathway-front-end-with-svelte/spec/index.md).

| Term | Meaning |
|---|---|
| **Entity** | One domain concept (here: Care Pathway) delivered as a trio of subprojects in one directory |
| **Trio** | The three subprojects: service crate, matcher crate, front-end project |
| **Entity-level spec** | This document set — source of truth for the cross-subproject contract |
| **Crate spec** | A subproject's own `spec/` — source of truth for that subproject's internals |
| **Clinical pathway / care pathway** | A structured, evidence-based, multidisciplinary plan of care for a specific clinical condition or patient group over a defined episode |
| **Integrated care pathway** | A care pathway spanning organisational boundaries (primary / secondary / community care) — same record shape here |
| **Guideline** | Published clinical guidance (e.g. from a NICE-style institute) from which pathways are derived; referenced via `GuidelineId` identifiers |
| **Condition code** | ICD-10 / ICD-11 / SNOMED CT code of the pathway's target clinical condition — the defining attribute of a pathway |
| **Care setting** | Where the pathway applies: `Inpatient`, `Outpatient`, `PrimaryCare`, `EmergencyDepartment`, `Community`, `HomeCare`, `Rehabilitation`, `MentalHealth`, `Palliative`, `Custom` |
| **Intervention** | A key treatment / action named by the pathway (free text; Jaccard-compared in matching) |
| **Provider-scoped code** | `pathway_code` (e.g. `STROKE-01`) — unique only within the issuing `provider_id`; never matched across providers |
| **Deterministic identifier** | Globally unique identifier (DOI, Wikidata, guideline-registry id, URI, UUID); a shared value pins the match score to 1.0 |
| **pid** | The public UUID of a stored pathway record (route param; distinct from the row's internal `id`) |
| **`data`** | The `care_pathways.data` JSONB column holding the full `CarePathway` payload verbatim |
| **DTO contract** | The API body **is** `care_pathway_matcher::CarePathway` — no separate service model, no adapter |
| **Match** | A comparison between two pathways yielding a 0.00–1.00 score, `Confidence` band, `is_match`, and per-component breakdown |
| **Check-duplicates** | `POST …/check-duplicates` — match a query against stored pathways, return ranked hits above threshold |
| **Soft delete** | Retention with `deleted_at` set; never `DELETE FROM` |
| **PlanDefinition** | HL7 FHIR resource representing a pathway / protocol template — the interop target for import/export (roadmap, §15) |
| **CQL** | Clinical Quality Language — HL7 language for shareable clinical logic; out of execution scope, in linkage scope |
| **CDS Hooks** | HL7 specification for hooking decision support into an EHR — execution-side, out of scope (§1.3) |
| **BPM+ Health** | OMG/industry framework for machine-readable pathways using BPMN (workflow), CMMN (case management), DMN (decision rules) — modelling-side, out of scope (§1.3) |
| **SSO** | Single sign-on via the [authentication entity](../../authentication/): magic-link, cookie session + PASETO v4 public cross-service tokens (see [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md), supersedes RS256 JWT + JWKS) |
| **Drift policy** | Front-ends keep per-project copies of types/client/forms; no shared package (repo decision 2026-06-02) |
