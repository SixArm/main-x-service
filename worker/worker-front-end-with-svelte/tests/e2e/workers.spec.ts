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
});
