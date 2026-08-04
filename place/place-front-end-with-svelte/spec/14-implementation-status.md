## 14. Implementation Status

| Area | Status |
| --- | --- |
| Project scaffold | ✅ |
| TypeScript types | ✅ |
| API client | ✅ (Vitest covered) |
| List + search | ✅ |
| Create + 409 surfacing | ✅ |
| Detail / Edit / Delete | ✅ |
| Audit view | ✅ |
| Match check | ✅ |
| Merge UI | ✅ |
| Batch deduplicate review board (`/review`) | ✅ (T-18, 2026-07-19) |
| Auth — BFF + httpOnly session + PASETO exchange | ✅ (T-22: `/signin`, `/verify`, `/api/proxy`) |
| Unit tests | ✅ (40 tests across 5 files: `client.test.ts`, `places.test.ts`, `i18n.test.ts`, `form.svelte.test.ts`, `layout.test.ts`) |
| E2E smoke | ✅ (5 tests, `tests/e2e/places.spec.ts`) |
| `pnpm check` verified | ✅ (0 errors, 0 warnings) |
| `pnpm install` / `pnpm test` / `pnpm build` verified | ✅ (2026-08-04 DOC-4 audit) |
| Live integration | ❌ — pending operator walkthrough |

