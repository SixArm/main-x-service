# Agent guide — course-front-end-with-svelte

Sibling to [`course-service-with-loco/`](../course-service-with-loco/). The Rust crate is the system of record; this is a thin presentation layer that calls its REST API.

## Single source of truth

- The service's [`spec.md`](../course-service-with-loco/spec/index.md) and [`AGENTS/`](../course-service-with-loco/AGENTS/) describe the API contract. If a field disappears from `Course` in the service, fix `src/lib/api/types.ts` here — do not let the front-end drift.
- This project has its own [`spec.md`](spec/index.md) (§1–§18) for front-end-specific decisions: routes, components, design system, build.

## Three-part change rule

A behavioural change here is one PR with three parts:

1. **Spec edit** — `spec.md` §13 (Tasks) or the relevant numbered section.
2. **Code edit** — `src/`.
3. **Test edit** — `tests/unit/` (vitest) and/or `tests/e2e/` (playwright).

## Drift policy

Per repo decision (2026-06-02), each `*-front-end-with-svelte` project keeps its own copy of API types, client, and form primitives. Drift between front-ends is accepted — do not factor shared code into a shared package without explicit user approval.

## Tech-stack ground rules

- **Svelte 5 runes only.** No legacy `$:` reactive statements, no `export let`. Use `$state`, `$derived`, `$effect`, `$props`, `$bindable`.
- **SvelteKit 2.** Pages are SPAs by default (no SSR data loading) — add `+page.ts` load functions when SSR fetch is needed; pass `event.fetch` to `new ApiClient`.
- **TypeScript strict + `noUncheckedIndexedAccess`.** No `any` without a comment explaining why.
- **SVAR DataGrid** for tabular data. Native HTML for simple lists.
- **Lily Headless** for accessibility primitives where Lily wins (focus trap, listbox, combobox, dialog). Native HTML elsewhere.
- **No global stores** for HTTP state. Construct a `CourseRepository` per page/component.

## What lives where

| Concern | Location |
| --- | --- |
| Wire format types | `src/lib/api/types.ts` |
| HTTP envelope handling | `src/lib/api/client.ts` |
| Course endpoints | `src/lib/api/courses.ts` |
| Reusable form pieces | `src/lib/forms/` |
| Course-specific components | `src/lib/components/` |
| Routes / pages | `src/routes/` |

## What does NOT live here

- Authentication. Out of scope until the service ships auth (Course Service spec §15).
- FHIR Course UI. Out of scope — the service has no FHIR surface (service spec §2.2).
- Consent management UI. Out of scope for MVP (the Course Service exposes no consent endpoints).
- GDPR-export download UI. Out of scope for MVP.
