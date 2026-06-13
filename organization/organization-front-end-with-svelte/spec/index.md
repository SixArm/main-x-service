# Organization Front-End — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test. Live work queue is §13.
>
> Sibling service: [organization-service](../../organization-service-rust-crate/spec/index.md).

## 1. Purpose and vision

A small SvelteKit SPA for operators to create, browse, edit, and
duplicate-check organization records via the organization service.

## 2. Scope

In scope: the four routes (`/`, `/new`, `/[pid]`, `/[pid]/edit`), the
API client, and the organization form. Out of scope: full-text search
UI, audit views, auth (the service MVP is unauthenticated).

## 3. Stakeholders and users

Operators curating the organization registry.

## 4. Glossary

- **pid** — the organization's public id (route param).
- **Organization** — the `organization_matcher::Organization` payload.
- **check-duplicates** — POST the current record to find stored matches.

## 5. Information architecture

```
/            list of organizations
/new         create form
/[pid]       detail + delete + check-duplicates
/[pid]/edit  edit form
```

## 6. Functional requirements

1. List active organizations (`GET /api/organizations`).
2. Create (`POST`), redirect to the new detail page.
3. Detail: render the stored `Organization`; offer edit, delete, and
   check-duplicates.
4. Edit (`PUT`), redirect back to detail.
5. Delete (`DELETE`, soft), redirect to the list.
6. Check-duplicates posts the current record and lists matches (name,
   score, confidence), excluding the record itself.
7. **Session / bearer token.** A reactive token store
   (`src/lib/auth.svelte.ts`) holds an access token hydrated from
   `localStorage["mxi_access_token"]` (the family-shared key; guarded for
   SSR/preview). The `ApiClient` attaches `Authorization: Bearer <token>`
   on every request when the store holds one, and omits it otherwise; a
   per-request `token` overrides the store. Service-side enforcement
   (`ORGANIZATION_REQUIRE_AUTH`) is off by default, so an absent token
   keeps current behaviour. See `agents/share/jwt-enforcement.md`.
8. **Cross-origin SSO handoff (token acquisition).** When signed out the
   layout shows a primary **Sign in** action that navigates to
   `signInUrl()` =
   `${VITE_AUTH_FRONTEND_URL}/signin?return_to=<encoded origin+base>`
   (`src/lib/config.ts`). The authentication front-end runs the
   passwordless magic-link, then (if our origin is on its allowlist)
   redirects back to `return_to#access_token=<jwt>`. On app load the
   layout's `onMount` calls `captureFromLocation()` **before any API
   call**: `captureTokenFromHash(window.location.hash)` pulls the token
   out of the fragment, `setToken` stores it, and `history.replaceState`
   strips the fragment from the address bar. The token rides the URL
   fragment (never the query) so it is not sent to servers / logged. A
   manual paste field (under a "Paste a token" disclosure) is kept as a
   dev convenience. Guarded for SSR/preview (no `window`).

## 7. Non-functional requirements

Svelte 5 runes only; TS strict (`noUncheckedIndexedAccess`); SPA;
dependency-light (no data grid / design system).

## 8. Architecture

`ApiClient` (lean, raw-JSON, get/post/put/delete) → `OrganizationRepository`
→ routes. `OrganizationForm` builds an `Organization` from the inputs
(comma lists split, blanks stripped, address assembled only if any field
is set).

## 9. API consumption

| Route / action | Endpoint |
|---|---|
| `/` | `GET /api/organizations` |
| `/new` | `POST /api/organizations` |
| `/[pid]` load | `GET /api/organizations/{pid}` |
| `/[pid]` delete | `DELETE /api/organizations/{pid}` |
| `/[pid]` duplicates | `POST /api/organizations/check-duplicates` |
| `/[pid]/edit` | `PUT /api/organizations/{pid}` |

## 10. Persistence

None client-side beyond in-memory route state.

## 11. Testing strategy

`pnpm run check` (svelte-check strict, 0/0). **vitest** unit tests
(`tests/unit/`) cover the `ApiClient` (verb/body/headers/bearer-token —
explicit token, store-driven attach, clear, and explicit-`null`
override / error-classification / empty-body), the `auth` token store
(`setToken`/`clearToken` round-trip, trim/blank handling) and its SSO
handoff parser `captureTokenFromHash` (extracts the token from a
well-formed fragment, URL-decodes, returns `null` for
empty/garbage/no-token/blank), the `signInUrl` builder (encoded
`return_to` of origin + base, trailing-slash safe), and
`OrganizationRepository` (every method's path + verb, incl. a regression
pinning `check-duplicates`).
**Playwright** smoke tests (`tests/e2e/`) load the four routes (`/`,
`/new`, `/[pid]`, `/[pid]/edit`) with the API stubbed via
`page.route`, asserting each renders; they run against the production
build (`vite preview`) to avoid the `vite dev` cold-start module race.
Run: `pnpm test` (vitest) and `pnpm test:e2e` (Playwright).

## 12. Compliance

Contact fields may be personal data; defer to the service's privacy
controls when they land.

## 13. Tasks (live work queue)

- [x] vitest unit tests for `ApiClient` + `OrganizationRepository` +
  `auth` store (`tests/unit/`, 23 tests).
- [x] playwright smoke for the four routes (`tests/e2e/smoke.spec.ts`,
  4 tests, API stubbed, runs against `vite preview`).
- [ ] Identifier `Custom(label)` editing in the form.
- [ ] Search box once the service ships search.
- [x] Bearer token wiring — `auth.svelte.ts` token store
  (`localStorage["mxi_access_token"]`) + `ApiClient` auto-attach +
  layout session affordance, per `agents/share/jwt-enforcement.md`.
  Service enforcement is off by default.
- [x] Cross-origin SSO handoff — `signInUrl()` redirect to the
  authentication front-end + `captureFromLocation()` / `captureTokenFromHash`
  capture the token from `…#access_token=<jwt>` on load and strip the
  fragment (`VITE_AUTH_FRONTEND_URL`), per
  `agents/share/jwt-enforcement.md`. Manual paste kept as a dev fallback.

## 14. Implementation status

Done: all four routes; lean client (+put/delete); repository; form;
SPA config. `pnpm run check` clean; production build succeeds.

## 15. Roadmap

v0.1 (here): CRUD + duplicate-check UI. v0.2: tests + search box.
v0.3: auth token + audit views.

## 16. Open questions

- Real-time duplicate warning on the create form (vs the detail page)?
- Inline validation of identifier formats (LEI/DUNS length)?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
