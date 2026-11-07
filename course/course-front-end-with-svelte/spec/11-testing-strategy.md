## 11. Testing Strategy

| Layer | Tool | Scope |
| --- | --- | --- |
| Unit | Vitest + jsdom | `ApiClient` bare-JSON + problem handling, `ApiError` mapping, `CourseRepository` wiring (incl. search query-param forwarding), the `createForm` controller, and `CourseForm` validation rules (`courseFormValidate`). |
| E2E smoke | Playwright | Page-shell rendering for every MVP route without requiring a live service. |
| Live integration | (manual) | Run `pnpm dev` against a running `course-service-with-axum`; click through CRUD/match/merge. |

Run: `pnpm test`, `pnpm test:e2e`.

