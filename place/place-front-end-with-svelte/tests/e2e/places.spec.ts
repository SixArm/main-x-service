import { expect, test } from "@playwright/test";

test.describe("Place front-end smoke", () => {
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Places" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    test("places list renders search box and new place link", async ({ page }) => {
        await page.goto("/places");
        await expect(page.getByRole("heading", { name: "Places" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New place" })).toBeVisible();
    });

    test("new place form renders required name field", async ({ page }) => {
        await page.goto("/places/new");
        await expect(page.getByRole("heading", { name: "New place" })).toBeVisible();
        await expect(page.getByLabel(/^Name/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    test("match check form renders", async ({ page }) => {
        await page.goto("/places/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/places/merge");
        await expect(page.getByRole("heading", { name: "Merge places" })).toBeVisible();
        await expect(page.getByLabel(/Main place ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate place ID/)).toBeVisible();
    });
});
