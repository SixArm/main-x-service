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
| Duplicate-review board (`/review`, SVAR Kanban + drag-to-decide) | ✅ (2026-07-19) |
| Auth — BFF (session cookie, magic-link, PASETO exchange, proxy) | ✅ (2026-07-04, `f66ff50f`) — CSRF not yet implemented (T-22) |
| 13-locale i18n | ✅ — `pnpm test`'s key-parity check covers every locale, including `review.*`/`signin.*`/`verify.*` |
| Unit tests | ✅ (44 tests across `client.test.ts`, `things.test.ts`, `thing-form.test.ts`, `merge-validation.test.ts`, `i18n.test.ts`, `layout.test.ts`) |
| E2E smoke | ✅ (5 tests; pre-auth MVP routes only — `/review`/`/signin`/`/verify` have no Playwright coverage, T-23) |
| `pnpm install` verified | ✅ (2026-08-04) |
| `pnpm check` verified | ✅ (2026-08-04) — 0 errors, 0 warnings |
| `pnpm test` verified | ✅ (2026-08-04) — 44/44 |
| `pnpm build` verified | ✅ (2026-08-04) |
| Live integration | ❌ — pending operator walkthrough against a running service |

