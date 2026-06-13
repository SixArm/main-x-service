# Case Front-End — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test. Live work queue is §13.
>
> Sibling service: [case-service](../../case-service-rust-crate/spec/index.md).

## 1. Purpose and vision

A small SvelteKit SPA for caseworkers to create, browse, edit, and
duplicate-check governmental case-management records via the case
service.

## 2. Scope

In scope: the four routes (`/`, `/new`, `/[pid]`, `/[pid]/edit`), the
API client, and the case form. Out of scope: full-text search UI,
audit views, auth.

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

## 6. Functional requirements

1. List active cases (`GET /api/cases`).
2. Create (`POST`), redirect to the new detail page.
3. Detail: render the stored `Case`; offer edit, delete, and
   check-duplicates.
4. Edit (`PUT`), redirect back to detail.
5. Delete (`DELETE`, soft), redirect to the list.
6. Check-duplicates posts the current record and lists matches (title,
   score, confidence), excluding the record itself.
7. Session affordance: the layout sidebar lets an operator paste / clear
   a bearer **access token**, stored in the shared session store. While a
   token is set, the API client attaches `Authorization: Bearer <token>`
   to every request, so operator traffic passes the service's blanket JWT
   enforcement (`CASE_REQUIRE_AUTH`) once activated. The token is issued
   out-of-band by the central **authentication-service** (passwordless
   magic-link); full redirect wiring is a follow-up. The token lives under
   the family-shared `localStorage` key `mxi_access_token` (see the
   family contract `agents/share/jwt-enforcement.md`).

## 7. Non-functional requirements

Svelte 5 runes only; TS strict (`noUncheckedIndexedAccess`); SPA;
dependency-light (no data grid / design system).

## 8. Architecture

`ApiClient` (lean, raw-JSON, get/post/put/delete) → `CaseRepository`
→ routes. `CaseForm` builds a `Case` from the inputs (comma lists
split, blanks nulled, case type / status / priority / identifier
schemes from `ALL_*` dropdowns, identifiers as editable rows). The
reactive session store `src/lib/auth.svelte.ts` (`token` / `setToken` /
`clearToken`, hydrated from `localStorage["mxi_access_token"]`, guarded
for SSR / preview / vitest where `localStorage` is absent) is the default
token source for `ApiClient`, which attaches the bearer header per request
when a token is present; a per-call `token` (string or `null`) overrides.

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
guarded localStorage write-through under the shared key), and
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

- [x] vitest unit tests for `ApiClient` + `CaseRepository`
  (`tests/unit/`, 16 tests).
- [x] playwright smoke for the four routes (`tests/e2e/smoke.spec.ts`,
  4 tests, API stubbed, runs against `vite preview`).
- [ ] `Custom(label)` editing for case type / status / schemes.
- [ ] Search box once the service ships search.
- [ ] Bearer token wiring once the service enforces auth.

## 14. Implementation status

Done: all four routes; lean client; repository; form (incl. case
type / status / priority dropdowns + identifiers editor); SPA config.
`pnpm run check` clean; production build succeeds.

## 15. Roadmap

v0.1 (here): CRUD + duplicate-check UI. v0.2: tests + search box.
v0.3: auth token + audit views.

## 16. Open questions

- Real-time duplicate warning on the create form?
- Inline validation of agency / docket identifier formats?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
