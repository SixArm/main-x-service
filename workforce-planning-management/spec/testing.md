# Testing

- **Pure-core unit tests** (DB-free, exhaustive — 139 as of
  WPM-T36): every lifecycle's legal/illegal transition matrix
  (requisition, application, review, payroll run, leave, employee,
  mentorship, appraisal, adjustment, placement); leave-balance
  arithmetic; overtime derivation; shift conflicts; org-chart cycle
  refusal; payslip reconciliation + overflow refusal; benchmark
  flags; working-time boundary + rest-gap matrices; wellbeing
  eligibility (unknown-age-fails-banded pin) + prompt machine;
  pulse k-floor (count-withholding pin); 360 group floor +
  score-completeness; assessment category↔scale exhaustiveness
  (incl. the cognitive/selection refusal); DSE symptom-free
  checklist pin; erasable-status + retention-floor + sweep-list
  pins; notification fan-out.
- **Request tests** (Postgres, `#[ignore]`d,
  `cargo test -- --ignored` — 19 suites): the hire journey; leave
  balance journey incl. the decided race; time caps + overtime;
  shift conflicts; payroll derivation + approved-run immutability;
  benchmark flags; L&D, assessments, talent, wellbeing (×2), pulse,
  360 (with notifications + rater requests), ergonomics,
  adjustments, working-time, and subject-rights round-trips;
  unknown-pid 404s pinned throughout.
- **Enforcement binary** (own process — the OnceLock lesson): the
  persona matrix mounted on **the shipped reference policy file**
  (`config/abac-policy.reference.json`): 401/403 splits, payroll
  unmasked vs HR masked reads, `$sub` self-reads, payslip masking,
  destructive gating of delete / `/erase` / `/sweep`,
  subject-access masked-403, 360 report comment withholding, and
  svc-erase-of-active still refused.
- **Front-end**: vitest (10 — path map, `money()` honesty, 13-locale
  i18n parity over the full key set); Playwright (9 specs) over a
  `page.route`-stubbed API (contract-mirroring, unstubbed =
  404-loud).
- Seed task: a synthetic org (~40 employees, requisitions in every
  stage, a payroll run) — synthetic data only.
