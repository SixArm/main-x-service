import { expect, test } from "@playwright/test";

// Smoke tests that assert the page shell renders. They do NOT require a
// running Worker Service — failures from the API call are swallowed by
// the page and shown as banners, but the layout still renders. Run with
// the service started (`docker-compose up` in worker-service-rust-crate)
// for full coverage of the API-driven paths.

test.describe("Worker front-end smoke", () => {
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Workers" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    test("workers list renders search box and grid", async ({ page }) => {
        await page.goto("/workers");
        await expect(page.getByRole("heading", { name: "Workers" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New worker" })).toBeVisible();
    });

    test("new worker form renders required fields", async ({ page }) => {
        await page.goto("/workers/new");
        await expect(page.getByRole("heading", { name: "New worker" })).toBeVisible();
        await expect(page.getByLabel(/Family name/)).toBeVisible();
        await expect(page.getByLabel(/Given names/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    test("client-side validation blocks submission with empty family name", async ({ page }) => {
        await page.goto("/workers/new");
        const family = page.getByLabel(/Family name/);
        const given = page.getByLabel(/Given names/);
        await family.fill("");
        await given.fill("John");
        await page.getByRole("button", { name: "Create" }).click();
        await expect(family).toBeFocused();
    });

    test("match check form renders", async ({ page }) => {
        await page.goto("/workers/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/workers/merge");
        await expect(page.getByRole("heading", { name: "Merge workers" })).toBeVisible();
        await expect(page.getByLabel(/Main worker ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate worker ID/)).toBeVisible();
    });
});
