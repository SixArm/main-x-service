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
API client, and the care-pathway form. Out of scope: full-text search
UI, audit views, auth.

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
2. Create (`POST`), redirect to the new detail page.
3. Detail: render the stored `CarePathway`; offer edit, delete, and
   check-duplicates.
4. Edit (`PUT`), redirect back to detail.
5. Delete (`DELETE`, soft), redirect to the list.
6. Check-duplicates posts the current record and lists matches (name,
   score, confidence), excluding the record itself.

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
| `/` | `GET /api/care-pathways` |
| `/new` | `POST /api/care-pathways` |
| `/[pid]` load | `GET /api/care-pathways/{pid}` |
| `/[pid]` delete | `DELETE /api/care-pathways/{pid}` |
| `/[pid]` duplicates | `POST /api/care-pathways/check-duplicates` |
| `/[pid]/edit` | `PUT /api/care-pathways/{pid}` |

## 10. Persistence

None client-side beyond in-memory route state.

## 11. Testing strategy

`pnpm run check` (svelte-check strict, 0/0). **vitest** unit tests
(`tests/unit/`) cover the `ApiClient` (verb/body/headers/bearer-token/
error-classification/empty-body) and `CarePathwayRepository` (every
method's path + verb, incl. a regression pinning `check-duplicates`).
**Playwright** smoke tests (`tests/e2e/`) load the four routes (`/`,
`/new`, `/[pid]`, `/[pid]/edit`) with the API stubbed via
`page.route`, asserting each renders; they run against the production
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
- [ ] `Custom(label)` editing for code systems / settings / schemes.
- [ ] Search box once the service ships search.
- [ ] Bearer token wiring once the service enforces auth.

## 14. Implementation status

Done: all four routes; lean client; repository; form (incl. condition
codes + identifiers editors); SPA config. `pnpm run check` clean;
production build succeeds.

## 15. Roadmap

v0.1 (here): CRUD + duplicate-check UI. v0.2: tests + search box.
v0.3: auth token + audit views.

## 16. Open questions

- Real-time duplicate warning on the create form?
- Inline validation of ICD/SNOMED code formats?

## 17. References

- Sibling service spec; SvelteKit + Svelte 5 runes docs.

## 18. Change control

Update this spec with any behavioural change; bump `CHANGELOG.md`.
