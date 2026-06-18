// Playwright smoke tests: load each top-level route in a real browser and
// assert its key landmarks render. These guard wiring/routing, not logic.
import { expect, test } from "@playwright/test";

test.describe("Event front-end smoke", () => {
    // Pins: the dashboard heading and the primary nav links are present.
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Events" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    // Pins: the events list shows its heading, the search box, and the
    // "New event" action (scoped to <main> to avoid the sidebar link).
    test("events list renders search box and new event link", async ({ page }) => {
        await page.goto("/events");
        await expect(page.getByRole("heading", { name: "Events" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New event" })).toBeVisible();
    });

    // Pins: the new-event form exposes the required Name/Start fields and
    // the Create button.
    test("new event form renders required name and start fields", async ({ page }) => {
        await page.goto("/events/new");
        await expect(page.getByRole("heading", { name: "New event" })).toBeVisible();
        await expect(page.getByLabel(/^Name/)).toBeVisible();
        await expect(page.getByLabel(/^Start/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    // Pins: the match-check page renders with its "Find matches" button.
    test("match check form renders", async ({ page }) => {
        await page.goto("/events/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    // Pins: the merge page renders both the main and duplicate ID inputs.
    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/events/merge");
        await expect(page.getByRole("heading", { name: "Merge events" })).toBeVisible();
        await expect(page.getByLabel(/Main event ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate event ID/)).toBeVisible();
    });

    // Pins FR-5: the detail page renders the event identity once loaded.
    // The Event Service GET is stubbed at the browser boundary so the smoke
    // suite needs no backing service.
    test("detail page renders the loaded event identity", async ({ page }) => {
        await page.route("**/api/v1/events/evt-1", (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify({
                    success: true,
                    data: {
                        id: "evt-1",
                        name: "Annual Conference",
                        start_date: "2026-06-01T09:00:00Z",
                        event_status: "scheduled",
                        event_type: "conference",
                    },
                    error: null,
                }),
            }),
        );
        await page.goto("/events/evt-1");
        await expect(page.getByRole("heading", { name: "Annual Conference" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Edit" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Audit" })).toBeVisible();
    });

    // Pins FR-6: the edit page renders its shell heading even before the
    // record loads (the EventForm fills in once the GET resolves).
    test("edit page renders its heading", async ({ page }) => {
        await page.goto("/events/evt-1/edit");
        await expect(page.getByRole("heading", { name: "Edit event" })).toBeVisible();
    });

    // Pins: the per-record audit route renders its heading shell.
    test("audit page renders its heading", async ({ page }) => {
        await page.goto("/events/evt-1/audit");
        await expect(page.getByRole("heading", { name: "Audit log" })).toBeVisible();
    });
});
