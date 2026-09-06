// End-to-end smoke tests: load each primary route in a real browser and
// assert its key landmarks render. These pin routing + page scaffolding,
// not API behaviour (the backend may be absent; only static UI is checked).
//
// The suite carries a stub `__Host-mxi_session` cookie
// (`SMOKE_STORAGE_STATE` in playwright.config.ts), so the pages PRO-H10
// put behind `requireSignedIn` render instead of 303ing to /signin. The
// guard is presence-only, so a stub is enough; the "page-visit guard"
// describe at the bottom drops the cookie and pins the redirect itself.
import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";

test.describe("Place front-end smoke", () => {
    // Pins: the dashboard heading plus the sidebar nav links are present.
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        // The nav is a hamburger dropdown at every viewport width
        // (deliberate layout design) — open it before asserting links.
        await page.getByRole("button", { name: "Toggle navigation" }).click();
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

    // Pins: the merge page seeds both ids from the query string, which is
    // what the review board's post-confirmation deep link relies on.
    test("merge form pre-fills both IDs from the query string", async ({ page }) => {
        await page.goto("/places/merge?main=main-111&duplicate=dup-222");
        await expect(page.getByLabel(/Main place ID/)).toHaveValue("main-111");
        await expect(page.getByLabel(/Duplicate place ID/)).toHaveValue("dup-222");
    });

    // Pins: the masked-view toggle (T-19) re-fetches through
    // GET /api/places/{id}/masked and back, rather than redacting
    // client-side — the two stubs return visibly different telephone
    // values so the test can tell which endpoint actually answered.
    test("place detail toggles between the plain and masked view", async ({
        page,
    }) => {
        const id = "0c4f1e2a-0000-4000-8000-00000000000a";
        const base = {
            id,
            name: "Central Park",
            address: { address_locality: "New York" },
        };
        await page.route("**/api/places/**", async (route) => {
            const url = route.request().url();
            const envelope = url.includes("/masked")
                ? { success: true, data: { ...base, telephone: "***-***-5309" }, error: null }
                : { success: true, data: { ...base, telephone: "+1-555-867-5309" }, error: null };
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify(envelope),
            });
        });

        await page.goto(`/places/${id}`);
        await expect(page.getByText("+1-555-867-5309")).toBeVisible();
        await expect(
            page.getByText("Showing the masked view"),
        ).not.toBeVisible();

        await page.getByRole("button", { name: "Show masked" }).click();
        await expect(page.getByText("***-***-5309")).toBeVisible();
        await expect(page.getByText("Showing the masked view")).toBeVisible();

        await page.getByRole("button", { name: "Show full" }).click();
        await expect(page.getByText("+1-555-867-5309")).toBeVisible();
        await expect(
            page.getByText("Showing the masked view"),
        ).not.toBeVisible();
    });

    // Pins T-27: the list/search page's "Mask sensitive fields" checkbox
    // sets `mask_sensitive` on the search request and re-fetches
    // immediately (unlike fuzzy/phonetic, which wait for the next
    // explicit submit) — mirroring the detail page's masked-view toggle
    // above. The two stubbed responses carry different totals so the
    // rendered count is the visible proof the toggle actually changed
    // which response was used, not just the outgoing request.
    test("places list toggles mask_sensitive on the search request", async ({
        page,
    }) => {
        const place = {
            id: "0c4f1e2a-0000-4000-8000-00000000000b",
            name: "Central Park",
            address: { address_locality: "New York" },
        };
        let lastUrl = "";
        await page.route("**/api/places/search**", async (route) => {
            lastUrl = route.request().url();
            const masked = lastUrl.includes("mask_sensitive=true");
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify({
                    success: true,
                    data: { items: [place], total: masked ? 1 : 2 },
                    error: null,
                }),
            });
        });

        await page.goto("/places");
        await expect(page.getByText("2 places")).toBeVisible();
        expect(lastUrl).not.toContain("mask_sensitive=true");

        await page
            .getByRole("checkbox", { name: "Mask sensitive fields" })
            .check();
        await expect(page.getByText("1 place", { exact: true })).toBeVisible();
        expect(lastUrl).toContain("mask_sensitive=true");

        await page
            .getByRole("checkbox", { name: "Mask sensitive fields" })
            .uncheck();
        await expect(page.getByText("2 places")).toBeVisible();
        expect(lastUrl).toContain("mask_sensitive=false");
    });

    // Pins the FE-4 review screen: the board, the keyboard-reachable queue
    // table, and the side-by-side comparison the Compare button opens. The
    // queue and both sides of the pair are stubbed at the network layer so
    // the smoke project keeps its "no service required" contract.
    test("review board lists the queue and compares a pair on demand", async ({
        page,
    }) => {
        const idA = "aaaaaaaa-0000-4000-8000-000000000001";
        const idB = "bbbbbbbb-0000-4000-8000-000000000002";
        const envelope = (data: unknown) =>
            JSON.stringify({ success: true, data, error: null });

        await page.route("**/api/places/review-queue**", (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    items: [
                        {
                            id: "r1",
                            place_id_a: idA,
                            place_id_b: idB,
                            match_score: 0.91,
                            match_quality: "probable",
                            detection_method: "batch_deduplication",
                            status: "pending",
                            reviewed_by: null,
                            created_at: "2026-08-04T09:00:00Z",
                            reviewed_at: null,
                        },
                    ],
                    total: 1,
                }),
            }),
        );
        await page.route(`**/api/places/${idA}`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    id: idA,
                    name: "Central Library",
                    address: { address_locality: "Springfield" },
                }),
            }),
        );
        await page.route(`**/api/places/${idB}`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    id: idB,
                    name: "Central Libary",
                    address: { address_locality: "Springfield" },
                }),
            }),
        );

        await page.goto("/review");
        await expect(page.getByRole("heading", { name: "Review" })).toBeVisible();
        await expect(page.getByTestId("review-board")).toBeVisible();
        // The status filter must exist and default to every status.
        await expect(page.getByLabel("Status")).toHaveValue("all");

        // The detection method is visible at a glance in the queue (place
        // has no provenance column — see `$lib/review`'s module doc).
        const list = page.getByTestId("review-list");
        await expect(list).toBeVisible();
        await expect(list.getByText("batch_deduplication")).toBeVisible();

        // The keyboard-reachable path into the comparison: a real button,
        // not a drag.
        await list.getByRole("button", { name: /^Compare/ }).click();

        const panel = page.getByTestId("review-compare");
        await expect(panel).toBeVisible();
        // Both stubbed records, side by side.
        await expect(panel.getByText("Central Library")).toBeVisible();
        await expect(panel.getByText("Central Libary")).toBeVisible();
        // Place-service never serializes `score_breakdown` on the wire
        // today, so the breakdown section always renders its explicit
        // "not recorded" note rather than a table.
        await expect(panel.getByTestId("review-no-breakdown")).toBeVisible();
        // Both decisions are offered as real buttons for a pending item.
        await expect(
            panel.getByRole("button", { name: "Confirm duplicate" }),
        ).toBeEnabled();
        await expect(panel.getByRole("button", { name: "Reject" })).toBeEnabled();
    });

    // Pins: the GDPR export button (T-20) fetches GET /api/places/{id}/export
    // and saves what came back as `place-<id>-export.json` — a real browser
    // download (Blob object URL + synthetic anchor), asserted through
    // Playwright's download event, with the saved bytes compared to the
    // stubbed payload so a silently-empty file cannot pass.
    test("place detail downloads the GDPR export as JSON", async ({ page }) => {
        const id = "0c4f1e2a-0000-4000-8000-0000000000cc";
        const payload = { subject: id, exported_at: "2026-09-03T00:00:00Z", records: [] };
        const record = { id, name: "Central Park", address: { address_locality: "New York" } };
        await page.route("**/api/places/**", async (route) => {
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

        await page.goto(`/places/${id}`);
        const downloadPromise = page.waitForEvent("download");
        await page.getByRole("button", { name: "Export data (GDPR)" }).click();
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toBe(`place-${id}-export.json`);
        const saved = await download.path();
        expect(saved).not.toBeNull();
        expect(JSON.parse(readFileSync(saved as string, "utf8"))).toEqual(payload);
        await expect(page.getByRole("button", { name: "Export data (GDPR)" })).toBeEnabled();
    });
});

// Pins the PRO-H10 page-visit guard itself (T-25): with NO session
// cookie, every mutation page 303s to /signin rather than rendering a
// form whose submit would fail. This is the one place the smoke suite
// runs anonymous — the stub cookie above would otherwise make a removed
// or broken guard invisible. Read/list/view pages stay public and are
// deliberately not listed here; see AGENTS.md "Page-visit guard".
test.describe("Place front-end page-visit guard", () => {
    test.use({ storageState: { cookies: [], origins: [] } });

    const guarded = [
        "/places/new",
        "/places/0c4f1e2a-0000-4000-8000-000000000001/edit",
        "/places/merge",
        "/review",
    ];

    for (const path of guarded) {
        test(`anonymous visit to ${path} redirects to /signin`, async ({ page }) => {
            await page.goto(path);
            await expect(page).toHaveURL(/\/signin(\?|$)/);
        });
    }
});
