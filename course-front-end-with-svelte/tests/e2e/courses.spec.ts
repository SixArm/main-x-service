import { expect, test } from "@playwright/test";

test.describe("Course front-end smoke", () => {
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Courses" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    test("courses list renders search box and new course link", async ({ page }) => {
        await page.goto("/courses");
        await expect(page.getByRole("heading", { name: "Courses" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New course" })).toBeVisible();
    });

    test("new course form renders required name field", async ({ page }) => {
        await page.goto("/courses/new");
        await expect(page.getByRole("heading", { name: "New course" })).toBeVisible();
        await expect(page.getByLabel(/^Name/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    test("match check form renders", async ({ page }) => {
        await page.goto("/courses/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/courses/merge");
        await expect(page.getByRole("heading", { name: "Merge courses" })).toBeVisible();
        await expect(page.getByLabel(/Main course ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate course ID/)).toBeVisible();
    });
});
