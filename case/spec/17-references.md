## 17. References

Distilled from the entity's founding research notes (governmental
case-management tooling, standards, and open-source landscape), plus
the family's standing references. The registry-of-identities posture
relative to this landscape is §8.

### 17.1 Standards and specifications

- [schema.org](https://schema.org/) — the `same_as` field follows
  schema.org [`sameAs`](https://schema.org/sameAs) semantics; there is
  no single schema.org type for a government case, so the `Case` DTO is
  bespoke (§5).
- **CMMN** ([OMG Case Management Model and Notation](https://www.omg.org/spec/CMMN/))
  — the OMG standard for *case management* (reactive, knowledge-worker-
  driven work whose course is unpredictable). Execution-side; this
  registry links to such systems via identifiers, it does not run them
  (§1.3).
- **BPMN** ([OMG Business Process Model and Notation](https://www.omg.org/spec/BPMN/))
  / **DMN** ([Decision Model and Notation](https://www.omg.org/spec/DMN/))
  — workflow and decision standards used by agency case-handling
  systems; out of registry scope.
- [NIEM](https://niem.gov/) — the US National Information Exchange
  Model, with a Justice domain (court / case data exchange); a natural
  mapping target for `Docket` / `ExternalCaseId` identifiers (roadmap).
- [OASIS LegalXML / Electronic Court Filing (ECF)](https://www.oasis-open.org/committees/legalxml-courtfiling/)
  — court-filing message standards; docket numbers map to the `Docket`
  identifier scheme.
- Code / classification systems are jurisdiction-specific (benefit
  codes, court case-type codes, immigration matter types); the `Custom`
  escape hatches on `CaseType` / `CaseStatus` / `IdentifierScheme` carry
  them until namespaced taxonomies are designed (OQ-6).

### 17.2 Domain background

- Government case management spans many sectors — benefits, courts,
  social services, health and social care, housing, immigration,
  licensing, complaints, appeals, investigations, tax, employment — yet
  the *identity* of a case is consistent across them: a title, a
  handling agency, an agency-local case number, a type, a status, and
  the subject(s) it concerns. This entity models that shared identity,
  not the sector-specific case file.
- A robust case record in a source system typically also holds: parties
  and roles, documents and correspondence, decisions and outcomes,
  deadlines / SLAs, and a full activity history. These are case
  *content* — out of registry scope (§1.3) but useful when judging
  whether two records are the same case.

### 17.3 Open-source / landscape

- Open-source case-management platforms (e.g. court / social-services
  case systems, ECM workflow engines built on CMMN) are the *execution*
  systems this registry interoperates with via identifiers — it
  deduplicates and links the cases they hold, rather than competing with
  them.

### 17.4 Family references

- Subproject specs: [service](../case-service-with-loco/spec/index.md),
  [matcher](../case-matcher-rust-crate/spec/index.md),
  [front-end](../case-front-end-with-svelte/spec/index.md).
- Entity AGENTS reference set: [`agents/index.md`](../agents/index.md).
- Shared docs: [`agents/share/index.md`](../../agents/share/index.md);
  sibling entity-level spec exemplars:
  [person/spec](../../person/spec/index.md),
  [care-pathway/spec](../../care-pathway/spec/index.md).
- loco.rs: [loco.rs](https://loco.rs/) — the service framework.
