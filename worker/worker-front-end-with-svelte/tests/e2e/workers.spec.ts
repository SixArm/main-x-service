import { expect, test } from "@playwright/test";

// Smoke tests that assert the page shell renders. They do NOT require a
// running Worker Service — failures from the API call are swallowed by
// the page and shown as banners, but the layout still renders. Run with
// the service started (`docker-compose up` in worker-service-with-loco)
// for full coverage of the API-driven paths.

test.describe("Worker front-end smoke", () => {
    // Pins: the dashboard shell renders its heading and the sidebar nav links.
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        // The nav is a hamburger dropdown at every viewport width
        // (deliberate layout design) — open it before asserting links.
        await page.getByRole("button", { name: "Toggle navigation" }).click();
        await expect(page.getByRole("link", { name: "Workers" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    // Pins: the workers list shows its heading, the search box, and the
    // (main-scoped) "New worker" link — i.e. the page chrome loads even if
    // the API call fails.
    test("workers list renders search box and grid", async ({ page }) => {
        await page.goto("/workers");
        await expect(page.getByRole("heading", { name: "Workers" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New worker" })).toBeVisible();
    });

    // Pins: the create form exposes the required name fields and Create button.
    test("new worker form renders required fields", async ({ page }) => {
        await page.goto("/workers/new");
        await expect(page.getByRole("heading", { name: "New worker" })).toBeVisible();
        await expect(page.getByLabel(/Family name/)).toBeVisible();
        await expect(page.getByLabel(/Given names/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    // Pins: native required-field validation keeps focus on the empty family
    // input on submit, so the form doesn't POST with a missing family name.
    test("client-side validation blocks submission with empty family name", async ({ page }) => {
        await page.goto("/workers/new");
        const family = page.getByLabel(/Family name/);
        const given = page.getByLabel(/Given names/);
        await family.fill("");
        await given.fill("John");
        await page.getByRole("button", { name: "Create" }).click();
        await expect(family).toBeFocused();
    });

    // Pins: the match-check page renders its heading and submit button.
    test("match check form renders", async ({ page }) => {
        await page.goto("/workers/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    // Pins: the worker detail page renders the cross-service links panel —
    // its heading, the kind picker with both permitted kinds, and the
    // empty state. Unlike the other smoke tests this one needs a worker to
    // exist, so the two API calls the page makes are stubbed at the network
    // layer rather than requiring a running Worker Service.
    test("worker detail renders the cross-service links panel", async ({ page }) => {
        const id = "0c4f1e2a-0000-4000-8000-000000000009";
        await page.route("**/api/workers/**", async (route) => {
            const envelope = route.request().url().includes("/links")
                ? { success: true, data: [], error: null }
                : {
                      success: true,
                      data: {
                          id,
                          name: { family: "Smith", given: ["John"] },
                          gender: "male",
                          active: true,
                      },
                      error: null,
                  };
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify(envelope),
            });
        });

        await page.goto(`/workers/${id}`);
        await expect(
            page.getByRole("heading", { name: "Cross-service links" }),
        ).toBeVisible();
        const kind = page.getByLabel("Link kind");
        await expect(kind).toBeVisible();
        await expect(kind.locator("option")).toHaveText([
            "Same identity (→ person)",
            "Employed by (→ organization)",
        ]);
        await expect(page.getByText("No cross-service links yet.")).toBeVisible();
        await expect(page.getByRole("button", { name: "Assert link" })).toBeVisible();
    });

    // Pins: the merge page renders its heading and both id inputs.
    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/workers/merge");
        await expect(page.getByRole("heading", { name: "Merge workers" })).toBeVisible();
        await expect(page.getByLabel(/Main worker ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate worker ID/)).toBeVisible();
    });

    // Pins: the merge page seeds both ids from the query string, which is
    // what the review board's post-confirmation deep link relies on.
    test("merge form pre-fills both IDs from the query string", async ({ page }) => {
        await page.goto("/workers/merge?main=main-111&duplicate=dup-222");
        await expect(page.getByLabel(/Main worker ID/)).toHaveValue("main-111");
        await expect(page.getByLabel(/Duplicate worker ID/)).toHaveValue("dup-222");
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

        await page.route("**/api/workers/review-queue**", (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    items: [
                        {
                            id: "r1",
                            worker_id_a: idA,
                            worker_id_b: idB,
                            match_score: 0.91,
                            match_quality: "probable",
                            detection_method: "batch_deduplication",
                            score_breakdown: {
                                name_score: 0.94,
                                birth_date_score: 1.0,
                            },
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
        await page.route(`**/api/workers/${idA}`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    id: idA,
                    name: { family: "Kowalski", given: ["Anna"] },
                    gender: "female",
                    birth_date: "1980-01-15",
                    active: true,
                }),
            }),
        );
        await page.route(`**/api/workers/${idB}`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    id: idB,
                    name: { family: "Kowalsky", given: ["Ana"] },
                    gender: "female",
                    birth_date: "1980-01-15",
                    active: true,
                }),
            }),
        );

        await page.goto("/review");
        await expect(page.getByRole("heading", { name: "Review" })).toBeVisible();
        await expect(page.getByTestId("review-board")).toBeVisible();
        // The status filter must exist and default to every status.
        await expect(page.getByLabel("Status")).toHaveValue("all");

        const list = page.getByTestId("review-list");
        await expect(list).toBeVisible();

        // The keyboard-reachable path into the comparison: a real button,
        // not a drag.
        await list.getByRole("button", { name: /^Compare/ }).click();

        const panel = page.getByTestId("review-compare");
        await expect(panel).toBeVisible();
        // Both stubbed records, side by side.
        await expect(panel.getByText("Anna Kowalski")).toBeVisible();
        await expect(panel.getByText("Ana Kowalsky")).toBeVisible();
        // The breakdown renders only the components actually present.
        await expect(panel.getByTestId("review-breakdown")).toBeVisible();
        await expect(
            panel.getByTestId("review-breakdown").getByText("0.94"),
        ).toBeVisible();
        // Both decisions are offered as real buttons for a pending item.
        await expect(
            panel.getByRole("button", { name: "Confirm duplicate" }),
        ).toBeEnabled();
        await expect(panel.getByRole("button", { name: "Reject" })).toBeEnabled();
    });
});
