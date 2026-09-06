// Playwright smoke suite: loads each primary route and asserts its key
// landmarks render, catching broken routing / build regressions end-to-end.
//
// The suite carries a stub `__Host-mxi_session` cookie
// (`SMOKE_STORAGE_STATE` in playwright.config.ts), so the pages PRO-H10
// put behind `requireSignedIn` render instead of 303ing to /signin. The
// guard is presence-only, so a stub is enough; the "page-visit guard"
// describe at the bottom drops the cookie and pins the redirect itself.
import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";

test.describe("Thing front-end smoke", () => {
    // Pins: dashboard shows its heading and the main nav links.
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        // The nav is a hamburger dropdown at every viewport width
        // (deliberate layout design) — open it before asserting links.
        await page.getByRole("button", { name: "Toggle navigation" }).click();
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

    // Pins T-27: the "Mask sensitive" checkbox on /things sets
    // mask_sensitive on the search call and re-fetches immediately (no
    // form re-submit needed), rendering the masked vs. unmasked value the
    // stub returns for the same query.
    test("things list mask-sensitive toggle re-fetches with mask_sensitive", async ({
        page,
    }) => {
        await page.route("**/api/things/search**", async (route) => {
            const url = new URL(route.request().url());
            const masked = url.searchParams.get("mask_sensitive") === "true";
            const thing = {
                id: "0c4f1e2a-0000-4000-8000-00000000000b",
                name: masked ? "[redacted]" : "Pride and Prejudice",
            };
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify({ success: true, data: [thing], error: null }),
            });
        });

        await page.goto("/things");
        await expect(page.getByText("Pride and Prejudice")).toBeVisible();

        await page.getByLabel("Mask sensitive").check();
        await expect(page.getByText("[redacted]")).toBeVisible();
        await expect(page.getByText("Pride and Prejudice")).not.toBeVisible();

        await page.getByLabel("Mask sensitive").uncheck();
        await expect(page.getByText("Pride and Prejudice")).toBeVisible();
        await expect(page.getByText("[redacted]")).not.toBeVisible();
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

    // Pins: the merge page seeds both ids from the query string, which is
    // what the review board's post-confirmation deep link relies on
    // (`$lib/review`'s `mergeHref`).
    test("merge form pre-fills both IDs from the query string", async ({ page }) => {
        await page.goto("/things/merge?main=main-111&duplicate=dup-222");
        await expect(page.getByLabel(/Main thing ID/)).toHaveValue("main-111");
        await expect(page.getByLabel(/Duplicate thing ID/)).toHaveValue("dup-222");
    });

    // Pins: the masked-view toggle (T-19) re-fetches through
    // GET /api/things/{id}/masked and back, rather than redacting
    // client-side — the two stubs return visibly different owner
    // values so the test can tell which endpoint actually answered.
    test("thing detail toggles between the plain and masked view", async ({
        page,
    }) => {
        const id = "0c4f1e2a-0000-4000-8000-00000000000a";
        const base = { id, name: "Pride and Prejudice" };
        await page.route("**/api/things/**", async (route) => {
            const url = route.request().url();
            const envelope = url.includes("/masked")
                ? { success: true, data: { ...base, owner: "[owner withheld]" }, error: null }
                : { success: true, data: { ...base, owner: "Jane Bennet" }, error: null };
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify(envelope),
            });
        });

        await page.goto(`/things/${id}`);
        await expect(page.getByText("Jane Bennet")).toBeVisible();
        await expect(
            page.getByText("Showing the masked view"),
        ).not.toBeVisible();

        await page.getByRole("button", { name: "Show masked" }).click();
        await expect(page.getByText("[owner withheld]")).toBeVisible();
        await expect(page.getByText("Showing the masked view")).toBeVisible();

        await page.getByRole("button", { name: "Show full" }).click();
        await expect(page.getByText("Jane Bennet")).toBeVisible();
        await expect(
            page.getByText("Showing the masked view"),
        ).not.toBeVisible();
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

        await page.route("**/api/things/review-queue**", (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    items: [
                        {
                            id: "r1",
                            thing_id_a: idA,
                            thing_id_b: idB,
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
        await page.route(`**/api/things/${idA}`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    id: idA,
                    name: "Acme Widget",
                    additional_type: "Widget",
                }),
            }),
        );
        await page.route(`**/api/things/${idB}`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    id: idB,
                    name: "Acme Widgett",
                    additional_type: "Widget",
                }),
            }),
        );

        await page.goto("/review");
        await expect(page.getByRole("heading", { name: "Review" })).toBeVisible();
        await expect(page.getByTestId("review-board")).toBeVisible();
        // The status filter must exist and default to every status.
        await expect(page.getByLabel("Status")).toHaveValue("all");

        // Thing has no provenance column on the wire (see `$lib/review`'s
        // module doc) — the detection method is what the queue shows at a
        // glance instead.
        const list = page.getByTestId("review-list");
        await expect(list).toBeVisible();
        await expect(list.getByText("batch_deduplication")).toBeVisible();

        // The keyboard-reachable path into the comparison: a real button,
        // not a drag.
        await list.getByRole("button", { name: /^Compare/ }).click();

        const panel = page.getByTestId("review-compare");
        await expect(panel).toBeVisible();
        // Both stubbed records, side by side.
        await expect(panel.getByText("Acme Widget", { exact: true })).toBeVisible();
        await expect(panel.getByText("Acme Widgett")).toBeVisible();
        // Thing-service never populates `score_breakdown` on the wire
        // today (see `$lib/review`'s module doc), so the breakdown
        // section always renders its documented empty state.
        await expect(panel.getByTestId("review-no-breakdown")).toBeVisible();
        // Both decisions are offered as real buttons for a pending item.
        await expect(
            panel.getByRole("button", { name: "Confirm duplicate" }),
        ).toBeEnabled();
        await expect(panel.getByRole("button", { name: "Reject" })).toBeEnabled();
    });

    // Pins: the GDPR export button (T-20) fetches GET /api/things/{id}/export
    // and saves what came back as `thing-<id>-export.json` — a real browser
    // download (Blob object URL + synthetic anchor), asserted through
    // Playwright's download event, with the saved bytes compared to the
    // stubbed payload so a silently-empty file cannot pass.
    test("thing detail downloads the GDPR export as JSON", async ({ page }) => {
        const id = "0c4f1e2a-0000-4000-8000-0000000000cc";
        const payload = { subject: id, exported_at: "2026-09-03T00:00:00Z", records: [] };
        const record = { id, name: "Pride and Prejudice" };
        await page.route("**/api/things/**", async (route) => {
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

        await page.goto(`/things/${id}`);
        const downloadPromise = page.waitForEvent("download");
        await page.getByRole("button", { name: "Export data (GDPR)" }).click();
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toBe(`thing-${id}-export.json`);
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
test.describe("Thing front-end page-visit guard", () => {
    test.use({ storageState: { cookies: [], origins: [] } });

    const guarded = [
        "/things/new",
        "/things/0c4f1e2a-0000-4000-8000-000000000001/edit",
        "/things/merge",
        "/review",
    ];

    for (const path of guarded) {
        test(`anonymous visit to ${path} redirects to /signin`, async ({ page }) => {
            await page.goto(path);
            await expect(page).toHaveURL(/\/signin(\?|$)/);
        });
    }
});
