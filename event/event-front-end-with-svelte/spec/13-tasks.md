## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `event-service-with-loco/AGENTS/models.md`.
- [x] T-3: `ApiClient` + `EventRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `EventRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [ ] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Sub-record edit for identifiers / locations / parties (organizers, performers) / offers (currently read-only on detail; edit form re-PUTs the whole record but has no UI to add/remove sub-records). (Rewritten 2026-06-13: previous wording named person-service sub-records — addresses / emergency contacts — which Event does not have.)
- [ ] T-16: Theming tokens in `app.css` extracted to a small theme module.
- [ ] T-17: `check-duplicates` endpoint wired into create form (preview before commit).
- [ ] T-18: Batch deduplicate-scan results UI.
- [ ] T-19: Masked-view toggle on detail page.
- [ ] T-20: GDPR-export download button.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [ ] T-22: Phonetic search toggle on the list page — **blocked on the service**. `event-service-with-loco` exposes `q` / `fuzzy` / `mask_sensitive` / date / status / type on `GET /events/search` but no `phonetic` search parameter (Soundex is internal to the matcher's name scoring, not a search query param). Surface a phonetic toggle here only once the service search query accepts one; until then `SearchOptions` carries no `phonetic` field.
- [x] T-23a: Auth — BFF + httpOnly cookie: `/signin` + `/verify` (per-app magic-link), `src/lib/server/{session,auth,config}.ts`, `/api/proxy/[...path]` reverse proxy injecting a server-exchanged PASETO. The browser holds only `__Host-mxi_session`; no `mxi_access_token`/`localStorage` bearer, no fragment handoff (per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
- [ ] T-23b: CSRF protection on mutating browser→BFF calls (synchroniser token per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md) §4) — the remaining half of the original T-23, split out once the cookie/PASETO half landed. See §16 OQ-3.
- [ ] T-24: i18n coverage for `/signin` and `/verify` — currently plain English only (see the in-file comment on `src/routes/signin/+page.svelte`), unlike the rest of the app's 13-locale coverage.

