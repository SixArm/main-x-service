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
| Calendar view (`/calendar`) | ✅ |
| Auth — BFF cookie + PASETO exchange + proxy | ✅ (§13 T-23a) |
| Auth — CSRF | ❌ (§13 T-23b) |
| i18n — 13 locales (app shell + routes) | ✅ |
| i18n — `/signin` / `/verify` | ❌ — plain English only (§13 T-24) |
| Unit tests | ✅ (`client.test.ts` + `events.test.ts` + `form.test.ts` + `i18n.test.ts` + `layout.test.ts`, 34 tests) |
| E2E smoke | ✅ (`events.spec.ts`) |
| `pnpm check` verified | ✅ — 0 errors, 0 warnings |
| `pnpm test` verified | ✅ — 34/34 passing |
| Live integration | ❌ — pending operator walkthrough |

