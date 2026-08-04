## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `place-service-with-loco/AGENTS/models.md`.
- [x] T-3: `ApiClient` + `PlaceRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `PlaceRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [ ] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Identifier / opening-hours / amenity edit (these sub-record lists are read-only on detail; the edit form re-PUTs the whole record but has no UI to add/remove them. Address and geo are already editable via `PlaceForm`. Rewritten 2026-06-13: the original wording said "emergency-contact edit" — a person-entity copy artifact; places have no emergency contacts).
- [ ] T-16: Theming tokens in `app.css` extracted to a small theme module.
- [ ] T-17: `check-duplicates` endpoint wired into create form (preview before commit).
- [x] T-18: Batch deduplicate-scan results UI. Landed 2026-07-19 as `/review` — SVAR Kanban board (Pending / Confirmed / Rejected / AutoMerged) that loads the stored `GET /api/places/review-queue` on mount and drives decisions through `POST /api/places/review-queue/{id}/decision`; the scan button (`POST /api/places/deduplicate`, destructive-classed) is explicit, never a page-load side effect.
- [ ] T-19: Masked-view toggle on detail page.
- [ ] T-20: GDPR-export download button.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [x] T-22: Auth — adopt BFF + httpOnly cookie + CSRF; the browser holds only `__Host-mxi_session`, the SvelteKit server attaches a short-lived PASETO server-side; no `mxi_access_token`/`localStorage` bearer, no fragment handoff (per [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)). Landed: `/signin` + `/verify` routes, `src/lib/server/{session,auth,config}.ts`, and the `/api/proxy/[...path]` reverse proxy that injects the server-exchanged PASETO. CSRF on mutating browser→BFF calls is not yet separately verified — worth a follow-up task if it isn't covered elsewhere.

