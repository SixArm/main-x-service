# Agent guide — person-front-end-with-svelte

Sibling to [`person-service-with-loco/`](../person-service-with-loco/). The Rust crate is the system of record; this is a thin presentation layer that calls its REST API.

## Single source of truth

- The service's [`spec.md`](../person-service-with-loco/spec/index.md) and [`agents/`](../person-service-with-loco/agents/) describe the API contract. If a field disappears from `Person` in the service, fix `src/lib/api/types.ts` here — do not let the front-end drift.
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

**CSRF** (2026-08-28, closes the prior gap noted in `spec/13-tasks.md`
T-22 / `spec/16-open-questions.md` OQ-3): a double-submit cookie
protects mutating browser→BFF calls. `/verify` sets a second,
**non-httpOnly** cookie `__Host-mxi_csrf` alongside the session cookie
(`generateCsrfToken()`/`CSRF_COOKIE`/`CSRF_COOKIE_OPTIONS` in
`src/lib/server/session.ts`); `src/lib/api/client.ts`'s `ApiClient`
reads it from `document.cookie` (guarded — a no-op server-side) and
echoes it as `X-CSRF-Token` on every non-GET/HEAD request; the proxy
(`src/routes/api/proxy/[...path]/+server.ts`) rejects a mismatch with
`403 {"error":"csrf"}` before forwarding upstream, backstopped by an
Origin/Referer check (only rejects when one is present and disagrees).
Sign-out clears both cookies. See
`agents/share/authentication-sessions.md` §4 for the design this
implements.

## Page-visit guard (PRO-H10, 2026-08-29)

Every page whose sole purpose is submitting a mutation
(`/persons/new`, `/persons/[id]/edit`, `/persons/merge`,
`/persons/bulk`, `/review`) carries a `+page.server.ts` load function
calling `requireSignedIn(locals)` (`src/lib/server/session.ts`), which
redirects an unauthenticated visitor to `/signin` (303) rather than
render a form whose submit would fail. Read/list/search/view pages
(`/persons`, `/persons/[id]`, `/persons/[id]/audit`, `/persons/match`)
stay public — this mirrors the backend's own default-allow-read /
mutation-deny ABAC posture
(`agents/share/authorization-attributes.md` §5) rather than inventing
a separate front-end policy. `locals.sessionId` is presence-only (set
from the httpOnly cookie, never re-validated here) — a UX convenience
in front of the backend's real enforcement, not a substitute for it.

**Known v1 limitation, not an oversight**: no `next`-param round trip
back to the originally-requested page after signing in — the
magic-link flow only preserves `return_url`'s origin today
(`src/lib/server/auth.ts::requestMagicLink`), and carrying a return
path through it would touch the authentication-service contract, not
just this app. A visitor who signs in from a guarded page lands on
`/` and navigates back manually.

## What does NOT live here

- FHIR Person UI. Out of scope for MVP.
- Consent management UI. Out of scope for MVP (Person Service has `/consents` endpoints but no front-end yet).
- GDPR-export download UI. Out of scope for MVP.
