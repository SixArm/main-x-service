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

In scope: the routes (`/`, `/new`, `/[pid]`, `/[pid]/edit`, `/insights`,
`/board`, `/gantt`, `/sequence`, `/time`, `/signin`, `/verify`), the API client,
the care-pathway form, the SVAR **DataGrid + FilterBar** registry (`/`),
the five read-only **insights** lenses, the **instances** layer (the
detail page's instances section, the SVAR **Kanban** board, and the SVAR
**Gantt** instance timeline), the intervention-**sequence** Gantt, a
merge-duplicate action on the detail page, a per-pathway audit-trail
view on the detail page, and a BFF + httpOnly-cookie session (§6.9, per
[`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
Out of scope: fuzzy/full-text search UI, system-wide audit feed. The
repository still carries `search()` and `recentEvents()` methods
(unit-tested) from the pre-SVAR list page; neither is wired to any route
today — see §13.

## 3. Stakeholders and users

Clinical informaticians and pathway authors.

## 4. Glossary

- **pid** — the pathway's public id (route param).
- **CarePathway** — the `care_pathway_matcher::CarePathway` payload.
- **check-duplicates** — POST the current record to find stored matches.

## 5. Information architecture

```
/            registry (SVAR DataGrid + FilterBar)
/new         create form
/[pid]       detail + instances + delete + check-duplicates + merge + audit trail
/[pid]/edit  edit form
/insights    five read-only registry lenses
/board       instance Kanban (one pathway; drag = status move)
/gantt       instance timeline Gantt (one pathway)
/sequence    intervention-sequence Gantt (one pathway template)
```

### Layout shell & navigation

Cross-cutting UI rule for every `*-front-end-with-svelte` app:

- Global navigation MUST be a **top navigation bar** (header) spanning the full viewport width. There MUST NOT be a left-hand navigation sidebar / rail.
- On narrow viewports the top-bar navigation MUST collapse behind a **hamburger menu** toggle.
- The main content area MUST be **full-width** — never inset by a persistent side-navigation column.

## 6. Functional requirements

1. Registry (`/`, `GET /api/care-pathways`): pathways render in an SVAR
   **DataGrid** (name / pid columns; row selection navigates to the
   detail route) with an SVAR **FilterBar** doing a client-side
   contains-match filter on name over the loaded rows. Loading, empty,
   and error states are shown.
   - *Superseded, not wired to any route today*: a search-on-submit box
     (`GET /api/care-pathways/search?q=`) and a "Show recent activity"
     toggle (`GET /api/care-pathways/events/recent`) shipped in v0.2/v0.3
     (§15) and were not carried forward when the list page was rebuilt
     onto the SVAR grid (2026-08-01). `CarePathwayRepository.search()`
     and `.recentEvents()` still exist and are still unit-tested (§11)
     but no UI calls them — flagged as a possible unintentional
     regression rather than a deliberate removal; see §13.
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
11. Registry insights (`/insights`, read-only): the five lenses —
    directory (by care setting / specialty), coverage (condition codes ×
    care setting, plus gaps), variants (cross-provider name variants),
    providers (provider directory), languages (per-`in_language`
    counts) — each rendered as a table from its own
    `GET /api/care-pathways/insights/{directory,coverage,variants,providers,languages}`
    call. Each lens's `note` string is shown verbatim; the UI does not
    recompute the numbers.
12. Pathway instances: the detail page (`/[pid]`) renders the template's
    enrolled instances (`GET /api/care-pathways/{pid}/instances`,
    best-effort — a fetch failure degrades to an empty list rather than
    failing the page). The SVAR **Kanban** board (`/board`) shows one
    selected pathway's instances as status columns (Active / On hold /
    Completed / Discontinued); dragging a card calls
    `POST /api/instances/{pid}/status`, and an illegal transition
    (service `422`) is rolled back by reload. The SVAR **Gantt**
    (`/gantt`) renders the same instances as bars from `enrolled_on` to
    `next_review_on ?? closed_on ?? today`, labelled by `subject_ref`;
    instances without an `enrolled_on` list below the chart instead of
    being invented onto the timeline.
13. Intervention-sequence Gantt (`/sequence`): a selected pathway
    template's own `interventions` as ordered bars on an **ordinal**
    axis — explicitly a sequence view, not a schedule (the model carries
    order only, no per-step duration or date).

## 7. Non-functional requirements

Svelte 5 runes only; TS strict (`noUncheckedIndexedAccess`); SPA. Uses
the SVAR component suite (DataGrid + FilterBar, Kanban, Gantt) and the
Lily Design System headless `ThemePicker`/`LocalePicker` — see
`AGENTS.md` ground rule 4. This stopped being a dependency-light app on
2026-07-19 (§15); forms remain plain inputs, not a form-builder library.

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
| `/` registry grid | `GET /api/care-pathways` |
| `/new` | `POST /api/care-pathways` |
| `/[pid]` load | `GET /api/care-pathways/{pid}` |
| `/[pid]` instances | `GET /api/care-pathways/{pid}/instances` (→ `PathwayInstance[]`) |
| `/[pid]` delete | `DELETE /api/care-pathways/{pid}` |
| `/[pid]` duplicates | `POST /api/care-pathways/check-duplicates` |
| `/[pid]` merge | `POST /api/care-pathways/merge` (`{main_pid, duplicate_pid, reason?}`) |
| `/[pid]` audit | `GET /api/care-pathways/{pid}/audit` (→ `AuditEntry[]`) |
| `/[pid]/edit` | `PUT /api/care-pathways/{pid}` |
| `/insights` (5 lenses) | `GET /api/care-pathways/insights/{directory,coverage,variants,providers,languages}` |
| `/board` instance move | `POST /api/instances/{pid}/status` (`{to}`) |
| `/gantt` | `GET /api/care-pathways/{pid}/instances` |
| — (unwired, §6.1/§13) | `GET /api/care-pathways/search?q=`, `GET /api/care-pathways/events/recent` |
| — (repository only, not wired to a route) | `GET /api/instances/{pid}` (`InstanceDetail`), `GET /api/instances/caseload` |

## 10. Persistence

None client-side beyond in-memory route state.

## 11. Testing strategy

`pnpm run check` (svelte-check strict, 0/0). **vitest** unit tests
(`tests/unit/`, 48 tests across 5 files) cover: `client.test.ts` (11 —
the `ApiClient` verb/body/headers/error-classification/empty-body;
no bearer-token tests remain, since the browser holds no token under the
BFF model, §6.9); `care-pathways.test.ts` (25 — every
`CarePathwayRepository` method's path + verb, incl. a regression pinning
`check-duplicates`, `search()` and `recentEvents()` (still pinned though
unwired to any route, §6.1), `merge()` pinning `POST /merge` with the
`{main_pid, duplicate_pid, reason?}` body plus its `404`/`422`
`ApiError` propagation, `audit()`, and the insights/instances/caseload
methods); `i18n.test.ts` (6 — the 13-locale catalog: exact locale list,
full key coverage every locale, RTL flags, English fallback); `layout.test.ts`
(1); and `care-pathway-form.test.ts` (5, via `@testing-library/svelte`
mounted client-side by the `svelteTesting()` vite plugin: the
required-name guard blocks `onsubmit` on a blank/whitespace name and
shows the banner; `build()` trims scalars + nulls blanks, splits the
comma list fields incl. `in_language`, drops empty condition-code /
identifier rows, and collapses a `Custom` care-setting / identifier-scheme
seed). **Playwright** smoke tests (`tests/e2e/smoke.spec.ts`, 6 tests,
API stubbed via `page.route`) cover: the registry grid renders a seeded
pathway; the detail page renders the pathway plus its instances; the
insights page renders all five lens tables; the board renders instances
as Kanban cards; the gantt renders the instance timeline; the sequence
route renders the intervention-sequence gantt. The v0.2/v0.3-era smoke
tests for `/new`, `/[pid]/edit`, list search, detail merge, and
detail/list audit/recent-activity were replaced by this suite in the
2026-08-01 SVAR rebuild rather than kept alongside it — merge and audit
trail are exercised only at the vitest/repository level today (§13).
They run against the production build (`vite preview`) to avoid the
`vite dev` cold-start module race. Run: `pnpm test` (vitest) and
`pnpm test:e2e` (Playwright).

## 12. Compliance

Care pathways are clinical artefacts; defer to the service's controls
for any access/audit requirements.

## 13. Tasks (live work queue)

- [x] vitest unit tests for `ApiClient` + `CarePathwayRepository`
  (incl. merge 404/422 `ApiError` propagation) + `CarePathwayForm`
  (required-name guard + `build()` normalization, incl. `in_language`) +
  the i18n catalog (`tests/unit/`, 48 tests across 5 files — see §11).
- [x] Playwright smoke, rewritten 2026-08-01 for the SVAR surface
  (`tests/e2e/smoke.spec.ts`, 6 tests — registry / detail+instances /
  insights / board / gantt / sequence, API stubbed, runs against
  `vite preview`). Superseded the earlier 8-test suite covering the four
  CRUD routes plus search/merge/audit/recent-activity; see §11 for what
  is no longer exercised at the e2e level.
- [x] Merge-duplicate action on the detail page — each duplicate row
  offers "Merge into this record" (two-step inline confirm) calling
  `POST /api/care-pathways/merge`; adopts the returned survivor and
  re-checks. `repository.merge()` added; vitest covers it (no e2e
  coverage since the 2026-08-01 Playwright rewrite, §11).
- [x] Audit-trail view on the detail page — a "Show audit trail" toggle
  lazy-loads `GET /api/care-pathways/{pid}/audit` and renders the rows
  newest-first (action, actor or "—", timestamp) with loading/empty/error
  states. `repository.audit()` + `AuditEntry` type added; vitest covers
  it (no e2e coverage since the 2026-08-01 Playwright rewrite, §11).
- [x] SVAR component suite (2026-07-19 – 2026-08-01): DataGrid +
  FilterBar registry (`/`), Kanban instance board (`/board`), Gantt
  instance timeline (`/gantt`) and intervention-sequence Gantt
  (`/sequence`), the five `/insights` lenses, and the `/[pid]` instances
  section — see §6.1/§6.11–13. This retired the §7 "dependency-light
  (no data grid / design system)" claim; fixed 2026-08-04.
- [ ] **Follow-up (flagged 2026-08-04, not yet triaged):** decide
  whether the list-page search-on-submit box and "Show recent activity"
  toggle (v0.2/v0.3, dropped in the 2026-08-01 SVAR rebuild) should be
  restored — the FilterBar covers client-side name filtering but not
  full-text search or event-stream visibility — or whether
  `CarePathwayRepository.search()`/`.recentEvents()` and their unit
  tests should be retired as intentionally superseded. Either way,
  restore Playwright coverage for merge and audit trail, which currently
  have no e2e assertions (§11).
- [ ] `Custom(label)` editing for code systems / settings / schemes.
- [x] ~~Bearer token wiring — reactive token store `$lib/auth.svelte`
  (`localStorage["mxi_access_token"]`) + `ApiClient` bearer attach +
  layout paste/clear affordance~~ — **superseded** (see auth-migration task below).
- [x] ~~Cross-origin SSO token handoff — `captureTokenFromHash` +
  `captureFromLocation()` fragment capture + `signInUrl()` redirect~~ —
  **superseded** (see auth-migration task below).
- [x] Auth — adopt BFF + httpOnly cookie + CSRF; remove
  `mxi_access_token`/`localStorage` bearer + fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
  **Done:** `src/lib/server/` (session cookie + magic-link +
  session→PASETO exchange), `/signin` + `/verify` routes, and the
  same-origin `/api/proxy` BFF route that injects the PASETO bearer
  server-side; the browser holds no token.

## 14. Implementation status

Done: all nine routes (`/`, `/new`, `/[pid]`, `/[pid]/edit`,
`/insights`, `/board`, `/gantt`, `/sequence`, `/signin`+`/verify`); lean
client; repository (CRUD + `checkDuplicates` + `merge` + `audit` + the
five `insights*` lenses + `listInstances`/`getInstance`/
`setInstanceStatus`/`caseload`, plus the unwired `search()` and
`recentEvents()` — §6.1/§13); SVAR **DataGrid + FilterBar** registry,
**Kanban** instance board, **Gantt** instance timeline +
intervention-sequence Gantt, and the `/insights` lenses; detail-page
instances section, merge-duplicate action, and audit-trail toggle; BFF
auth (`src/lib/server/` session cookie + magic-link + session→PASETO
exchange, `/signin` + `/verify` routes, `/api/proxy` bearer injection —
the browser holds no token); form (incl. condition codes + identifiers
editors); SPA config. `pnpm run check` clean; production build
succeeds.

## 15. Roadmap

v0.1: CRUD + duplicate-check UI. v0.2: tests + search box. v0.3:
audit-trail view + recent-activity view + auth token + cross-origin SSO
sign-in handoff (shipped, since superseded by the BFF + cookie-session
model — §13). v0.4 (2026-07-19 – 2026-08-01): rebuilt onto the SVAR
component suite — DataGrid + FilterBar registry, Kanban instance board,
Gantt instance timeline + intervention-sequence Gantt, the five
insights lenses, and Lily theme/locale pickers — retiring the v0.1–v0.3
dependency-light posture (§7). The v0.2 search box and v0.3
recent-activity view were not carried into the v0.4 registry page and
their Playwright coverage (along with merge/audit's) was dropped in the
same rebuild; neither has been re-triaged (§13).

## 16. Open questions

- Real-time duplicate warning on the create form?
- Inline validation of ICD/SNOMED code formats?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
