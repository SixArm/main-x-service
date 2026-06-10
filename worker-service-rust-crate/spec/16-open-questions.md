## 16. Open Questions

- **OQ-1 — Credential validation.** Should we call out to an
  NPI / DEA / board licence registry to verify a credential at create
  time, or accept-and-flag for later verification?
- **OQ-2 — Cross-organisation merge.** Two workers with the same NPI
  registered under different `managing_organization` records — auto-
  merge, or always review-queue?
- **OQ-3 — Soft-delete vs deactivation.** A worker leaving an
  organisation is a different state from a duplicate being merged.
  Today both set `active = false`. Do we need a distinct
  `employment_status` field?

