// Playwright smoke suite: loads each primary route and asserts its key
// landmarks render, catching broken routing / build regressions end-to-end.
import { expect, test } from "@playwright/test";

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
});
