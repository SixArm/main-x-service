import { expect, test } from "@playwright/test";

test.describe("Thing front-end smoke", () => {
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Things" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    test("things list renders search box and new thing link", async ({ page }) => {
        await page.goto("/things");
        await expect(page.getByRole("heading", { name: "Things" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New thing" })).toBeVisible();
    });

    test("new thing form renders required name field", async ({ page }) => {
        await page.goto("/things/new");
        await expect(page.getByRole("heading", { name: "New thing" })).toBeVisible();
        await expect(page.getByLabel(/^Name/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    test("match check form renders", async ({ page }) => {
        await page.goto("/things/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/things/merge");
        await expect(page.getByRole("heading", { name: "Merge things" })).toBeVisible();
        await expect(page.getByLabel(/Main thing ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate thing ID/)).toBeVisible();
    });
});
