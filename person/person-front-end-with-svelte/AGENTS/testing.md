# Testing strategy

Three layers, mapped to spec [§11 Testing Strategy](../spec/11-testing-strategy.md).

| Layer | Tool | Runs against | Purpose |
|---|---|---|---|
| Unit | Vitest | Mocked `fetch` / in-memory | Pin ApiClient envelope, repository wiring, form-store behaviour |
| Smoke | Playwright (`smoke` project) | Built SvelteKit preview, no service | Pin route shells render, nav, basic form submit (mocked) |
| Integration | Playwright (`integration` project) | Built preview + a running `person-service` | Pin the actual operator flows end-to-end over real HTTP |

## Running

```bash
# Unit tests (vitest)
pnpm test

# Smoke (playwright; no service needed)
pnpm test:e2e

# Integration (playwright; requires PUBLIC_API_BASE_URL service up)
pnpm test:integration

# Type check
pnpm svelte-check

# Lint (if configured)
pnpm lint
```

## Unit tests (`tests/unit/`)

Conventions:

- One file per source module under test (`client.test.ts`,
  `persons.test.ts`).
- Mock `fetch` via `vi.fn()`; assert on URL, method, headers, body.
- For repository tests: pin the exact route path (e.g.
  `POST /api/persons/check-duplicates` — Person Service uses
  `/check-duplicates`).
- For envelope tests: assert that `ApiClient` unwraps `{success,
  data, error}` correctly and surfaces `ApiError` with `isConflict`
  / `isNotFound` / `isValidation` shortcuts.

## Smoke tests (`tests/e2e/`)

Conventions:

- Cover route shells, nav, form rendering — **not** API behaviour.
- Use Playwright's `expect(page).toHaveURL(/...)` for route assertions.
- Mock or stub the API at the browser boundary (`page.route(...)`) so
  the smoke suite passes without a backing service.
- Goal: every MVP route renders, primary action is clickable.

## Integration tests (`tests/integration/`)

Conventions:

- Require a live `person-service-with-loco` at `PUBLIC_API_BASE_URL`.
- The `playwright.config.ts` `webServer` command bakes
  `PUBLIC_API_BASE_URL` into the preview build so the front-end talks
  to the configured service.
- Each test is **idempotent**: creates its own records, cleans up via
  the service's soft-delete endpoint. Do not assume a pristine
  database.
- 9 tests total = 8 FR-mapped + 1 non-FR audit-presence test:
  search-finds-record (FR-1), create-lands-on-detail (FR-3),
  inline-409 (FR-3), detail-nested-fields (FR-5), edit-PUT (FR-6),
  soft-delete-hides (FR-7), match-renders-score (FR-8),
  merge-soft-deletes-duplicate (FR-9), and audit-log-presence (no FR).

## Writing new tests

### Unit test pattern

```ts
import { describe, it, expect, vi } from "vitest";
import { ApiClient } from "$lib/api/client";

describe("ApiClient", () => {
  it("unwraps the success envelope", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ success: true, data: { id: 1 } })),
    );
    const client = new ApiClient("http://test", fetchMock);
    const result = await client.get("/health");
    expect(result).toEqual({ id: 1 });
  });
});
```

### Playwright pattern

```ts
import { test, expect } from "@playwright/test";

test("dashboard renders health badge", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText(/healthy|ok|up/i)).toBeVisible();
});
```

## CI expectations

- `svelte-check` must report 0 errors, 0 warnings.
- `pnpm test` (vitest) must pass.
- `pnpm test:e2e` (smoke) must pass without a backing service.
- `pnpm test:integration` runs only on PRs that touch
  `src/lib/api/` or `tests/integration/` (or on demand).

See [`../CHANGELOG.md`](../CHANGELOG.md) for the validation-status
table that ships with each release.
