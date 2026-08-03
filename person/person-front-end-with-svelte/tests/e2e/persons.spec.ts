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

    // Pins: the merge page renders both the main and duplicate id inputs.
    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/persons/merge");
        await expect(page.getByRole("heading", { name: "Merge persons" })).toBeVisible();
        await expect(page.getByLabel(/Main person ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate person ID/)).toBeVisible();
    });
});
