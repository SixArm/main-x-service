# Agent guide — thing-front-end-with-svelte

Sibling to [`thing-service-with-loco/`](../thing-service-with-loco/). The Rust crate is the system of record; this is a thin presentation layer that calls its REST API.

## Single source of truth

- The service's [`spec.md`](../thing-service-with-loco/spec/index.md) and [`AGENTS/`](../thing-service-with-loco/AGENTS/) describe the API contract. If a field disappears from `Thing` in the service, fix `src/lib/api/types.ts` here — do not let the front-end drift.
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
| BFF session + auth helpers (server-only) | `src/lib/server/` |

## Authentication — BFF pattern (implemented)

This front-end runs its own SvelteKit server as a **Backend-For-Frontend**,
per [`agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)
§6. The browser holds only the httpOnly `__Host-mxi_session` cookie and
never sees a token or calls the Thing Service directly:

- `src/hooks.server.ts` resolves `locals.sessionId` from the cookie.
- `src/routes/signin/` + `src/routes/verify/` implement the magic-link
  login/verify pages (`src/lib/server/auth.ts` calls the authentication
  service).
- `src/routes/api/proxy/[...path]/+server.ts` is the same-origin reverse
  proxy: it exchanges the session for a short-lived PASETO
  (`src/lib/server/auth.ts::exchangeToken`) and forwards to
  `THING_API_URL` with `Authorization: Bearer <paseto>`. Pages keep using
  the existing `ApiClient`/`ThingRepository` unchanged — its base URL just
  points at `/api/proxy`.
- Env vars are read server-side only, in `src/lib/server/config.ts`
  (`THING_API_URL`, `AUTH_API_URL`) — see `.env.example`.

## What does NOT live here

- FHIR Thing UI. Out of scope for MVP.
- Consent management UI. Out of scope for MVP (Thing Service has `/consents` endpoints but no front-end yet).
- GDPR-export download UI. Out of scope for MVP.
