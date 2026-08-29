# Agent guide — thing-front-end-with-svelte

Sibling to [`thing-service-with-loco/`](../thing-service-with-loco/). The Rust crate is the system of record; this is a thin presentation layer that calls its REST API.

## Single source of truth

- The service's [`spec.md`](../thing-service-with-loco/spec/index.md) and [`agents/`](../thing-service-with-loco/agents/) describe the API contract. If a field disappears from `Thing` in the service, fix `src/lib/api/types.ts` here — do not let the front-end drift.
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

**CSRF** (2026-08-28, closes the gap T-22 flagged, tracked family-wide as
PRO-H5): a double-submit cookie protects mutating browser→BFF calls.
`/verify` sets a second, **non-httpOnly** cookie `__Host-mxi_csrf`
alongside the session cookie (`generateCsrfToken()`/`CSRF_COOKIE`/
`CSRF_COOKIE_OPTIONS` in `src/lib/server/session.ts`); the `ApiClient`
reads it from `document.cookie` (guarded — a no-op server-side) and
echoes it as `X-CSRF-Token` on every non-GET/HEAD request; the proxy
(`src/routes/api/proxy/[...path]/+server.ts`) rejects a mismatch with
`403 {"error":"csrf"}` before forwarding upstream, backstopped by an
Origin/Referer check (only rejects when one is present and disagrees).
Sign-out clears both cookies. See
`../../agents/share/authentication-sessions.md` §4 for the design this
implements.

## Page-visit guard (PRO-H10, 2026-08-29)

Every page whose sole purpose is submitting a mutation
(`/things/new`, `/things/[id]/edit`, `/things/merge`, `/review`)
carries a `+page.server.ts` load function calling
`requireSignedIn(locals)` (`src/lib/server/session.ts`), which redirects
an unauthenticated visitor to `/signin` (303) rather than render a form
whose submit would fail. `/review` is guarded in full even though it
also *lists* the stored review queue on load: the queue exists solely to
be decided on (confirm/reject, plus the deep-link into `/things/merge`),
so an unauthenticated visitor gains nothing from seeing it that isn't
also blocked at the point of action — the same call person's own
`/review` guard made. Read/list/search/view pages stay public
(`/things`, `/things/[id]`, `/things/[id]/audit`, `/things/match`) —
this mirrors the backend's own default-allow-read / mutation-deny ABAC
posture (`agents/share/authorization-attributes.md` §5) rather than
inventing a separate front-end policy. `locals.sessionId` is
presence-only (set from the httpOnly cookie, never re-validated here) —
a UX convenience in front of the backend's real enforcement, not a
substitute for it.

This crate has **no `/things/bulk` route to guard** — unlike person,
thing carries no bulk import/export capability at all (see
`agents/share/overview.md`'s capability matrix: the bulk row is `–` for
thing), so there was nothing to add there.

**Known v1 limitation, not an oversight**: no `next`-param round trip
back to the originally-requested page after signing in — the magic-link
flow only preserves `return_url`'s origin today
(`src/lib/server/auth.ts::requestMagicLink`), and carrying a return path
through it would touch the authentication-service contract, not just
this app. A visitor who signs in from a guarded page lands on `/` and
navigates back manually. This crate's BFF/auth wiring (`hooks.server.ts`,
`/signin`, `/verify`, the proxy, CSRF) is the same shape as person's —
ported in T-22/T-25 — so the limitation applies identically here.

## What does NOT live here

- FHIR Thing UI. Out of scope for MVP.
- Consent management UI. Out of scope for MVP (Thing Service has `/consents` endpoints but no front-end yet).
- GDPR-export download UI. Out of scope for MVP.
