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
| Theme + locale switchers (FR-11 / FR-12) | ✅ (Lily `ThemeSelect` + `LocaleSelect` in the layout shell) |
| Unit tests | ✅ (`client.test.ts` + `persons.test.ts` + `person-form-validation.test.ts`) |
| E2E smoke | ✅ (`tests/e2e/persons.spec.ts`) |
| `pnpm install` verified | ✅ (`node_modules` present) |
| `pnpm test` verified | ✅ |
| Live integration | ⚠️ partial — 6/9 integration tests pass against the live stack (OQ-5 resolved); remaining 3 are duplicate-detector test-data interactions |

