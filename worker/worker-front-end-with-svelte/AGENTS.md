# Agent guide — worker-front-end-with-svelte

Sibling to [`worker-service-with-loco/`](../worker-service-with-loco/). The Rust crate is the system of record; this is a thin presentation layer that calls its REST API.

## Single source of truth

- The service's [`spec.md`](../worker-service-with-loco/spec/index.md) and [`agents/`](../worker-service-with-loco/agents/) describe the API contract. If a field disappears from `Worker` in the service, fix `src/lib/api/types.ts` here — do not let the front-end drift.
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
- **SvelteKit 2, CSR-only (`+layout.ts` sets `ssr = false`).** `+page.server.ts`/`+layout.server.ts` still exist for the BFF (below) — they run server actions and cookie-backed loads, not page rendering. Pass `event.fetch` to `new ApiClient` where an SSR-adjacent load runs server-side.
- **TypeScript strict + `noUncheckedIndexedAccess`.** No `any` without a comment explaining why.
- **SVAR DataGrid** for tabular data. Native HTML for simple lists.
- **Lily Headless** for accessibility primitives where Lily wins (focus trap, listbox, combobox, dialog). Native HTML elsewhere.
- **No global stores** for HTTP state. Construct a `WorkerRepository` per page/component.

## BFF pattern (auth + API proxy)

This front-end is a **Backend-For-Frontend**, per
[`authentication-sessions.md`](../../agents/share/authentication-sessions.md)
§6 — the browser never holds a bearer token:

- The browser authenticates via this app's own `/signin` → `/verify`
  magic-link pages and thereafter holds only the opaque, httpOnly
  `__Host-mxi_session` cookie (`src/lib/server/session.ts`,
  `src/hooks.server.ts` reads it into `event.locals.sessionId`).
- All entity-API calls from the browser go to the same-origin
  `src/routes/api/proxy/[...path]/+server.ts`, which exchanges the
  session for a short-lived PASETO
  (`src/lib/server/auth.ts::exchangeToken`) and forwards to the Worker
  Service with `Authorization: Bearer <paseto>`. `ApiClient`'s base URL
  is this proxy (`src/lib/config.ts`), so page code is unaware of the
  indirection.
- Server-side base URLs (`WORKER_API_URL`, `AUTH_API_URL`) live in
  `src/lib/server/config.ts`, read from the environment — see
  `.env.example`. Never import `src/lib/server/*` from browser code.
- **Known gap**: CSRF protection on mutating browser→BFF calls
  (`authentication-sessions.md` §4) is not yet implemented — see
  `spec/13-tasks.md` T-22.

## What lives where

| Concern | Location |
| --- | --- |
| Wire format types | `src/lib/api/types.ts` |
| HTTP envelope handling | `src/lib/api/client.ts` |
| Worker endpoints | `src/lib/api/workers.ts` |
| Reusable form pieces | `src/lib/forms/` |
| Worker-specific components | `src/lib/components/` |
| Routes / pages | `src/routes/` |
| BFF: session cookie, PASETO exchange, service base URLs | `src/lib/server/`, `src/hooks.server.ts` |
| BFF: entity-API reverse proxy | `src/routes/api/proxy/[...path]/+server.ts` |

## What does NOT live here

- FHIR Worker UI. Out of scope for MVP.
- Consent management UI. Out of scope for MVP (Worker Service has `/consents` endpoints but no front-end yet).
- GDPR-export download UI. Out of scope for MVP.
