## 11. Testing Strategy

| Layer | Tool | Scope |
| --- | --- | --- |
| Unit | Vitest + jsdom | `ApiClient` envelope handling, `ApiError` mapping, `EventRepository` wiring (incl. `fuzzy` param, merge body shape, merge-preview GET), and `createForm` form-store + Event time-window validation (FR-4). |
| E2E smoke | Playwright | Page-shell rendering for every MVP route without requiring a live service (dashboard, list, new, match, merge, detail, edit, audit). |
| Live integration | (manual) | Run `pnpm dev` against a running `event-service-with-loco`; click through CRUD/match/merge. |

Run: `pnpm test`, `pnpm test:e2e`.

