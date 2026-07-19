# Organization Front-End — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test. Live work queue is §13.
>
> Sibling service: [organization-service](../../organization-service-with-loco/spec/index.md).

## 1. Purpose and vision

A small SvelteKit SPA for operators to create, browse, edit, and
duplicate-check organization records via the organization service.

## 2. Scope

In scope: the four routes (`/`, `/new`, `/[pid]`, `/[pid]/edit`), the
API client, the organization form, and the session/auth integration
(§6.7/§6.8 — BFF + httpOnly-cookie + PASETO per
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md);
service-side enforcement off by default). Out of scope: full-text search
UI, audit views.

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

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.

## 6. Functional requirements

1. List active organizations (`GET /api/organizations`).
2. Create (`POST`), redirect to the new detail page.
3. Detail: render the stored `Organization`; offer edit, delete, and
   check-duplicates.
4. Edit (`PUT`), redirect back to detail.
5. Delete (`DELETE`, soft), redirect to the list.
6. Check-duplicates posts the current record and lists matches (name,
   score, confidence), excluding the record itself.
7. **Session (BFF + httpOnly cookie).** The browser holds only the
   `__Host-mxi_session` httpOnly cookie — no token in JS, no
   `localStorage`. The SvelteKit **server** (BFF) holds the session and,
   when calling the organization service, attaches a short-lived PASETO
   server-side; the browser never calls the service directly. Mutating
   browser→BFF calls carry a CSRF token. Service-side enforcement
   (`ORGANIZATION_REQUIRE_AUTH`) is off by default. Per
   [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
8. **Sign-in.** When signed out the layout shows a primary **Sign in**
   action routed through the BFF to the central authentication front-end
   for the passwordless magic-link; on success the authentication-service
   sets the `__Host-mxi_session` cookie. There is no client-held access
   token and no URL-fragment handoff. Per
   [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
9. **Layout shell.** Global navigation is a full-width **top bar**
   (header) with a **hamburger** toggle on narrow viewports — NOT a left
   sidebar — and the main content area is **full-width**.

## 7. Non-functional requirements

Svelte 5 runes only; TS strict (`noUncheckedIndexedAccess`); SPA;
dependency-light (no data grid / design system).

## 8. Architecture

`ApiClient` (lean, raw-JSON, get/post/put/delete) → `OrganizationRepository`
→ routes. The `Organization` payload is the matcher shape serialized
snake_case: `name`, `legal_name`, `alternate_names`, `identifiers`,
`url`, `same_as`, `address`, `jurisdiction`, `founding_date`,
`telephone`, `email` (both contact fields; personal data — see §12),
`keywords`. `OrganizationForm` edits these and assembles the payload via
the shared, unit-tested `src/lib/api/build.ts` (`buildOrganization` +
`splitList`/`blankToUndef`): comma lists split, blanks → `null`, address
assembled all-or-nothing only if any part is set, empty identifier rows
dropped. The detail route's `excludeSelf` (same module) drops the
record's own pid from check-duplicates results (§6.6).

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
(`tests/unit/`, 49 across 5 files) cover:
- `client.test.ts` — the `ApiClient` (verb/body/headers/bearer-token —
  explicit token, store-driven attach, clear, and explicit-`null`
  override / error-classification / empty-body).
- `auth.test.ts` — the `auth` token store (`setToken`/`clearToken`
  round-trip, trim/blank handling), the pure SSO parser
  `captureTokenFromHash` (extract / URL-decode / `null` for
  empty/garbage/no-token/blank), and `captureFromLocation` (browser
  side: stores the token and strips the fragment via
  `history.replaceState`; no-op when none).
- `config.test.ts` — the `signInUrl` builder (encoded `return_to` of
  origin + base, trailing-slash safe).
- `organizations.test.ts` — `OrganizationRepository` (every method's
  path + verb, incl. a regression pinning `check-duplicates`).
- `build.test.ts` — the spec §8 core in `src/lib/api/build.ts`:
  `buildOrganization` (blank → `null`, comma-list split, contact fields,
  all-or-nothing address, dropping empty identifier rows),
  `splitList`/`blankToUndef`, and `excludeSelf` (§6.6 self-match drop).
**Playwright** smoke tests (`tests/e2e/`) load the four routes (`/`,
`/new`, `/[pid]`, `/[pid]/edit`) with the API stubbed via
`page.route`, asserting each renders; they run against the production
build (`vite preview`) to avoid the `vite dev` cold-start module race.
Run: `pnpm test` (vitest) and `pnpm test:e2e` (Playwright).

## 12. Compliance

Contact fields may be personal data; defer to the service's privacy
controls when they land.

## 13. Tasks (live work queue)

- [x] vitest unit tests (`tests/unit/`, 49 across 5 files) for the
  `ApiClient`, `OrganizationRepository`, `auth` store + SSO handoff
  (`captureTokenFromHash` / `captureFromLocation`), `signInUrl`, and the
  form/payload core in `build.ts` (`buildOrganization`, `splitList`,
  `blankToUndef`, `excludeSelf`).
- [x] playwright smoke for the four routes (`tests/e2e/smoke.spec.ts`,
  4 tests, API stubbed, runs against `vite preview`).
- [ ] Identifier `Custom(label)` editing in the form.
- [ ] Search box once the service ships search.
- [x] ~~Bearer token wiring — `auth.svelte.ts` token store
  (`localStorage["mxi_access_token"]`) + `ApiClient` auto-attach +
  layout session affordance~~ — **superseded** (see auth-migration task below).
- [x] ~~Cross-origin SSO handoff — `signInUrl()` redirect +
  `captureFromLocation()` / `captureTokenFromHash` fragment capture~~ —
  **superseded** (see auth-migration task below).
- [x] Auth — adopt BFF + httpOnly cookie + CSRF; remove
  `mxi_access_token`/`localStorage` bearer + fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
  **Done:** `src/lib/server/` (session cookie + magic-link +
  session→PASETO exchange), `/signin` + `/verify` routes, and the
  same-origin `/api/proxy` BFF route that injects the PASETO bearer
  server-side; the browser holds no token.

- [x] **2026-07-19 — `/review` drag-to-decide board.** Stored review
  queue as a SVAR Kanban (loads on mount via `GET /review-queue`; the
  destructive scan stays button-only; drag pending → confirmed/rejected
  posts the decision). Verified: svelte-check 0 errors, vitest 43
  (repo path pins for the three new methods), Playwright 5 (stubbed
  review-board smoke incl. a no-scan-on-load pin).

## 14. Implementation status

Done: all four routes; lean client (+put/delete); repository; form;
SPA config. `pnpm run check` clean; production build succeeds.

## 15. Roadmap

v0.1 (here): CRUD + duplicate-check UI, vitest + Playwright suites. The
v0.1 session shipped as a client-held bearer + cross-origin SSO handoff;
that is now superseded by the BFF + httpOnly-cookie model (§6.7/§6.8,
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)),
which has since shipped (§13). Next: search box once the service ships
search; audit views.

## 16. Open questions

- Real-time duplicate warning on the create form (vs the detail page)?
- Inline validation of identifier formats (LEI/DUNS length)?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
