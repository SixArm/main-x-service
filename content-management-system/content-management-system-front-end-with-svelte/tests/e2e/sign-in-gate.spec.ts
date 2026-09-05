// Root sign-in gate (CMS-T31): a visitor with no session is
// redirected to /signin from any page, and /signin itself stays
// reachable with no session. Runs with NO session cookie — the
// opposite of every other e2e spec in this project — so it gets its
// own file rather than sharing `dashboard.spec.ts`/`entries.spec.ts`'s
// signed-in `beforeEach`.

import { expect, test } from "@playwright/test";

test("a signed-out visitor is redirected from the dashboard to /signin", async ({
  page,
}) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/signin$/);
});

test("a signed-out visitor is redirected from a protected page to /signin", async ({
  page,
}) => {
  await page.goto("/entries");
  await expect(page).toHaveURL(/\/signin$/);
});

test("/signin itself stays reachable with no session", async ({ page }) => {
  const response = await page.goto("/signin");
  expect(response?.status()).toBe(200);
  await expect(page).toHaveURL(/\/signin$/);
});
