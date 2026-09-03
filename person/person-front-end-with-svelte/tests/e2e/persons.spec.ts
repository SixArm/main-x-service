import { expect, test } from "@playwright/test";

// Smoke tests that assert the page shell renders. They do NOT require a
// running Person Service — failures from the API call are swallowed by
// the page and shown as banners, but the layout still renders. Run with
// the service started (`docker-compose up` in person-service-with-loco)
// for full coverage of the API-driven paths.

test.describe("Person front-end smoke", () => {
    // Pins: the dashboard shell renders with the primary nav links present.
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        // The nav is a hamburger dropdown at every viewport width
        // (deliberate layout design) — open it before asserting links.
        await page.getByRole("button", { name: "Toggle navigation" }).click();
        await expect(page.getByRole("link", { name: "Persons" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    // Pins: the list page renders the search box and the "New person" CTA.
    test("persons list renders search box and grid", async ({ page }) => {
        await page.goto("/persons");
        await expect(page.getByRole("heading", { name: "Persons" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New person" })).toBeVisible();
    });

    // Pins: the create form exposes the required name fields and submit button.
    test("new person form renders required fields", async ({ page }) => {
        await page.goto("/persons/new");
        await expect(page.getByRole("heading", { name: "New person" })).toBeVisible();
        await expect(page.getByLabel(/Family name/)).toBeVisible();
        await expect(page.getByLabel(/Given names/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    // Pins: client-side validation stops submit and focuses the empty
    // required family field (HTML required attribute behaviour).
    test("client-side validation blocks submission with empty family name", async ({ page }) => {
        await page.goto("/persons/new");
        const family = page.getByLabel(/Family name/);
        const given = page.getByLabel(/Given names/);
        await family.fill("");
        await given.fill("John");
        await page.getByRole("button", { name: "Create" }).click();
        await expect(family).toBeFocused();
    });

    // Pins: the match-check page renders with its submit button.
    test("match check form renders", async ({ page }) => {
        await page.goto("/persons/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    // Pins: the person detail page renders the cross-service links panel
    // (heading + kind select). The detail page renders nothing until the
    // record loads, so this stubs the two API calls at the network layer
    // rather than requiring a live service — keeping the smoke project's
    // "no service required" contract.
    test("person detail renders the cross-service links panel", async ({ page }) => {
        const id = "0c4f1e2a-0000-4000-8000-0000000000aa";
        const envelope = (data: unknown) =>
            JSON.stringify({ success: true, data, error: null });

        // Most specific first: the links collection, then the record.
        await page.route(`**/api/persons/${id}/links`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope([]),
            }),
        );
        await page.route(`**/api/persons/${id}`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    id,
                    name: { family: "Smith", given: ["John"] },
                    gender: "male",
                    active: true,
                }),
            }),
        );

        await page.goto(`/persons/${id}`);
        await expect(
            page.getByRole("heading", { name: "Cross-service links" }),
        ).toBeVisible();
        await expect(page.getByLabel("Link kind")).toBeVisible();
        // The empty state, not a silent blank section.
        await expect(page.getByText("No cross-service links yet.")).toBeVisible();
    });

    // Pins: the masked-view toggle (T-19) re-fetches through
    // GET /api/persons/{id}/masked and back, rather than redacting
    // client-side — the two stubs return visibly different tax_id
    // values so the test can tell which endpoint actually answered.
    test("person detail toggles between the plain and masked view", async ({
        page,
    }) => {
        const id = "0c4f1e2a-0000-4000-8000-0000000000bb";
        const envelope = (data: unknown) =>
            JSON.stringify({ success: true, data, error: null });
        const base = {
            id,
            name: { family: "Smith", given: ["John"] },
            gender: "male",
            active: true,
        };

        await page.route(`**/api/persons/${id}/links`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope([]),
            }),
        );
        // Most specific first: /masked before the bare {id} record.
        await page.route(`**/api/persons/${id}/masked`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({ ...base, tax_id: "***-**-****" }),
            }),
        );
        await page.route(`**/api/persons/${id}`, (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({ ...base, tax_id: "123-45-6789" }),
            }),
        );

        await page.goto(`/persons/${id}`);
        await expect(page.getByText("123-45-6789")).toBeVisible();
        await expect(
            page.getByText("Showing the masked view"),
        ).not.toBeVisible();

        await page.getByRole("button", { name: "Show masked" }).click();
        await expect(page.getByText("***-**-****")).toBeVisible();
        await expect(page.getByText("Showing the masked view")).toBeVisible();

        await page.getByRole("button", { name: "Show full" }).click();
        await expect(page.getByText("123-45-6789")).toBeVisible();
        await expect(
            page.getByText("Showing the masked view"),
        ).not.toBeVisible();
    });

    // Pins: the bulk page renders both submit sections. The recent-jobs
    // fetch runs on mount, so it is stubbed at the network layer (as the
    // detail-page test does) to keep the smoke project service-free.
    test("bulk page renders the import and export sections", async ({ page }) => {
        await page.route("**/api/persons/bulk-jobs**", (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify({ success: true, data: [], error: null }),
            }),
        );

        await page.goto("/persons/bulk");
        await expect(
            page.getByRole("heading", { name: "Bulk import / export" }),
        ).toBeVisible();

        // Import controls.
        await expect(page.getByRole("heading", { name: "Import", exact: true })).toBeVisible();
        await expect(page.getByLabel(/^File/)).toBeVisible();
        await expect(page.getByLabel("Dry run (validate only)")).toBeVisible();
        await expect(page.getByRole("button", { name: "Start import" })).toBeVisible();
        // Parquet is export-only, so it must not be offered for import.
        await expect(
            page.locator("#bulk-import-format option"),
        ).toHaveText(["JSONL", "CSV"]);

        // Export controls.
        await expect(page.getByRole("heading", { name: "Export", exact: true })).toBeVisible();
        await expect(page.getByLabel("Masking profile")).toBeVisible();
        await expect(page.getByRole("button", { name: "Start export" })).toBeVisible();

        // The recent-jobs section renders its empty state, not a blank gap.
        await expect(page.getByText("No bulk jobs yet.")).toBeVisible();
    });

    // Pins: the merge page renders both the main and duplicate id inputs.
    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/persons/merge");
        await expect(page.getByRole("heading", { name: "Merge persons" })).toBeVisible();
        await expect(page.getByLabel(/Main person ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate person ID/)).toBeVisible();
    });

    // Pins: the merge page seeds both ids from the query string, which is
    // what the review board's post-confirmation deep link relies on.
    test("merge form pre-fills both IDs from the query string", async ({ page }) => {
        await page.goto("/persons/merge?main=main-111&duplicate=dup-222");
        await expect(page.getByLabel(/Main person ID/)).toHaveValue("main-111");
        await expect(page.getByLabel(/Duplicate person ID/)).toHaveValue("dup-222");
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

        await page.route("**/api/persons/review-queue**", (route) =>
            route.fulfill({
                status: 200,
                contentType: "application/json",
                body: envelope({
                    items: [
                        {
                            id: "r1",
                            person_id_a: idA,
                            person_id_b: idB,
                            match_score: 0.91,
                            match_quality: "probable",
                            detection_method: "batch_deduplication",
                            score_breakdown: {
                                name_score: 0.94,
                                birth_date_score: 1.0,
                            },
                            status: "pending",
                            provenance: "import",
                            reviewed_by: null,
                            created_at: "2026-08-04T09:00:00Z",
                            reviewed_at: null,
                        },
                    ],
                    total: 1,
                }),
            }),
        );
        await page.route(`**/api/persons/${idA}`, (route) =>
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
        await page.route(`**/api/persons/${idB}`, (route) =>
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

        // Provenance is visible at a glance in the queue, not only in the
        // detail panel.
        const list = page.getByTestId("review-list");
        await expect(list).toBeVisible();
        await expect(list.getByText("Bulk import")).toBeVisible();

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
