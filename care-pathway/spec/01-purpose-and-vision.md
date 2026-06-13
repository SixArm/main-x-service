## 1. Purpose and Vision

### 1.1 Purpose

The **care-pathway entity** is the clinical care-pathway registry of
the Main X Index — a federated identity index serving a worldwide
public governmental system with millions of users. A *care pathway*
(also "clinical pathway", "critical pathway", "integrated care
pathway") is a structured, evidence-based, multidisciplinary plan of
care for a specific clinical condition or patient group over a
defined episode. It is delivered as a trio of subprojects that
compose into one capability:

| Subproject | Role |
|---|---|
| [care-pathway-service-rust-crate](../care-pathway-service-rust-crate/) | Registry service — loco.rs CRUD + matching over REST; PostgreSQL persistence |
| [care-pathway-matcher-rust-crate](../care-pathway-matcher-rust-crate/) | Canonical pairwise matching library — deterministic + probabilistic, embedded by the service |
| [care-pathway-front-end-with-svelte](../care-pathway-front-end-with-svelte/) | Operator UI — SvelteKit SPA over the service's REST API |

The entity gives national health systems one canonical record per
published care pathway — NICE-style national guidelines, hospital and
trust pathways, integrated care pathways — regardless of how many
ministries, trusts, and guideline bodies hold a variant of that
pathway, deduplicated and matchable on the attributes that identify a
pathway (name, target condition codes, provider-scoped code, care
setting, document identifiers).

### 1.2 Vision

One canonical record per published care pathway, across national
health systems:

- **Registry of pathway identities.** The entity records *which*
  pathways exist and how they are identified — not the pathway
  content itself. Identifiers (DOI, Wikidata, guideline-registry ids,
  URIs) make it the linkage hub between guideline publishers,
  hospital pathway teams, and execution systems.
- **Explainable matching.** Every match decision returns a
  per-component score breakdown (name, condition codes, pathway
  code, care setting, interventions, keywords) that a clinical
  informatician or auditor can inspect — no black boxes.
- **Condition-code-centric.** Target condition codes (ICD-10 /
  ICD-11 / SNOMED CT) are the defining attribute of a pathway and
  carry the second-highest matching weight.
- **Multinational by design.** `in_language` on every record;
  operator surfaces localize to the locales in
  [`agents/share/locales.md`](../../agents/share/locales.md)
  (roadmap, §15).
- **Audit-grade.** Soft delete from day one; full audit logging and
  event streaming reach the family baseline on the roadmap (§15),
  suitable for NHS / HIPAA-grade information governance.

### 1.3 Non-goals

- **Not** a pathway *execution* engine — no BPMN / CMMN / DMN
  runtime, no CQL evaluation, no CDS Hooks integration. Execution
  belongs to EHR and decision-support platforms; this registry
  interoperates with them via identifiers (§17).
- **Not** a patient record system — it holds pathway *definitions*,
  never patient-level data. Person identity is the
  [person entity](../../person/).
- **Not** a FHIR `PlanDefinition` authoring or content store —
  pathway content (step timings, variance tracking, outcomes) stays
  in the publishing system; import/export of `PlanDefinition`
  identifiers is roadmap (§15).
- **Not** an authentication / authorisation provider. Sign-on for
  the whole index is the
  [authentication entity](../../authentication/) (passwordless
  magic-link, RS256 JWT + JWKS); this entity is a JWT *verifier*
  (roadmap, §15).
