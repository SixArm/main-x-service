# Care Pathway Front-End — Specification

> **Single source of truth.** Code conforms to this spec. Behavioural
> change = spec + code + test. Live work queue is §13.
>
> Sibling service: [care-pathway-service](../../care-pathway-service-rust-crate/spec/index.md).

## 1. Purpose and vision

A small SvelteKit SPA for clinical informaticians to create, browse,
edit, and duplicate-check care-pathway records via the care-pathway
service.

## 2. Scope

In scope: the four routes (`/`, `/new`, `/[pid]`, `/[pid]/edit`), the
API client, the care-pathway form, a name-search box on the list, a
merge-duplicate action on the detail page, and a per-pathway audit-trail
view on the detail page. Out of scope: fuzzy/full-text search UI,
system-wide audit/event feeds, auth.

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

## 6. Functional requirements

1. List active care pathways (`GET /api/care-pathways`).
   - Search box (search-on-submit): a non-blank query calls
     `GET /api/care-pathways/search?q=` (URL-encoded) and renders the
     filtered results; **Clear** (or an empty query) restores the full
     list. Loading and empty-result states are shown.
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

## 7. Non-functional requirements

Svelte 5 runes only; TS strict (`noUncheckedIndexedAccess`); SPA;
dependency-light (no data grid / design system).

## 8. Architecture

`ApiClient` (lean, raw-JSON, get/post/put/delete) → `CarePathwayRepository`
→ routes. `CarePathwayForm` builds a `CarePathway` from the inputs
(comma lists split, blanks nulled, condition codes and identifiers as
editable rows).

## 9. API consumption

| Route / action | Endpoint |
|---|---|
| `/` list | `GET /api/care-pathways` |
| `/` search | `GET /api/care-pathways/search?q=` |
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
error-classification/empty-body) and `CarePathwayRepository` (every
method's path + verb, incl. a regression pinning `check-duplicates`,
`search()` pinning the `/search?q=` path with URL-encoding, and
`merge()` pinning `POST /merge` with the `{main_pid, duplicate_pid,
reason?}` body — pids in the body, not the URL; and `audit()` pinning
`GET /{pid}/audit` with URL-encoding). **Playwright** smoke
tests (`tests/e2e/`) load the four routes (`/`, `/new`, `/[pid]`,
`/[pid]/edit`) with the API stubbed via `page.route`, asserting each
renders; one test exercises the list search box (matching query keeps
the row, non-matching shows the empty-result message); one test drives
the detail-page merge action (check-duplicates → confirm merge →
success message, asserting the merge endpoint fired); one test opens the
detail-page audit trail (toggle → rows render with action + "—" actor).
They run against the production
build (`vite preview`) to avoid the `vite dev` cold-start module race.
Run: `pnpm test` (vitest) and `pnpm test:e2e` (Playwright).

## 12. Compliance

Care pathways are clinical artefacts; defer to the service's controls
for any access/audit requirements.

## 13. Tasks (live work queue)

- [x] vitest unit tests for `ApiClient` + `CarePathwayRepository`
  (`tests/unit/`, 16 tests).
- [x] playwright smoke for the four routes (`tests/e2e/smoke.spec.ts`,
  4 tests, API stubbed, runs against `vite preview`).
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
- [ ] `Custom(label)` editing for code systems / settings / schemes.
- [x] Search box once the service ships search — list page calls
  `GET /api/care-pathways/search?q=` (search-on-submit + Clear);
  `repository.search()` added; vitest (2) + Playwright (1) cover it.
- [ ] Bearer token wiring once the service enforces auth.

## 14. Implementation status

Done: all four routes; lean client; repository (incl. `search()`,
`merge()`, and `audit()`); list search box; detail-page merge-duplicate
action; detail-page audit-trail view; form (incl. condition codes +
identifiers editors); SPA config. `pnpm run check` clean; production
build succeeds.

## 15. Roadmap

v0.1 (here): CRUD + duplicate-check UI. v0.2: tests + search box.
v0.3: audit-trail view (done) + auth token.

## 16. Open questions

- Real-time duplicate warning on the create form?
- Inline validation of ICD/SNOMED code formats?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
