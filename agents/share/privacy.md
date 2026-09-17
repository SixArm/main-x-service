### Privacy

- Data masking for sensitive fields (coordinates, telephone numbers, email contacts, postal addresses)
- Masked person view endpoint (GET /\*/{id}/masked)
- GDPR data export (GET /\*/{id}/export)
- Consent management model (DataProcessing, DataSharing, Marketing, Research)
- Consent status tracking (Active, Revoked, Expired)
- Active consent checking utility

### Fairness/bias slices by demographic attributes — computed by the caller, not the registry

Settled family-wide (repo `tasks.md` PA-5, from care-pathway's
[OQ-7](../../care-pathway/spec/16-open-questions.md)): a fairness or
bias slice ("who is breaching the standard, by age/sex/ethnicity/
deprivation") is **never computed inside an entity registry**. The
registries that carry demographic attributes (person) and the ones that
carry the outcome being sliced (e.g. care-pathway's [time-based
analysis](time-based-analysis.md)) are different services by design, and
joining them inside either one makes it a processor of data it never
collected for that purpose, with everything that implies for consent and
export governance.

The answer is instead: **the caller joins in its own secure processing
environment, over already-exported features** — care-pathway's
`journey_features` codec ([bulk-import-export.md
§5.1](bulk-import-export.md), PA-4) is exactly what such an export is
for. A registry may still offer `journey_features`/equivalent exports
(patient-level, gated as a disclosure, never as a read — §5.1), but it
does not itself perform the demographic join, the standardised-
mean-difference computation, or the case-mix adjustment. This closes the
question for every entity that might otherwise re-open it (`case`,
`patient-flow`) — the join-elsewhere answer is the family's, not a
per-service call.
