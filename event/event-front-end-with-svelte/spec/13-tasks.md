## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `event-service-rust-crate/AGENTS/models.md`.
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
- [ ] T-15: Identifier / address / emergency-contact edit (currently read-only on detail; edit form re-PUTs whole record but no UI to add/remove sub-records).
- [ ] T-16: Theming tokens in `app.css` extracted to a small theme module.
- [ ] T-17: `check-duplicates` endpoint wired into create form (preview before commit).
- [ ] T-18: Batch deduplicate-scan results UI.
- [ ] T-19: Masked-view toggle on detail page.
- [ ] T-20: GDPR-export download button.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.

