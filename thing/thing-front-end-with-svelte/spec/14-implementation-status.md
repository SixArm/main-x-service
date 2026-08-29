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
| Duplicate-review screen (`/review`, board + keyboard-reachable table + inline compare panel) | ✅ (2026-07-19 board; 2026-08-04 filter/table/compare/merge-seed, T-24) |
| Auth — BFF (session cookie, magic-link, PASETO exchange, proxy) | ✅ (2026-07-04, `f66ff50f`) — CSRF not yet implemented (T-22) |
| 13-locale i18n | ✅ — `pnpm test`'s key-parity check covers every locale, including `review.*`/`signin.*`/`verify.*` |
| Unit tests | ✅ (63 tests across `client.test.ts`, `things.test.ts`, `thing-form.test.ts`, `merge-validation.test.ts`, `review.test.ts`, `i18n.test.ts`, `layout.test.ts`) |
| E2E smoke | ✅ (7 tests, `tests/e2e/things.spec.ts`; covers `/`, `/things`, `/things/new`, `/things/match`, `/things/merge` (incl. query-string pre-fill), and `/review` — `/signin`/`/verify` have no Playwright coverage, T-23) |
| `pnpm install` verified | ✅ (2026-08-04) |
| `pnpm check` verified | ✅ (2026-08-04) — 0 errors, 0 warnings |
| `pnpm test` verified | ✅ (2026-08-04) — 63/63 |
| `pnpm build` verified | ✅ (2026-08-04) |
| Live integration | ❌ — pending operator walkthrough against a running service |

