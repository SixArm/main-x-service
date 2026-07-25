# Documentation index

Workforce Planning Management — Loco edition. JSON-only back-end API
for the employment lifecycle: hiring, workforce (incl. working-time
guardrails), HR core (incl. wellbeing, pulse, ergonomics,
adjustments, notifications, subject rights), talent development
(incl. 360°s and five assessment categories), payroll.

## Start here

- **[README.md](README.md)** — what this is, status, the surface, a
  worked curl tutorial, and the auth-activation pointer.
- **[../spec/](../spec/index.md)** — the cross-cutting specification
  (single source of truth: domain model, the five pillars, auth,
  audit, requirements WPM-R1–R33, design decisions WPM-D1–D25).
- **[spec/](spec/index.md)** — this edition's stack-specific spec
  (layout, env vars, masking/ownership/privacy mechanics, gotchas).
- **[AGENTS.md](AGENTS.md)** — working agreements for contributors
  (incl. the unrepresentability ground rule).
- **[CHANGELOG.md](CHANGELOG.md)** — Keep a Changelog format.
- **[config/abac-policy.reference.json](config/abac-policy.reference.json)**
  — the shipped, matrix-verified persona policy; runbook in
  [../spec/auth.md](../spec/auth.md).

## Running

```bash
cargo run -- db migrate && cargo run -- task seed && cargo run -- start
cargo test                          # 139 DB-free unit tests
cargo test -- --ignored             # 19 request suites (Postgres)
cargo test --test enforcement -- --ignored   # persona matrix
```

## The task queue

Live delivery checklist: [../spec/tasks.md](../spec/tasks.md)
(WPM-T1–T36 delivered; production gates WPM-G1/G2 `[~]` — code
complete, operational/legal work remains).
