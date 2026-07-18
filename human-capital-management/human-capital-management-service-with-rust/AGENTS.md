# AGENTS.md — working agreements

A pocket guide for human and AI collaborators working in this
subproject. Read this **before** opening a PR.

## What this project is

A **back-end JSON API**, written in Rust on [Loco](https://loco.rs)
(Axum + SeaORM + PostgreSQL), for human capital management: employee
records and the employment lifecycle, the applicant-tracking
pipeline and onboarding checklists, time & attendance, leave, shift
scheduling, benefits, performance reviews, training enrollments,
succession plans, payroll runs with payslips, and salary
benchmarking. There is no built-in UI — the
[Svelte sibling](../human-capital-management-front-end-with-svelte/)
is the HR / manager / self-service client.

**Domain ownership.** HCM **owns the employment relationship and its
operational state** (its own tables) but **references identities**:
humans are person-service records, professional identities
worker-service, employers organization-service, training courses
course-service — always as EntityRef URNs (`person:<uuid>`), never
duplicated demographics. See the cross-cutting spec's
[scope boundary](../spec/scope.md).

> ⚠️ Demo software, not a production HR/payroll system. Synthetic
> data only. See [regulatory](../spec/regulatory.md).

## Ground rules

1. **Spec first.** The cross-cutting spec at
   [`../spec/`](../spec/index.md) is the single source of truth;
   this subproject's `spec/` adds stack detail only. A behavioural
   change is spec edit + code + tests in one PR. The live task queue
   is [`../spec/tasks.md`](../spec/tasks.md) (HCM-T* ids, traced to
   HCM-D*/HCM-R*).
2. **Family conventions.** Loco-idiomatic layout
   (`src/controllers/`), `#![forbid(unsafe_code)]`, thiserror,
   tracing + OTLP, OpenAPI/Swagger, header API versioning
   (`Accepts-version`), Podman not Docker, PostgreSQL not SQLite,
   `bg_pg` jobs, in-memory loco cache. See
   [rust-loco-stack](../../agents/share/rust-loco-stack.md).
3. **Pure core.** Every lifecycle state machine (requisition,
   application, employee status, leave, review, payroll run), the
   leave-balance and overtime arithmetic, shift-conflict checks, the
   org-chart cycle check, and payslip derivation live in DB-free
   `src/rules/` modules with exhaustive unit tests; controllers only
   wire them ([design](../spec/design.md) HCM-D3–D5).
4. **Money discipline.** Minor units (`i64`) + ISO-4217 everywhere;
   overflow refused; `net = gross − Σ deductions` enforced before
   persist. No floats.
5. **Sensitive data.** Salary, payslips, review content, and
   succession plans are masked under the ABAC `mask` obligation and
   their reads are **audited** ([auth](../spec/auth.md),
   [audit](../spec/audit.md)). Never log them.
6. **Known family gotchas.** loco `create_table` pluralizes table
   names (use already-plural names / explicit SQL);
   `ModelError::EntityNotFound` is NOT mapped to 404 (return
   `Error::NotFound` at `find_by_pid` call sites); enforcement tests
   need their own test binary (OnceLock caching).

## Running

```bash
cargo run -- db migrate && cargo run -- task seed && cargo run -- start
cargo test                    # DB-free unit tests
cargo test -- --ignored       # request tests (needs Postgres)
```
