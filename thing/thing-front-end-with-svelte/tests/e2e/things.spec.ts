// Playwright smoke suite: loads each primary route and asserts its key
// landmarks render, catching broken routing / build regressions end-to-end.
import { expect, test } from "@playwright/test";

test.describe("Thing front-end smoke", () => {
    // Pins: dashboard shows its heading and the main nav links.
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Things" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    // Pins: the list page renders the search box and a "New thing" link.
    test("things list renders search box and new thing link", async ({ page }) => {
        await page.goto("/things");
        await expect(page.getByRole("heading", { name: "Things" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New thing" })).toBeVisible();
    });

    // Pins: the new-thing form shows the required Name field and Create button.
    test("new thing form renders required name field", async ({ page }) => {
        await page.goto("/things/new");
        await expect(page.getByRole("heading", { name: "New thing" })).toBeVisible();
        await expect(page.getByLabel(/^Name/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    // Pins: the match-check page renders with its "Find matches" submit.
    test("match check form renders", async ({ page }) => {
        await page.goto("/things/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    // Pins: the merge page renders both the main and duplicate ID inputs.
    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/things/merge");
        await expect(page.getByRole("heading", { name: "Merge things" })).toBeVisible();
        await expect(page.getByLabel(/Main thing ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate thing ID/)).toBeVisible();
    });
});
