# Pillar 5 — Payroll & compensation

## Payroll runs

A run covers one organization and period and is a strict state
machine: `draft → calculated → approved → paid`, each transition
audited with actor. **Calculate** produces one payslip per active
employee from pure-core arithmetic:

- **Gross** — the salary pro-rated to the period (FTE-aware), plus
  overtime minutes × the derived rate.
- **Deductions** — kinded lines (`tax`, `social`, `pension`,
  `other`); v1 rates come from configured stub tables (percentages),
  **not** jurisdiction-complete tax law (documented limitation) —
  pension lines derive from active benefit enrollments.
- **Net** — gross − Σ deductions, with the invariant enforced in
  code (a payslip that doesn't reconcile refuses to persist) and
  every amount an overflow-checked i64 in minor units.

Recalculation is allowed from `draft`/`calculated` only; `approved`
freezes the slips; `paid` is a record of fact, not a payment
execution (bank-file export is roadmap). Payslips are visible to the
payroll persona and to each employee (their own, via self-service);
every payslip read is audited.

## Salary benchmarking

Benchmarks record market min/median/max per job title (+ optional
location) in a currency, dated `as_of`. The comparison view joins
active employees to their title's benchmark and flags:

- `below_min` — retention risk, and
- `above_max` — cost outlier,

with the portfolio-style rollup per department. Benchmarks are data
the deployment loads (import or manual entry) — WPM does not scrape
market data.
