## 13. Tasks

- [x] T-1: Scaffold SvelteKit project (config, app shell, CSS).
- [x] T-2: Wire TypeScript types matching `worker-service-with-loco/AGENTS/models.md`.
- [x] T-3: `ApiClient` + `WorkerRepository`.
- [x] T-4: Form primitives (`LabeledField`, `FieldError`, `FieldRow`, `createForm`).
- [x] T-5: List route with SVAR DataGrid + search box.
- [x] T-6: Create route with 409-duplicate inline surfacing.
- [x] T-7: Detail / edit / soft-delete.
- [x] T-8: Audit log view.
- [x] T-9: Match check route.
- [x] T-10: Merge UI with preview.
- [x] T-11: Vitest unit tests for `ApiClient` + `WorkerRepository`.
- [x] T-12: Playwright e2e smoke for every MVP route.
- [x] T-23 (FE-2): Cross-service links panel on the worker detail route —
  list / assert / withdraw the worker's outbound `entity_links` edges
  (`GET`/`POST`/`DELETE /api/workers/{id}/links`). Only the two kinds
  worker may originate are offered (`same_identity` → `person`,
  `employed_by` → `organization`, where `role` is the job title), with a
  client-side mirror of the service's `validate_edge` so a wrong target
  type is caught before the `422`; the server's reason is still shown
  inline when it is the one to refuse. Deliberately distinct from the
  within-service `Worker.links`, which stays untouched (the partition
  rule, [`cross-service-linking.md`](../../../agents/share/cross-service-linking.md) §7).
- [ ] T-13: SSR-safe load functions using `event.fetch` for SEO-irrelevant but warm-cache wins.
- [ ] T-14: Integrate Lily Headless components beyond Button (Dialog for merge confirm, Combobox for identifier system, Banner for error states).
- [ ] T-15: Identifier / address / emergency-contact edit (currently read-only on detail; edit form re-PUTs whole record but no UI to add/remove sub-records).
- [ ] T-16: Theming tokens in `app.css` extracted to a small theme module.
- [ ] T-17: `check-duplicates` endpoint wired into create form (preview before commit).
- [ ] T-18: Batch deduplicate-scan results UI.
- [ ] T-19: Masked-view toggle on detail page.
- [ ] T-20: GDPR-export download button.
- [ ] T-21: Validate the SVAR licensing fit (free GPL-3.0 vs Pro) — see §16 OQ-1.
- [x] T-22a: Auth — adopt the BFF + httpOnly-cookie shape: `/signin` +
  `/verify` per-app magic-link pages, `__Host-mxi_session` httpOnly
  cookie (`src/lib/server/session.ts`, `src/hooks.server.ts`), and the
  same-origin `/api/proxy` reverse proxy that exchanges the session for
  a short-lived PASETO server-side (`src/lib/server/auth.ts`) before
  calling the Worker Service. No `mxi_access_token`/`localStorage`
  bearer, no fragment handoff (per
  [`../../../agents/share/authentication-sessions.md`](../../../agents/share/authentication-sessions.md)).
  Landed 2026-06-18 (`f66ff50f`); see `CHANGELOG.md`.
- [ ] T-22b: CSRF protection on mutating browser→BFF calls
  (`authentication-sessions.md` §4 — synchroniser token echoed in an
  `X-CSRF-Token` header) is not yet implemented. Every `POST`/`PUT`/
  `DELETE` under `/api/proxy` today relies on `SameSite=Lax` alone.

