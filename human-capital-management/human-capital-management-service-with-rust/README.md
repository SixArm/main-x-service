# Human Capital Management — Loco JSON API

A back-end **JSON API** for human capital management across the full
employee lifecycle: requisitions and applicant tracking, onboarding,
employee records and org charts, time & attendance, leave, shift
scheduling, benefits, performance reviews, training, succession
planning, payroll runs with payslips, and salary benchmarking.
Implemented in Rust on [Loco](https://loco.rs) (Axum + SeaORM +
PostgreSQL). No built-in UI — the
[Svelte sibling](../human-capital-management-front-end-with-svelte/)
provides the HR, manager, and employee self-service client.

> ⚠️ **Demo software.** Not a production HR or payroll system;
> statutory calculations are illustrative stubs; synthetic data
> only. See [spec/regulatory](../spec/regulatory.md).

**Status: not started.** The cross-cutting specification landed
2026-07-18; the delivery queue is
[../spec/tasks.md](../spec/tasks.md) (HCM-T1 onward).

## What it will answer

- *Where is this vacancy in its pipeline?* — requisition +
  application state machines
- *Can this employee take two weeks in August?* — leave balances +
  rota conflicts
- *Who reports to whom?* — the derived org chart
- *What does this month's payroll cost?* — calculated runs with
  per-employee payslips (minor-unit arithmetic, stub tax tables)
- *Which critical roles have no ready successor?* — the succession
  gap report

## Target surface (per the cross-cutting spec)

Requisitions / candidates / applications / interviews · onboarding
items · employees + org-chart · time entries · leave entitlements +
requests · shifts + assignments · benefit plans + enrollments ·
review cycles / reviews / goals / feedback · training enrollments ·
succession plans · payroll runs + payslips · benchmarks · audits ·
`/events/recent` · OpenAPI + Swagger · `/metrics.prom`.

Auth enforcement defaults **off** (`HCM_REQUIRE_AUTH` is the family
activation gate); upstream lookups will default to **stub mode**;
events default to the in-memory transport.
