# Agent guide — course-front-end-with-svelte

Sibling to [`course-service-with-loco/`](../course-service-with-loco/). The Rust crate is the system of record; this is a thin presentation layer that calls its REST API.

## Single source of truth

- The service's [`spec.md`](../course-service-with-loco/spec/index.md) and [`agents/`](../course-service-with-loco/agents/) describe the API contract. If a field disappears from `Course` in the service, fix `src/lib/api/types.ts` here — do not let the front-end drift.
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

- FHIR Course UI. Out of scope — the service has no FHIR surface (service spec §2.2).
- Consent management UI. Out of scope for MVP (the Course Service exposes no consent endpoints).
- GDPR-export download UI. Out of scope for MVP.

## Authentication (BFF)

Landed 2026-06-18, family-wide. The browser holds only the httpOnly
`__Host-mxi_session` cookie; the SvelteKit server (`src/hooks.server.ts`,
`src/lib/server/{auth,config,session}.ts`) does the magic-link exchange
and PASETO minting server-side, and `src/routes/api/proxy/[...path]/+server.ts`
attaches the token on every proxied call. No token ever reaches client JS
or `localStorage`. See
[`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).

**CSRF** (2026-08-28, closes the CSRF half of the prior gap noted in
`spec/13-tasks.md` T-26): a double-submit cookie protects mutating
browser→BFF calls. `/verify` sets a second, **non-httpOnly** cookie
`__Host-mxi_csrf` alongside the session cookie
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

Every page whose sole purpose is submitting a mutation — `/courses/new`,
`/courses/[id]/edit`, `/courses/merge` — carries a `+page.server.ts`
load function calling `requireSignedIn(locals)`
(`src/lib/server/session.ts`), which redirects an unauthenticated
visitor to `/signin` (303) rather than render a form whose submit would
fail. Read/list/search/view pages stay public: `/courses` (list),
`/courses/[id]` (detail — its embedded soft-delete action does not make
the page's *sole* purpose a mutation, same call as the reference's
`/persons/[id]`), `/courses/[id]/audit`, `/courses/match` (an
evaluative match-check probe with no persistent state change),
`/board` (the lifecycle Kanban — primarily a view; a drag *is* a PUT,
but the page's purpose is visualizing status, not submitting one), and
`/calendar` (read-only instance schedule view). This mirrors the
backend's own default-allow-read / mutation-deny ABAC posture
(`agents/share/authorization-attributes.md` §5) rather than inventing a
separate front-end policy. `locals.sessionId` is presence-only (set
from the httpOnly cookie, never re-validated here) — a UX convenience
in front of the backend's real enforcement, not a substitute for it.
Course has no bulk-import/export UI (`agents/share/overview.md`'s
capability matrix: "–" for course) and no review-queue route, so
neither exists to guard; course also has no standalone
`CourseInstance` create/edit route today (T-15: instances render
read-only on the detail page), so there is nothing instance-specific
to guard either.

**Known v1 limitation, not an oversight**: no `next`-param round trip
back to the originally-requested page after signing in — the
magic-link flow only preserves `return_url`'s origin today, and
carrying a return path through it would touch the
authentication-service contract, not just this app. A visitor who
signs in from a guarded page lands on `/` and navigates back manually.

The entity service's own `COURSE_REQUIRE_AUTH` gate (default off)
remains the real enforcement point for whether an unauthenticated
*mutation* is actually accepted, per the family's activation-gate
design (`agents/share/security.md` §4); the BFF proxy forwards a
request regardless of session presence, exactly as the reference proxy
does — this guard only stops a signed-out visitor from *seeing* a form
that would otherwise fail (or silently succeed against a deployment
with enforcement off).
