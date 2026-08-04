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
| Duplicate-review board (`/review`, drag-to-decide) | ✅ (2026-07-19) |
| Credential-expiry calendar (`/expiry`) | ✅ (2026-07-19) |
| BFF auth (`/signin`, `/verify`, session cookie, PASETO proxy) | ✅ (2026-06-18) — CSRF still pending, T-22b |
| Cross-service links panel (worker detail) | ✅ (2026-08-03, FE-2) |
| i18n — 13 locales, full key parity | ✅ (pinned by `tests/unit/i18n.test.ts`) |
| Unit tests | ✅ (37 tests across `client.test.ts`, `workers.test.ts`, `links-validation.test.ts`, `i18n.test.ts`, `layout.test.ts`) |
| E2E smoke | ✅ (7 tests, `tests/e2e/workers.spec.ts`) |
| `pnpm install` verified | ✅ (DOC-4, 2026-08-04) |
| `pnpm check` verified | ✅ 0 errors / 0 warnings (DOC-4, 2026-08-04) |
| `pnpm test` verified | ✅ (DOC-4, 2026-08-04) |
| `pnpm build` verified | ✅ (DOC-4, 2026-08-04) |
| Live integration | ❌ — pending operator walkthrough against a running Worker Service |

