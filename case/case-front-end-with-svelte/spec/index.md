# Case Front-End — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test. Live work queue is §13.
>
> Sibling service: [case-service](../../case-service-with-loco/spec/index.md).

## 1. Purpose and vision

A small SvelteKit SPA for caseworkers to create, browse, edit, and
duplicate-check governmental case-management records via the case
service.

## 2. Scope

In scope: the four routes (`/`, `/new`, `/[pid]`, `/[pid]/edit`), the
API client, the case form, and a BFF + httpOnly-cookie session (§6.7/§6.8,
per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
Out of scope: full-text search UI, audit views.

## 3. Stakeholders and users

Caseworkers and case administrators across governmental agencies.

## 4. Glossary

- **pid** — the case's public id (route param).
- **Case** — the `case_matcher::Case` payload.
- **check-duplicates** — POST the current record to find stored matches.

## 5. Information architecture

```
/            list of cases
/new         create form
/[pid]       detail + delete + check-duplicates
/[pid]/edit  edit form
```

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.

## 6. Functional requirements

1. List active cases (`GET /api/cases`).
2. Create (`POST`), redirect to the new detail page.
3. Detail: render the stored `Case`; offer edit, delete, and
   check-duplicates.
4. Edit (`PUT`), redirect back to detail.
5. Delete (`DELETE`, soft), redirect to the list.
6. Check-duplicates posts the current record and lists matches (title,
   score, confidence), excluding the record itself.
7. Session affordance (BFF + httpOnly cookie): the top navigation bar
   offers a primary **Sign in** link and, once signed in, **Sign out**.
   The browser holds only the `__Host-mxi_session` httpOnly cookie — **no
   token in JS, no `localStorage`**. The SvelteKit **server** (BFF) holds
   the session and attaches a short-lived PASETO server-side when calling
   the case service, so operator traffic passes the service's blanket
   enforcement (`CASE_REQUIRE_AUTH`) once activated. Mutating browser→BFF
   calls carry a CSRF token. Per
   [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
8. Sign-in (BFF): **Sign in** is routed through the BFF to the central
   **authentication-service** front-end for the passwordless magic-link;
   on success the authentication-service sets the `__Host-mxi_session`
   cookie. There is no client-held access token and **no URL-fragment
   handoff**. Per
   [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
9. Layout shell: global navigation is a full-width **top bar** (header)
   with a **hamburger** toggle on narrow viewports — NOT a left sidebar —
   and the main content area is **full-width**.

## 7. Non-functional requirements

Svelte 5 runes only; TS strict (`noUncheckedIndexedAccess`); SPA;
dependency-light (no data grid / design system).

## 8. Architecture

`ApiClient` (lean, raw-JSON, get/post/put/delete) → `CaseRepository`
→ routes. `CaseForm` builds a `Case` from the inputs (comma lists
split, blanks nulled, case type / status / priority / identifier
schemes from `ALL_*` dropdowns, identifiers as editable rows). Under the
BFF model (§6.7) the browser carries only the `__Host-mxi_session` cookie
and the SvelteKit server attaches the short-lived PASETO server-side when
calling the service; no token is read or attached in browser JS.

## 9. API consumption

| Route / action | Endpoint |
|---|---|
| `/` | `GET /api/cases` |
| `/new` | `POST /api/cases` |
| `/[pid]` load | `GET /api/cases/{pid}` |
| `/[pid]` delete | `DELETE /api/cases/{pid}` |
| `/[pid]` duplicates | `POST /api/cases/check-duplicates` |
| `/[pid]/edit` | `PUT /api/cases/{pid}` |

## 10. Persistence

None client-side beyond in-memory route state.

## 11. Testing strategy

`pnpm run check` (svelte-check strict, 0/0). **vitest** unit tests
(`tests/unit/`) cover the `ApiClient` (verb/body/headers, per-call and
session-store bearer-token attachment, per-call `null` override,
error-classification/empty-body), the session store
(`auth.test.ts`: no-token default, `setToken`/`clearToken` round-trip,
guarded localStorage write-through under the shared key, and
`captureTokenFromHash` — well-formed extract, multi-param, no leading
`#`, URL-decode, empty/`#`, no-token, garbage → `null`), the sign-in URL
builder (`config.test.ts`: `signInUrl` encoded `return_to`, base path,
trailing-slash safety), and
`CaseRepository` (every method's path + verb, incl. a regression pinning
`check-duplicates`).
**Playwright** smoke tests (`tests/e2e/`) load the four routes (`/`,
`/new`, `/[pid]`, `/[pid]/edit`) with the API stubbed via
`page.route`, asserting each renders; they run against the production
build (`vite preview`) to avoid the `vite dev` cold-start module race.
Run: `pnpm test` (vitest) and `pnpm test:e2e` (Playwright).

## 12. Compliance

Cases are governmental records; defer to the service's controls for any
access/audit requirements.

## 13. Tasks (live work queue)

- [x] vitest unit tests for `ApiClient` + `CaseRepository` + auth store +
  `signInUrl` + `CaseForm` assembly (`tests/unit/`, 40 tests across 5 files).
- [x] playwright smoke for the four routes + check-duplicates self-exclusion
  (`tests/e2e/smoke.spec.ts`, 5 tests, API stubbed, runs against `vite preview`).
- [x] ~~Cross-origin SSO token handoff (consumer side): capture token
  from the URL fragment + strip it; `signInUrl` builder + top-bar **Sign
  in** redirect~~ — **superseded** (see auth-migration task below).
- [ ] `Custom(label)` editing for case type / status / schemes.
- [ ] Search box once the service ships search.
- [ ] Auth — adopt BFF + httpOnly cookie + CSRF; remove
  `mxi_access_token`/`localStorage` bearer + fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).

## 14. Implementation status

Done: all four routes; lean client; repository; form (incl. case
type / status / priority dropdowns + identifiers editor); SPA config.
`pnpm run check` clean; production build succeeds.

## 15. Roadmap

v0.1 (here): CRUD + duplicate-check UI. v0.2: tests + search box.
v0.3: auth (BFF + httpOnly cookie + CSRF, per
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md))
+ audit views.

## 16. Open questions

- Real-time duplicate warning on the create form?
- Inline validation of agency / docket identifier formats?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
