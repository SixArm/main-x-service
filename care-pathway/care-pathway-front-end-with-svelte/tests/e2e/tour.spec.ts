import { test, expect } from "@playwright/test";

// /tour is public: it renders signed-out, with the opener plus six
// workflow sections of four steps each, and no backend calls.
test("tour renders anonymously", async ({ page }) => {
  await page.goto("/tour");
  await expect(page).toHaveURL(/\/tour$/);
  await expect(page.locator(".tour section[id]")).toHaveCount(8);
  await expect(page.locator(".tour .steps li")).toHaveCount(28);
  await expect(page.locator("body")).not.toContainText("tour.");
});
