# Regulatory posture

> ⚠️ **Demo software.** Not a production HR/payroll system; no real
> personal data; statutory calculations are illustrative stubs.

## Observed by design

- **Data minimisation** — identities are URNs; HCM stores employment
  facts, not demographics; display names are refreshable caches.
- **Purpose limitation & consent** — the candidate pool is
  consent-bounded (`consent_until`); expired candidates leave search
  and are flagged for purge.
- **Access control** — ABAC personas + salary/review masking; the
  activation gate must be on before any real exposure.
- **Auditability** — mutations and sensitive reads audited (the
  substrate for GDPR accountability and payroll audit).
- **Synthetic data only** in seeds and tests.

## Production would additionally require

- UK GDPR / DPA 2018 lawful-basis mapping per record class;
  statutory **retention schedules** (payroll and right-to-work
  records carry multi-year duties; candidate data the opposite);
  subject-access and erasure flows coordinated with the identity
  services; jurisdiction-correct payroll/tax engines; equality-law
  review of any scoring (application screening, succession
  readiness); works-council/union consultation where applicable.
  Tracked as production gates in [tasks.md](tasks.md).
