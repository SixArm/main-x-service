## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `thing-service-with-loco/AGENTS/models.md`.
- [x] T-3: `ApiClient` + `ThingRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `ThingRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [ ] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Edit UI for the remaining Thing fields — `images`, `main_entity_of_page`, `subject_of`, `potential_action` (the edit form re-PUTs the whole record, so these round-trip unchanged; identifiers, alternate names, and same-as URLs are already editable).
- [ ] T-16: Theming tokens in `app.css` extracted to a small theme module.
- [ ] T-17: `check-duplicates` endpoint wired into create form (preview before commit).
- [x] T-18: Batch deduplicate-scan results UI — `/review` (SVAR Kanban: Pending / Confirmed / Rejected / AutoMerged), landed 2026-07-19; drag-to-decide against `POST /api/things/review-queue/{id}/decision` landed the same day.
- [ ] T-19: Masked-view toggle on detail page.
- [ ] T-20: GDPR-export download button.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [x] T-22: Auth — BFF + httpOnly `__Host-mxi_session` cookie + session→PASETO exchange; the browser never holds a token (per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)). Landed 2026-07-04 (`f66ff50f`): `hooks.server.ts`, `/signin`, `/verify`, `/api/proxy/[...path]`, `src/lib/server/{config,session,auth}.ts`. **CSRF is not yet implemented** — the BFF has no `X-CSRF-Token`/synchroniser-cookie check on mutating browser→BFF calls; tracked as a follow-up rather than closed silently under this checkbox.
- [ ] T-23: E2E coverage for the BFF pages — `tests/e2e/things.spec.ts` (5 tests) covers only the pre-auth MVP routes; `/review`, `/signin`, and `/verify` have no Playwright smoke test yet.

