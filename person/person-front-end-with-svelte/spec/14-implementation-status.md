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
| Cross-service links panel (FR-21, T-23) | ✅ `LinksPanel.svelte` on `/persons/[id]` |
| Bulk import/export screen (FR-22, T-24) | ✅ `/persons/bulk` |
| Duplicate review-queue screen (FR-14…FR-20, T-25) | ✅ `/review` — board + queue table + comparison panel |
| Auth — BFF + httpOnly cookie + PASETO exchange (T-22) | ✅ session/sign-in/sign-out; ⚠️ CSRF synchroniser token not yet added |
| Theme + locale switchers (FR-11 / FR-12) | ✅ (Lily `ThemePicker` + `LocalePicker` in the layout shell) |
| Full 13-locale i18n catalog (parity-tested) | ✅ (`tests/unit/i18n.test.ts`; every locale covers every key, no English-fallback stubs) |
| Unit tests | ✅ (7 files, 69 tests — `client`, `persons`, `bulk`, `links-validation`, `review`, `i18n`, `layout`) |
| E2E smoke | ✅ (`tests/e2e/`, route-stubbed) |
| `pnpm install` verified | ✅ (`node_modules` present) |
| `pnpm test` verified | ✅ |
| Live integration | ⚠️ partial — the duplicate-detector test-data interaction (3/9 failing) is fixed (PRO-P4, 2026-08-29, OQ-5); 7/9 now fail instead on the newer page-visit auth guard (PRO-H10) + CSRF check (PRO-H5), which this suite does not sign in for — see OQ-5 |

