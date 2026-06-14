// End-to-end smoke tests: load each primary route in a real browser and
// assert its key landmarks render. These pin routing + page scaffolding,
// not API behaviour (the backend may be absent; only static UI is checked).
import { expect, test } from "@playwright/test";

test.describe("Place front-end smoke", () => {
    // Pins: the dashboard heading plus the sidebar nav links are present.
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Places" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    // Pins: the places index shows its heading, the searchbox, and the
    // "New place" link (scoped to <main> to avoid the sidebar duplicate).
    test("places list renders search box and new place link", async ({ page }) => {
        await page.goto("/places");
        await expect(page.getByRole("heading", { name: "Places" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New place" })).toBeVisible();
    });

    // Pins: the create page renders the required Name field and Create button.
    test("new place form renders required name field", async ({ page }) => {
        await page.goto("/places/new");
        await expect(page.getByRole("heading", { name: "New place" })).toBeVisible();
        await expect(page.getByLabel(/^Name/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    // Pins: the match-check page renders its heading and "Find matches" button.
    test("match check form renders", async ({ page }) => {
        await page.goto("/places/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    // Pins: the merge page renders both the main and duplicate ID inputs.
    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/places/merge");
        await expect(page.getByRole("heading", { name: "Merge places" })).toBeVisible();
        await expect(page.getByLabel(/Main place ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate place ID/)).toBeVisible();
    });
});
