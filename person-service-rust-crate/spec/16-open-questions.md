## 16. Open Questions

- **OQ-1 — FHIR Organization resource.** Do we expose `/fhir/Organization`
  here, or push it into a separate `organization-service` crate? Spec
  does not yet decide.
- **OQ-2 — Tax-ID short-circuit threshold.** Currently any TAX-type
  identifier match short-circuits to 1.0. Should we require both records
  to also share birth-year before the short-circuit fires, to avoid
  false positives on shared corporate tax IDs?
- **OQ-3 — Consent enforcement.** Should the query layer hide records
  lacking active `DataProcessing` consent, or surface them with a
  `consent_required: true` flag and leave the filtering to the caller?

Open questions resolve into §13 tasks or §5–§9 amendments when
decisions are made.

