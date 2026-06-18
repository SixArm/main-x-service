## 11. Testing Strategy

| Layer | Tool | Scope | Run |
| --- | --- | --- | --- |
| Unit | Vitest + jsdom | `ApiClient` envelope handling, `ApiError` mapping, `PersonRepository` wiring. | `pnpm test` |
| E2E smoke | Playwright (project `smoke`) | Page-shell rendering for every MVP route. Does not need a live service — the page renders an error banner if the API call fails. | `pnpm test:e2e` |
| Live integration | Playwright (project `integration`) | Golden-path flows that drive the live SvelteKit preview against a running `person-service-with-loco`. 9 tests = 8 FR-mapped (FR-1, FR-3 ×2, FR-5, FR-6, FR-7, FR-8, FR-9) + 1 per-record audit-presence test that maps to no FR. Self-cleanup via REST `DELETE` in `afterAll`. | `bin/e2e` (health-checks then runs) or `pnpm test:integration` |

The integration suite assumes a running Person Service at
`PUBLIC_API_BASE_URL` (default `http://localhost:8080`). Bring it up
with the service's podman compose:

```bash
(cd ../person-service-with-loco && podman compose up -d)
bin/e2e
```

Each integration test creates its own records with a timestamped
family name so the suite is idempotent across runs. Records are
soft-deleted via the REST API in `afterAll`.

