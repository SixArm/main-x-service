# Testing

- **Pure-core unit tests** (DB-free, exhaustive): every pipeline's
  legal/illegal transition matrix (requisition, application, review,
  payroll run, leave, employee lifecycle); leave-balance arithmetic
  (annual refusal vs sick negative-flag); overtime derivation;
  shift double-booking + leave-conflict; org-chart cycle refusal;
  payslip reconciliation + overflow refusal; benchmark flags.
- **Request tests** (Postgres, `#[ignore]`d, `cargo test --
  --ignored`): the hire journey end-to-end (requisition → apply →
  interview → offer → hire → onboarding → activate); leave
  request/approve with balance change; payroll draft → calculate →
  approve with payslip totals; the approval race (two concurrent
  approvals ⇒ one wins); unknown-pid 404s (the family
  EntityNotFound lesson — pinned from day one).
- **Enforcement binary** (own process — the OnceLock lesson): the
  persona matrix (employee self-read vs other-read, manager team
  scope, salary masking under the `mask` obligation, payroll-only
  payslip access).
- **Front-end**: vitest for the API client path map + money
  formatting + core components; Playwright over a `page.route`-stubbed
  API (contract-mirroring, unstubbed = 404-loud).
- Seed task: a synthetic org (~40 employees, requisitions in every
  stage, a payroll run) — synthetic data only.
