# Agent guide — thing-front-end-with-svelte

Sibling to [`thing-service-rust-crate/`](../thing-service-rust-crate/). The Rust crate is the system of record; this is a thin presentation layer that calls its REST API.

## Single source of truth

- The service's [`spec.md`](../thing-service-rust-crate/spec.md) and [`AGENTS/`](../thing-service-rust-crate/AGENTS/) describe the API contract. If a field disappears from `Thing` in the service, fix `src/lib/api/types.ts` here — do not let the front-end drift.
- This project has its own [`spec.md`](spec.md) (§1–§18) for front-end-specific decisions: routes, components, design system, build.

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
- **No global stores** for HTTP state. Construct a `ThingRepository` per page/component.

## What lives where

| Concern | Location |
| --- | --- |
| Wire format types | `src/lib/api/types.ts` |
| HTTP envelope handling | `src/lib/api/client.ts` |
| Thing endpoints | `src/lib/api/things.ts` |
| Reusable form pieces | `src/lib/forms/` |
| Thing-specific components | `src/lib/components/` |
| Routes / pages | `src/routes/` |

## What does NOT live here

- Authentication. Out of scope until the service ships auth (Thing Service spec §15).
- FHIR Thing UI. Out of scope for MVP.
- Consent management UI. Out of scope for MVP (Thing Service has `/consents` endpoints but no front-end yet).
- GDPR-export download UI. Out of scope for MVP.
