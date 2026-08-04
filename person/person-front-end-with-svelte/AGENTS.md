# Agent guide — person-front-end-with-svelte

Sibling to [`person-service-with-loco/`](../person-service-with-loco/). The Rust crate is the system of record; this is a thin presentation layer that calls its REST API.

## Single source of truth

- The service's [`spec.md`](../person-service-with-loco/spec/index.md) and [`AGENTS/`](../person-service-with-loco/AGENTS/) describe the API contract. If a field disappears from `Person` in the service, fix `src/lib/api/types.ts` here — do not let the front-end drift.
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
- **No global stores** for HTTP state. Construct a `PersonRepository` per page/component.

## What lives where

| Concern | Location |
| --- | --- |
| Wire format types | `src/lib/api/types.ts` |
| HTTP envelope handling | `src/lib/api/client.ts` |
| Person endpoints | `src/lib/api/persons.ts` |
| Reusable form pieces | `src/lib/forms/` |
| Person-specific components | `src/lib/components/` |
| BFF server-only config/session/auth helpers (never bundled to the browser) | `src/lib/server/` |
| Routes / pages | `src/routes/` |

## Authentication — the BFF pattern

This front-end is its own Backend-For-Frontend, per
[`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
The browser never holds a token:

- `/signin` (`src/routes/signin/`) requests a magic link from the
  authentication service, returning to this app's own `/verify`.
- `/verify` (`src/routes/verify/`) consumes the single-use link
  server-side and sets the httpOnly `__Host-mxi_session` cookie
  (`src/lib/server/session.ts`) — no access token ever reaches the
  browser, `localStorage`, or a URL fragment.
- `src/hooks.server.ts` reads that cookie into `locals.sessionId` on
  every request.
- `src/routes/api/proxy/[...path]/+server.ts` is the reverse proxy every
  entity-API call goes through: it strips the browser's cookie,
  exchanges the session for a short-lived PASETO server-side
  (`src/lib/server/auth.ts::exchangeToken`), and forwards with
  `Authorization: Bearer …` to `PERSON_API_URL`.
- Sign-out is the root `+page.server.ts`'s `signout` action: revokes the
  session server-side, then clears the cookie.

**Remaining gap**: no explicit CSRF synchroniser token on mutating
browser→BFF calls yet — only `SameSite=Lax` backstops it today. See
`spec/13-tasks.md` T-22 and `spec/16-open-questions.md` OQ-3.

## What does NOT live here

- FHIR Person UI. Out of scope for MVP.
- Consent management UI. Out of scope for MVP (Person Service has `/consents` endpoints but no front-end yet).
- GDPR-export download UI. Out of scope for MVP.
