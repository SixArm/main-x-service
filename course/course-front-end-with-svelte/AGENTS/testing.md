# Testing strategy

Two automated layers plus a manual integration pass, mapped to spec
[§11 Testing Strategy](../spec/11-testing-strategy.md).

| Layer | Tool | Runs against | Purpose |
|---|---|---|---|
| Unit | Vitest + jsdom | Mocked `fetch` / in-memory | Pin `ApiClient` envelope handling, `ApiError` mapping, `CourseRepository` wiring, form-store behaviour, `CourseForm` validation |
| Smoke (e2e) | Playwright | Built SvelteKit preview, no service | Pin route shells render, nav, basic form rendering (mocked) |
| Live integration | (manual) | `pnpm dev` against a running `course-service-with-loco` | Click through CRUD / match / merge over real HTTP |

There is **no** `tests/integration/` directory and **no**
`test:integration` script — live integration is the manual pass in the
table above. Only two automated directories exist: `tests/unit/`
(Vitest) and `tests/e2e/` (Playwright).

## Running

```bash
# Unit tests (vitest)
pnpm test

# Smoke (playwright; no service needed)
pnpm test:e2e

# Type check (svelte-kit sync + svelte-check)
pnpm check

# Lint (prettier --check src)
pnpm lint
```

## Unit tests (`tests/unit/`)

Real files: `client.test.ts` (ApiClient envelope + error tests),
`courses.test.ts` (CourseRepository wiring + search query-param
forwarding), `form.test.ts` (the `createForm` rune controller:
validate-blocks-submit, submit-error capture, reset, per-field
set/clear), and `courseFormValidate.test.ts` (FR-4 rules — required
name, http(s) URL fields, `course_code` ≤ 100, `number_of_credits`
≥ 0 — plus the `normalizeForWire` blank→undefined sweep). The
validator/normaliser live in `src/lib/components/courseFormValidate.ts`
(extracted from `CourseForm.svelte` so they unit-test without a DOM
mount).

Conventions:

- Mock `fetch` via a cast `vi.fn()`-style handler and assert on URL,
  method, headers, body.
- For repository tests: pin the exact route path. The Course Service
  duplicate-check endpoint is `POST /api/courses/check-duplicates`
  (not `/duplicates`).
- For envelope tests: assert that `ApiClient` unwraps `{ success,
  data, error }` correctly and that `ApiError` exposes the
  `isConflict` / `isNotFound` / `isValidation` shortcuts.

`ApiClient` takes an **options object**, not positional args:

```ts
import { describe, it, expect, vi } from "vitest";
import { ApiClient } from "$lib/api/client";

describe("ApiClient", () => {
  it("unwraps the success envelope", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ success: true, data: { id: 1 }, error: null }), {
        headers: { "content-type": "application/json" },
      }),
    ) as unknown as typeof fetch;
    const client = new ApiClient({ baseUrl: "http://test", fetch: fetchMock });
    const result = await client.get("/api/health");
    expect(result).toEqual({ id: 1 });
  });
});
```

Form-store / validation tests run the rune-backed controller directly.
`createForm` and `CourseForm`'s `validate` are pure functions of their
input, so they unit-test without a DOM mount.

## Smoke tests (`tests/e2e/`)

Real file: `courses.spec.ts`.

Conventions:

- Cover route shells, nav, form rendering — **not** API behaviour.
- Use Playwright's `expect(page).toHaveURL(/.../)` for route assertions.
- Mock or stub the API at the browser boundary (`page.route(...)`) so
  the smoke suite passes without a backing service.
- Goal: every MVP route renders, primary action is clickable.

```ts
import { test, expect } from "@playwright/test";

test("dashboard renders health badge", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText(/healthy|ok|up/i)).toBeVisible();
});
```

## Manual live integration

No automated tests. Run `pnpm dev` (or a preview build) with
`PUBLIC_API_BASE_URL` pointed at a running
`course-service-with-loco`, then click through one flow per spec §6
FR: search-finds-record, create-lands-on-detail, inline-409, edit-PUT,
soft-delete-hides, match-renders-score, merge-soft-deletes-duplicate,
audit-log-presence. Each pass should create its own records and clean
up via the soft-delete endpoint — do not assume a pristine database.

## CI expectations

- `pnpm check` (svelte-check) must report 0 errors, 0 warnings.
- `pnpm test` (vitest) must pass.
- `pnpm test:e2e` (smoke) must pass without a backing service.

See [`../CHANGELOG.md`](../CHANGELOG.md) for the validation-status
table that ships with each release.
