# Regulatory posture

> ⚠️ **Demo software.** Patient Flow is a demonstration and
> integration exercise. It is **not** assured for clinical use, is
> not a medical device, holds no real patient data, and must not be
> deployed into live care without the production gates below.

## What this demo observes by design

- **Data minimisation** (UK GDPR / DPA 2018): Patient Flow stores
  URNs plus operational state, not demographics; the only cached
  personal field is a display name, and masking removes it from
  unauthorised views.
- **Purpose limitation**: flow data is used for flow; no analytics
  export of person-identifying data exists in v1.
- **Auditability**: every mutation and every sensitive read is
  audited ([audit.md](audit.md)) — the substrate for a Caldicott /
  IG audit.
- **Access control**: family SSO + ABAC with ward-level scoping and
  masking obligations ([auth.md](auth.md)).
- **Synthetic data only**: seeds and tests use synthetic people
  (person-service test fixtures); no NHS numbers of real people.

## Applicable regimes (production)

Per the family docs
([compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md)):
UK DPA 2018 + UK GDPR, the Common Law Duty of Confidentiality, and
NHS information-governance requirements (DSP Toolkit). Patient
location + infection status is special-category-adjacent data;
whiteboards visible to visitors are a specific, well-known IG risk —
hence masked corridor mode.

## Production gates (design-only here; tracked as PF-T-G* in [tasks.md](tasks.md))

1. **Clinical safety**: DCB0129/DCB0160 clinical risk management
   assessment (a bed board that is *wrong* is a hazard); a named
   Clinical Safety Officer.
2. **IG**: DPIA, DSP Toolkit submission, retention schedule for
   stays/audit, subject-access path (via person-service export +
   Patient Flow's per-person slice).
3. **Security activation**: `PATIENT_FLOW_REQUIRE_AUTH=on`, mounted
   ABAC policy, TLS everywhere, per family security.md §4 checklist.
4. **Resilience**: whiteboards are a 24/7 clinical operations
   surface — HA deployment, tested failover, and a defined paper
   fallback procedure.
