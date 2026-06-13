# Testing strategy

Two layers, mapped to spec [§11 Testing Strategy](../spec/11-testing-strategy.md).

| Layer | Tool | Runs against | Purpose |
|---|---|---|---|
| Unit | Vitest | Mocked `fetch` / in-memory | Pin ApiClient envelope, repository wiring, form-store behaviour |
| Smoke | Playwright (`smoke` project) | Built SvelteKit preview, no service | Pin route shells render, nav, basic form submit (mocked) |

## Running

```bash
# Unit tests (vitest)
pnpm test

# Smoke (playwright; no service needed)
pnpm test:e2e

# Type check
pnpm svelte-check

# Lint (if configured)
pnpm lint
```

## Unit tests (`tests/unit/`)

Conventions:

- One file per source module under test (`client.test.ts`,
  `things.test.ts`).
- Mock `fetch` via `vi.fn()`; assert on URL, method, headers, body.
- For repository tests: pin the exact route path (e.g.
  `POST /api/things/duplicates` — Thing Service uses `/duplicates`,
  not `/check-duplicates`).
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

See [`../CHANGELOG.md`](../CHANGELOG.md) for the validation-status
table that ships with each release.
