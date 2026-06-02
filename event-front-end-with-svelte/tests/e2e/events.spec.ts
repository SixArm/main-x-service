import { expect, test } from "@playwright/test";

test.describe("Event front-end smoke", () => {
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Events" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    test("events list renders search box and new event link", async ({ page }) => {
        await page.goto("/events");
        await expect(page.getByRole("heading", { name: "Events" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New event" })).toBeVisible();
    });

    test("new event form renders required name and start fields", async ({ page }) => {
        await page.goto("/events/new");
        await expect(page.getByRole("heading", { name: "New event" })).toBeVisible();
        await expect(page.getByLabel(/^Name/)).toBeVisible();
        await expect(page.getByLabel(/^Start/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    test("match check form renders", async ({ page }) => {
        await page.goto("/events/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/events/merge");
        await expect(page.getByRole("heading", { name: "Merge events" })).toBeVisible();
        await expect(page.getByLabel(/Main event ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate event ID/)).toBeVisible();
    });
});
