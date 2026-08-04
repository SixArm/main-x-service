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
| BFF auth (magic-link sign-in + PASETO exchange) | ✅ (landed 2026-06-18; CSRF + route guard still open — T-26) |
| i18n (13 locales) + theme/locale pickers | ✅ (landed 2026-06-18; `/signin`/`/verify` still English-only — T-27) |
| Kanban board / instance calendar | ✅ (landed 2026-07-19) |
| Unit tests | ✅ 35 tests (`client.test.ts` 5 + `courses.test.ts` 6 + `form.test.ts` 7 + `courseFormValidate.test.ts` 9 + `i18n.test.ts` 7 + `layout.test.ts` 1) |
| E2E smoke | ✅ 5 Playwright tests in `e2e/courses.spec.ts` |
| `pnpm install` verified | ✅ |
| `pnpm check` verified | ✅ 0 errors / 0 warnings |
| `pnpm test` verified | ✅ |
| Live integration | ❌ — pending operator walkthrough against a running service |

