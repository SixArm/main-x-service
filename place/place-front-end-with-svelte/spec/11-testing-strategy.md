## 11. Testing Strategy

| Layer | Tool | Scope |
| --- | --- | --- |
| Unit | Vitest + jsdom | `ApiClient` envelope handling, `ApiError` mapping, `PlaceRepository` wiring. |
| E2E smoke | Playwright | Page-shell rendering for every MVP route without requiring a live service. |
| Live integration | (manual) | Run `pnpm dev` against a running `place-service-rust-crate`; click through CRUD/match/merge. |

Run: `pnpm test`, `pnpm test:e2e`.

