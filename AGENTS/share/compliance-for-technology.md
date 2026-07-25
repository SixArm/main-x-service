# Compliance for technology

- United Kingdom (UK) Data Protection Act (DPA) 2018
- United Kingdom (UK) General Data Protection Regulation (GDPR)
- European Union (EU) General Data Protection Regulation (GDPR)
- ISO/IEC 27001 International Standard for Information Security Management Systems (ISMS)
- ISO/IEC 42001:2023 International Standard for Artificial Intelligence Management System (AIMS)

See [security.md](security.md) for the technical controls behind these
standards: the cross-cutting security invariants, the
`<ENTITY>_REQUIRE_AUTH` activation gate, secret-handling rules, and the
threat model.

## Frameworks that reach beyond healthcare

Two of the four control-driving frameworks in
[compliance-for-healthcare.md](compliance-for-healthcare.md) §2 apply to
**every** crate in the family, clinical or not, and are summarised there
rather than duplicated here:

- **GDPR / UK DPA 2018** — the erasure-versus-immutable-history collision,
  declared data residency, recorded lawful basis, and the cross-border
  transfer posture (the **EU EHDS** half is health-specific and does not
  engage the non-clinical registries). See §2.2.
- **IEC 62304 / SaMD supply-chain evidence** — the SOUP register + SBOM,
  machine-checked requirement→test traceability, and signed reproducible
  builds. The *device* framing engages only where a deployment crosses into
  clinical decision support (§2.4's qualification caveat), but the
  **evidence artefacts** are repository-wide engineering practice and feed
  ISO/IEC 27001 configuration-management and supply-chain controls
  directly.

The other two (**HIPAA** §2.1, **ONC / HTI certification** §2.3) engage
only where an entity carries clinical or patient-linked data, or exposes a
FHIR surface.
