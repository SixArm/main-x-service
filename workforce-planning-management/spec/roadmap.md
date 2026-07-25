# Roadmap

Beyond the v1 queue ([tasks.md](tasks.md)):

- **`employed_by` edge emission** — write the registry's
  cross-service link on hire/termination so the link-graph tracks
  employment automatically ([integrations.md](integrations.md)).
- **Payroll payment execution** — bank-file (BACS/SEPA) export from
  approved runs; real statutory tax tables per jurisdiction.
- **Vendor integrations** — e-signature, background-check, and
  job-board posting providers behind the onboarding/requisition
  checklists.
- **Timeclock hardware / geofenced mobile clock-in.**
- **Compensation reviews** — salary-change workflows with approval
  chains and benchmark-aware proposals.
- **People analytics** — attrition risk, absence patterns,
  time-to-fill; strictly additive over the v1 arithmetic dashboards.
- **Multi-org / group payroll** — one deployment serving a corporate
  group with consolidated reporting.
- **WPM ↔ PPM bridge** — allocations in
  project-portfolio-management referencing WPM employees for
  capacity-vs-contract checks.
- **Outbound notification delivery** — email/push over the upstream
  person service's contact details (in-app is v1; WPM-D23 keeps
  contact details out of WPM).
- **Adjustment review cadence** — periodic "is it still working?"
  check-ins on in-place adjustments (the lifecycle currently ends at
  `in_place`).
- **DSE re-assessment scheduling** — due-date driven re-assessment
  prompts (workstation moves, annual cycles).
- **360 → development-plan linking** — graft a shared report's
  competency gaps into WPM-R21 plan items.
