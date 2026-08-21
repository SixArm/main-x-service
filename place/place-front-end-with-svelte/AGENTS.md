# Agent guide — place-front-end-with-svelte

Sibling to [`place-service-with-loco/`](../place-service-with-loco/). The Rust crate is the system of record; this is a thin presentation layer that calls its REST API.

## Single source of truth

- The service's [`spec.md`](../place-service-with-loco/spec/index.md) and [`agents/`](../place-service-with-loco/agents/) describe the API contract. If a field disappears from `Place` in the service, fix `src/lib/api/types.ts` here — do not let the front-end drift.
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
- **No global stores** for HTTP state. Construct a `PlaceRepository` per page/component.

## Authentication — the BFF pattern

Per [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
§6, the browser never holds a token. It carries only the httpOnly
`__Host-mxi_session` cookie and calls same-origin routes:

- `/signin` and `/verify` — per-app magic-link sign-in / verification
  (`src/lib/server/auth.ts`, `src/lib/server/session.ts`).
- `/api/proxy/[...path]` — a same-origin reverse proxy
  (`src/routes/api/proxy/[...path]/+server.ts`). The server exchanges the
  session for a short-lived PASETO and forwards to the Place Service with
  `Authorization: Bearer <paseto>`; it never forwards the browser's
  cookie upstream. Every browser API call goes through this proxy — see
  `src/lib/config.ts` (`API_BASE_URL` = `location.origin + "/api/proxy"`).
- `src/lib/server/config.ts` reads `PLACE_API_URL` / `AUTH_API_URL`
  server-side only; neither is exposed to the client bundle.

## What lives where

| Concern | Location |
| --- | --- |
| Wire format types | `src/lib/api/types.ts` |
| HTTP envelope handling | `src/lib/api/client.ts` |
| Place endpoints | `src/lib/api/places.ts` |
| Reusable form pieces | `src/lib/forms/` |
| Place-specific components | `src/lib/components/` |
| Routes / pages | `src/routes/` |
| BFF server-only helpers (session, auth, config) | `src/lib/server/` |

## What does NOT live here

- FHIR Place UI. Out of scope for MVP.
- Consent management UI. Out of scope for MVP (Place Service has `/consents` endpoints but no front-end yet).
- GDPR-export download UI. Out of scope for MVP.
