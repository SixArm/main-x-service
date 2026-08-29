# Agent guide — event-front-end-with-svelte

Sibling to [`event-service-with-loco/`](../event-service-with-loco/). The Rust crate is the system of record; this is a thin presentation layer that calls its REST API.

## Single source of truth

- The service's [`spec.md`](../event-service-with-loco/spec/index.md) and [`agents/`](../event-service-with-loco/agents/) describe the API contract. If a field disappears from `Event` in the service, fix `src/lib/api/types.ts` here — do not let the front-end drift.
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
- **No global stores** for HTTP state. Construct a `EventRepository` per page/component.

## What lives where

| Concern | Location |
| --- | --- |
| Wire format types | `src/lib/api/types.ts` |
| HTTP envelope handling | `src/lib/api/client.ts` |
| Event endpoints | `src/lib/api/events.ts` |
| Reusable form pieces | `src/lib/forms/` |
| Event-specific components | `src/lib/components/` |
| Routes / pages | `src/routes/` |

## Authentication (BFF)

Sign-in is landed: `/signin` + `/verify` (magic-link, per-app), `src/lib/server/{session,auth,config}.ts`, and the `/api/proxy/[...path]` reverse proxy that injects a server-exchanged PASETO — the browser holds only the httpOnly `__Host-mxi_session` cookie, never a token. See [`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).

**CSRF** (2026-08-28, closes T-23b / the CSRF half of §16 OQ-3): a
double-submit cookie protects mutating browser→BFF calls. `/verify` sets
a second, **non-httpOnly** cookie `__Host-mxi_csrf` alongside the session
cookie (`generateCsrfToken()`/`CSRF_COOKIE`/`CSRF_COOKIE_OPTIONS` in
`src/lib/server/session.ts`); `src/lib/api/client.ts`'s `ApiClient` reads
it from `document.cookie` (guarded — a no-op server-side) and echoes it
as `X-CSRF-Token` on every non-GET/HEAD request; the proxy
(`src/routes/api/proxy/[...path]/+server.ts`) rejects a mismatch with
`403 {"error":"csrf"}` before forwarding upstream, backstopped by an
Origin/Referer check (only rejects when one is present and disagrees).
Sign-out clears both cookies. See
`agents/share/authentication-sessions.md` §4 for the design this
implements. The 401/403 redirect UX (the other half of OQ-3) is still
open.

## Page-visit guard (PRO-H10, 2026-08-29)

Every page whose sole purpose is submitting a mutation (`/events/new`,
`/events/[id]/edit`, `/events/merge`) carries a `+page.server.ts` load
function calling `requireSignedIn(locals)`
(`src/lib/server/session.ts`), which redirects an unauthenticated
visitor to `/signin` (303) rather than render a form whose submit
would fail. Read/list/search/view pages (`/events`, `/events/[id]`,
`/events/[id]/audit`, `/events/match`) stay public — this mirrors the
backend's own default-allow-read / mutation-deny ABAC posture
(`agents/share/authorization-attributes.md` §5) rather than inventing
a separate front-end policy. `locals.sessionId` is presence-only (set
from the httpOnly cookie, never re-validated here) — a UX convenience
in front of the backend's real enforcement, not a substitute for it.

`/calendar` is also left public despite writing back through the
update endpoint on drag-to-reschedule: its entire purpose is *viewing*
the schedule, with an incidental write affordance, not a page whose
sole purpose is submitting a mutation — the same reasoning that leaves
`/events/[id]`'s embedded delete button unguarded. Event has no
`/events/bulk` or `/review` route (no bulk import/export, no review
queue — see `agents/share/overview.md`'s capability matrix), so
neither is guarded here, unlike `person-front-end-with-svelte`'s
otherwise-parallel `/persons/bulk` and `/review`.

**Known v1 limitation, not an oversight**: no `next`-param round trip
back to the originally-requested page after signing in — the
magic-link flow only preserves `return_url`'s origin today
(`src/lib/server/auth.ts::requestMagicLink`), and carrying a return
path through it would touch the authentication-service contract, not
just this app. A visitor who signs in from a guarded page lands on
`/` and navigates back manually.

## What does NOT live here

- FHIR Event UI. Out of scope for MVP.
- Consent management UI. Out of scope for MVP (Event Service has `/consents` endpoints but no front-end yet).
- GDPR-export download UI. Out of scope for MVP.
