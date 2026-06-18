# Care Pathway Front-End — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test. Live work queue is §13.
>
> Sibling service: [care-pathway-service](../../care-pathway-service-with-loco/spec/index.md).

## 1. Purpose and vision

A small SvelteKit SPA for clinical informaticians to create, browse,
edit, and duplicate-check care-pathway records via the care-pathway
service.

## 2. Scope

In scope: the four routes (`/`, `/new`, `/[pid]`, `/[pid]/edit`), the
API client, the care-pathway form, a name-search box on the list, a
merge-duplicate action on the detail page, a per-pathway audit-trail
view on the detail page, and a system-wide recent-activity (event
stream) view on the list page, and a BFF + httpOnly-cookie session (§6.9, per
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
Out of scope: fuzzy/full-text search UI, system-wide audit feed.

## 3. Stakeholders and users

Clinical informaticians and pathway authors.

## 4. Glossary

- **pid** — the pathway's public id (route param).
- **CarePathway** — the `care_pathway_matcher::CarePathway` payload.
- **check-duplicates** — POST the current record to find stored matches.

## 5. Information architecture

```
/            list of care pathways
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

1. List active care pathways (`GET /api/care-pathways`).
   - Search box (search-on-submit): a non-blank query calls
     `GET /api/care-pathways/search?q=` (URL-encoded) and renders the
     filtered results; **Clear** (or an empty query) restores the full
     list. Loading and empty-result states are shown.
   - Recent activity: a "Show recent activity" toggle lazy-loads
     `GET /api/care-pathways/events/recent` on first open and renders the
     events newest-first (highest `seq` first): the kind
     (created/updated/deleted/merged), the name (linked to the pathway by
     pid), and the `seq`. Loading, empty, and error states are shown; the
     panel does not auto-load on mount.
2. Create (`POST`), redirect to the new detail page.
3. Detail: render the stored `CarePathway`; offer edit, delete, and
   check-duplicates.
4. Edit (`PUT`), redirect back to detail.
5. Delete (`DELETE`, soft), redirect to the list.
6. Check-duplicates posts the current record and lists matches (name,
   score, confidence), excluding the record itself.
7. Merge: each duplicate row offers "Merge into this record" (the detail
   record is the survivor/main; the row's pid is the duplicate). A
   two-step inline confirm calls `POST /api/care-pathways/merge` with
   `{main_pid, duplicate_pid}`. On success it adopts the returned
   survivor record, re-runs check-duplicates, and shows a success
   message. Equal pids are guarded client-side (the service `422`s);
   `404`/other errors surface via the error banner.
8. Audit trail: the detail page offers a "Show audit trail" toggle that
   lazy-loads `GET /api/care-pathways/{pid}/audit` on first open and
   renders the rows newest-first (action, actor or "—" when null,
   timestamp). Loading, empty, and error states are shown; the panel
   does not auto-load on mount.
9. Session / auth (BFF + httpOnly cookie): the top navigation bar
   carries a session affordance. The primary path is **Sign in**, routed
   through the BFF to the central authentication front-end for the
   passwordless magic-link; on success the authentication-service sets the
   `__Host-mxi_session` httpOnly cookie. The browser holds only that
   cookie — **no token in JS, no `localStorage`, no URL-fragment handoff**.
   The SvelteKit **server** (BFF) holds the session and attaches a
   short-lived PASETO server-side when calling the care-pathway service;
   the browser never calls the service directly. Mutating browser→BFF
   calls carry a CSRF token; **Sign out** revokes the session. This lets
   operator traffic through once the service turns on blanket enforcement
   (`CARE_PATHWAY_REQUIRE_AUTH`, off by default). Per
   [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md).
10. Layout shell: global navigation is a full-width **top bar** (header)
    with a **hamburger** toggle on narrow viewports — NOT a left sidebar —
    and the main content area is **full-width**.

## 7. Non-functional requirements

Svelte 5 runes only; TS strict (`noUncheckedIndexedAccess`); SPA;
dependency-light (no data grid / design system).

## 8. Architecture

`ApiClient` (lean, raw-JSON, get/post/put/delete) → `CarePathwayRepository`
→ routes. Under the BFF model (§6.9) the browser carries only the
`__Host-mxi_session` cookie and the SvelteKit server attaches the
short-lived PASETO server-side when calling the service; no token is read
or attached in browser JS. `CarePathwayForm` builds a `CarePathway`
from the inputs (comma lists split, blanks nulled, condition codes and
identifiers as editable rows). The editable fields are: `name`
(required), `care_setting` (unit-variant `<select>`; a seeded `Custom`
collapses to "—"), `pathway_code`, `provider_id`, `provider_name`, the
comma-separated list fields `alternate_names` / `interventions` /
`keywords` / `same_as` / `in_language` (BCP-47 tags), and the repeatable
`condition_codes` / `identifiers` rows (empty rows dropped on submit; a
seeded `Custom`-scheme identifier is dropped because the scheme
`<select>` offers only unit variants). The detail page renders the same
fields (incl. `in_language` as "Languages").

## 9. API consumption

| Route / action | Endpoint |
|---|---|
| `/` list | `GET /api/care-pathways` |
| `/` search | `GET /api/care-pathways/search?q=` |
| `/` recent activity | `GET /api/care-pathways/events/recent` (→ `PathwayEvent[]`) |
| `/new` | `POST /api/care-pathways` |
| `/[pid]` load | `GET /api/care-pathways/{pid}` |
| `/[pid]` delete | `DELETE /api/care-pathways/{pid}` |
| `/[pid]` duplicates | `POST /api/care-pathways/check-duplicates` |
| `/[pid]` merge | `POST /api/care-pathways/merge` (`{main_pid, duplicate_pid, reason?}`) |
| `/[pid]` audit | `GET /api/care-pathways/{pid}/audit` (→ `AuditEntry[]`) |
| `/[pid]/edit` | `PUT /api/care-pathways/{pid}` |

## 10. Persistence

None client-side beyond in-memory route state.

## 11. Testing strategy

`pnpm run check` (svelte-check strict, 0/0). **vitest** unit tests
(`tests/unit/`) cover the `ApiClient` (verb/body/headers/bearer-token/
error-classification/empty-body), the **auth token store**
(`tests/unit/auth.test.ts`: `setToken`/`clearToken` round-trip; the
client attaches `Authorization: Bearer` when the store holds a token and
omits it when empty; a per-call token/`null` overrides the store; and
`captureTokenFromHash` — well-formed extract, multi-param fragment, no
leading `#`, URL-decode, empty/`#`, no-token, and garbage/blank → null),
the **SSO sign-in URL builder** (`tests/unit/config.test.ts`: `signInUrl`
encodes `return_to`, includes the SvelteKit base path, and trims a
trailing slash so there is no `//signin`), and
`CarePathwayRepository` (every
method's path + verb, incl. a regression pinning `check-duplicates`,
`search()` pinning the `/search?q=` path with URL-encoding, and
`merge()` pinning `POST /merge` with the `{main_pid, duplicate_pid,
reason?}` body — pids in the body, not the URL, plus `404` (unknown pid)
and `422` (equal-pid) `ApiError` propagation for the detail-page error
banner; `audit()` pinning `GET /{pid}/audit` with URL-encoding; and
`recentEvents()` pinning `GET /events/recent`), and the
**`CarePathwayForm`** component (`tests/unit/care-pathway-form.test.ts`,
via `@testing-library/svelte` mounted client-side by the
`svelteTesting()` vite plugin: the required-name guard blocks `onsubmit`
on a blank/whitespace name and shows the banner; `build()` trims scalars
+ nulls blanks, splits the comma list fields incl. `in_language`, drops
empty condition-code / identifier rows, and collapses a `Custom`
care-setting / identifier-scheme seed). **Playwright** smoke
tests (`tests/e2e/`) load the four routes (`/`, `/new`, `/[pid]`,
`/[pid]/edit`) with the API stubbed via `page.route`, asserting each
renders; one test exercises the list search box (matching query keeps
the row, non-matching shows the empty-result message); one test drives
the detail-page merge action (check-duplicates → confirm merge →
success message, asserting the merge endpoint fired); one test opens the
detail-page audit trail (toggle → rows render with action + "—" actor);
one test opens the list-page recent-activity panel (toggle → events
render newest-first with kind + seq). They run against the production
build (`vite preview`) to avoid the `vite dev` cold-start module race.
Run: `pnpm test` (vitest) and `pnpm test:e2e` (Playwright).

## 12. Compliance

Care pathways are clinical artefacts; defer to the service's controls
for any access/audit requirements.

## 13. Tasks (live work queue)

- [x] vitest unit tests for `ApiClient` + `CarePathwayRepository`
  (incl. merge 404/422 `ApiError` propagation) + auth token store +
  `signInUrl` + `CarePathwayForm` (required-name guard + `build()`
  normalization, incl. `in_language`) (`tests/unit/`, 46 tests across 5 files).
- [x] playwright smoke for the four routes (`tests/e2e/smoke.spec.ts`,
  8 tests — the four routes plus search / merge / audit / recent-activity,
  API stubbed, runs against `vite preview`).
- [x] Merge-duplicate action on the detail page — each duplicate row
  offers "Merge into this record" (two-step inline confirm) calling
  `POST /api/care-pathways/merge`; adopts the returned survivor and
  re-checks. `repository.merge()` added; vitest (2) + Playwright (1)
  cover it.
- [x] Audit-trail view on the detail page — a "Show audit trail" toggle
  lazy-loads `GET /api/care-pathways/{pid}/audit` and renders the rows
  newest-first (action, actor or "—", timestamp) with loading/empty/error
  states. `repository.audit()` + `AuditEntry` type added; vitest (2) +
  Playwright (1) cover it.
- [x] Recent-activity view on the list page — a "Show recent activity"
  toggle lazy-loads `GET /api/care-pathways/events/recent` and renders
  the events newest-first (kind, name linked by pid, seq) with
  loading/empty/error states. `repository.recentEvents()` + `PathwayEvent`
  type added; vitest (1) + Playwright (1) cover it.
- [ ] `Custom(label)` editing for code systems / settings / schemes.
- [x] Search box once the service ships search — list page calls
  `GET /api/care-pathways/search?q=` (search-on-submit + Clear);
  `repository.search()` added; vitest (2) + Playwright (1) cover it.
- [x] ~~Bearer token wiring — reactive token store `$lib/auth.svelte`
  (`localStorage["mxi_access_token"]`) + `ApiClient` bearer attach +
  layout paste/clear affordance~~ — **superseded** (see auth-migration task below).
- [x] ~~Cross-origin SSO token handoff — `captureTokenFromHash` +
  `captureFromLocation()` fragment capture + `signInUrl()` redirect~~ —
  **superseded** (see auth-migration task below).
- [ ] Auth — adopt BFF + httpOnly cookie + CSRF; remove
  `mxi_access_token`/`localStorage` bearer + fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).

## 14. Implementation status

Done: all four routes; lean client; repository (incl. `search()`,
`merge()`, `audit()`, and `recentEvents()`); list search box;
list-page recent-activity (event-stream) view; detail-page
merge-duplicate action; detail-page audit-trail view; auth token store
(`$lib/auth.svelte`) + client bearer-attachment + layout session
affordance with cross-origin SSO sign-in (fragment capture + strip,
`signInUrl` redirect); form (incl.
condition codes + identifiers editors); SPA config. `pnpm run check`
clean; production build succeeds.

## 15. Roadmap

v0.1 (here): CRUD + duplicate-check UI. v0.2: tests + search box.
v0.3: audit-trail view (done) + recent-activity view (done) + auth token
(done) + cross-origin SSO sign-in handoff (done).

## 16. Open questions

- Real-time duplicate warning on the create form?
- Inline validation of ICD/SNOMED code formats?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
