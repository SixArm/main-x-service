## 17. References

Distilled from the entity's founding research notes (clinical-pathway
tooling, standards, and open-source landscape), plus the family's
standing references. The registry-of-identities posture relative to
this landscape is §8.5.

### 17.1 Standards and specifications

- [HL7 FHIR PlanDefinition](https://build.fhir.org/plandefinition.html)
  — FHIR resource for pathway / protocol / order-set templates; the
  primary interop target for import/export (roadmap §15).
- [Clinical Quality Language (CQL)](https://build.fhir.org/ig/HL7/cql/)
  — HL7 language for shareable clinical logic and reasoning rules;
  execution-side, linked here only via identifiers.
- [CDS Hooks](https://cds-hooks.org/) — HL7 specification for hooking
  decision support into an EHR with real-time recommendations;
  execution-side, out of scope (§1.3).
- **BPM+ Health** ([bpm-plus.org](https://www.bpm-plus.org/)) — the
  industry framework for unambiguous, machine-readable clinical
  pathways, combining three OMG standards. OMG and HL7 publish "Field
  Guides" with step-by-step instructions for applying them to
  real-world clinical workflows:
  - [BPMN](https://www.omg.org/spec/BPMN/) — Business Process Model
    and Notation; the *prescriptive* workflow (task order, events,
    logic).
  - [CMMN](https://www.omg.org/spec/CMMN/) — Case Management Model
    and Notation; *reactive* case-based management where the
    patient's course is unpredictable.
  - [DMN](https://www.omg.org/spec/DMN/) — Decision Model and
    Notation; clinical rules (dosage, diagnostic criteria) in
    clinician-verifiable decision tables.
- Code systems: [ICD](https://icd.who.int/) (WHO ICD-10 / ICD-11),
  [SNOMED CT](https://www.snomed.org/) — the condition-code systems
  in `CodeSystem` (§5.2).
- [schema.org/MedicalGuideline](https://schema.org/MedicalGuideline)
  — nearest schema.org type; `same_as` follows schema.org
  [`sameAs`](https://schema.org/sameAs) semantics.

### 17.2 Literature and clinical components

- [Clinical pathway modelling: a literature review](https://orca.cardiff.ac.uk/id/eprint/124977/13/Clinical%20pathway%20modelling%20a%20literature%20review.pdf)
  (Cardiff University) — survey of pathway-modelling approaches.
- [European Pathway Association — pathway facilitator tools](https://e-p-a.org/pathway-facilitator-tools/)
  — practitioner tooling catalogue.
- A robust pathway specification typically defines: entry/exit
  criteria (when a patient enters or leaves), timings and milestones
  (e.g. diagnostics within 24 h), escalation points (deviation
  protocols), and resource mapping (which roles / departments own
  each task). These are pathway *content* — out of registry scope
  (§1.3) but useful when judging whether two records are the same
  pathway.

### 17.3 Open-source tools (landscape)

Pathway building / modelling:

- **PathBuilder** — open-source annotation and development of
  pathway resources; manual entry + XML import.
- **CDS-Sandbox** — cloud VM for learning and testing clinical
  decision support; integrates FHIR + CQL artefact authoring.
- **Clinical AI Pathway Guide** — framework for moving clinical AI
  concepts from idea to clinical use.

Pathway execution / EHR integration:

- [OpenEMR](https://www.open-emr.org/) — open-source EHR with
  customizable workflows and CDS underpinning pathway delivery.
- [openEHR](https://openehr.org/) / **EHRServer** — clinical data
  repository with archetype-based standardized modelling.
- **clinical_pathway** (GitHub) — open-source clinical app for
  pathway-context note entry.

Analytics / pathway discovery (process mining):

- [bupaR](https://bupar.net/) — R process-mining suite used by NHS
  organisations on healthcare process data.
- [PM4Py](https://github.com/pm4py) — Python process-mining library;
  discovers pathways from historical records.
- **Defrag** — infers treatment pathways from complex,
  non-standardized health data (ScienceDirect).
- **opencodecounts** — R package + Shiny app exploring NHS clinical
  coding (SNOMED CT, ICD-10) over time.

Triaged for §13 T-14 (2026-09-03) — the code was read, not the README:

- [IPPA-py](https://github.com/PatientPathwayAnalysis/IPPA-py) —
  Individualised Patient Pathway Analysis (Python, Apache-2.0): timed
  state machines over claims visits, episode cutting, named anchors and
  delay decomposition. Ku CC et al., *BMJ Glob Health* 2020;5:e002187,
  [doi:10.1136/bmjgh-2019-002187](https://doi.org/10.1136/bmjgh-2019-002187).
  Companion pseudo-data release:
  [IPPA-data](https://github.com/PatientPathwayAnalysis/IPPA-data) (CC-BY-4.0).
- [process-mining-clinical-pathways](https://github.com/nhs-bnssg-analytics/process-mining-clinical-pathways)
  — NHS BNSSG + University of Bristol (R + T-SQL, no licence):
  SUS / e-RS / SWD rows → bupaR event log → variants, performance
  process maps, heuristics + inductive miners, PM4Py alignments, for
  elective hip replacement.
- [TreatmentPatterns](https://darwin-eu-dev.github.io/TreatmentPatterns/)
  — DARWIN EU / OHDSI (R, Apache-2.0, CRAN): OMOP cohorts → treatment
  eras → collapsed, combined, truncated pathway strings with strata,
  attrition, cell suppression, sunburst / Sankey. Markus A et al.,
  *Comput Methods Programs Biomed* 2022,
  [doi:10.1016/j.cmpb.2022.107081](https://doi.org/10.1016/j.cmpb.2022.107081).
- [ehrapy](https://github.com/theislab/ehrapy) — Theis lab (Python,
  Apache-2.0): scanpy-style exploratory EHR analysis on
  `EHRData(AnnData)` — QC + missingness, bias detection, Kaplan–Meier /
  Cox, clustering, CONSORT-style `CohortTracker`. Heumos L et al.,
  *Nat Med* 2024;30:3369–3380,
  [doi:10.1038/s41591-024-03214-0](https://doi.org/10.1038/s41591-024-03214-0).

Emerging AI tools:

- **CP-Env** (arXiv, 2025) — agentic hospital environment evaluating
  LLMs across end-to-end clinical pathways.
- **EHDViz** — toolkit for real-time clinical dashboards.

### 17.4 Family references

- Subproject specs: [service](../care-pathway-service-with-loco/spec/index.md),
  [matcher](../care-pathway-matcher-rust-crate/spec/index.md),
  [front-end](../care-pathway-front-end-with-svelte/spec/index.md).
- Entity AGENTS reference set: [`agents/index.md`](../agents/index.md).
- Shared docs: [`agents/share/index.md`](../../agents/share/index.md);
  sibling entity-level spec exemplar:
  [person/spec](../../person/spec/index.md).
- loco.rs: [loco.rs](https://loco.rs/) — the service framework.
