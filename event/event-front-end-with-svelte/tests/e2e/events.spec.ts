// Playwright smoke tests: load each top-level route in a real browser and
// assert its key landmarks render. These guard wiring/routing, not logic.
//
// The suite carries a stub `__Host-mxi_session` cookie
// (`SMOKE_STORAGE_STATE` in playwright.config.ts), so the pages PRO-H10
// put behind `requireSignedIn` render instead of 303ing to /signin. The
// guard is presence-only, so a stub is enough; the "page-visit guard"
// describe at the bottom drops the cookie and pins the redirect itself.
import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";

test.describe("Event front-end smoke", () => {
    // Pins: the dashboard heading and the primary nav links are present.
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        // The nav is a hamburger dropdown at every viewport width
        // (deliberate layout design) — open it before asserting links.
        await page.getByRole("button", { name: "Toggle navigation" }).click();
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
        await page.route("**/api/events/evt-1", (route) =>
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

    // Pins: the GDPR export button (T-20) fetches GET /api/events/{id}/export
    // and saves what came back as `event-<id>-export.json` — a real browser
    // download (Blob object URL + synthetic anchor), asserted through
    // Playwright's download event, with the saved bytes compared to the
    // stubbed payload so a silently-empty file cannot pass.
    test("event detail downloads the GDPR export as JSON", async ({ page }) => {
        const id = "0c4f1e2a-0000-4000-8000-0000000000cc";
        const payload = { subject: id, exported_at: "2026-09-03T00:00:00Z", records: [] };
        const record = { id, name: "Annual Conference", start_date: "2026-06-01T09:00:00Z", event_type: "conference", event_status: "scheduled" };
        await page.route("**/api/events/**", async (route) => {
            const url = route.request().url();
            const envelope = url.includes("/export")
                ? { success: true, data: payload, error: null }
                : { success: true, data: record, error: null };
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify(envelope),
            });
        });

        await page.goto(`/events/${id}`);
        const downloadPromise = page.waitForEvent("download");
        await page.getByRole("button", { name: "Export data (GDPR)" }).click();
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toBe(`event-${id}-export.json`);
        const saved = await download.path();
        expect(saved).not.toBeNull();
        expect(JSON.parse(readFileSync(saved as string, "utf8"))).toEqual(payload);
        await expect(page.getByRole("button", { name: "Export data (GDPR)" })).toBeEnabled();
    });
});

// Pins the PRO-H10 page-visit guard itself (WEB-1): with NO session
// cookie, every mutation page 303s to /signin rather than rendering a
// form whose submit would fail. This is the one place the smoke suite
// runs anonymous — the stub cookie above would otherwise make a removed
// or broken guard invisible. Read/list/view pages stay public and are
// deliberately not listed here; see AGENTS.md "Page-visit guard".
test.describe("Event front-end page-visit guard", () => {
    test.use({ storageState: { cookies: [], origins: [] } });

    const guarded = [
        "/events/new",
        "/events/0c4f1e2a-0000-4000-8000-000000000001/edit",
        "/events/merge",
    ];

    for (const path of guarded) {
        test(`anonymous visit to ${path} redirects to /signin`, async ({ page }) => {
            await page.goto(path);
            await expect(page).toHaveURL(/\/signin(\?|$)/);
        });
    }
});
