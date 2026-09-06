import { expect, test } from "@playwright/test";
import { readFileSync } from "node:fs";

// Smoke tests that assert the page shell renders. They do NOT require a
// running Worker Service — failures from the API call are swallowed by
// the page and shown as banners, but the layout still renders. Run with
// the service started (`docker-compose up` in worker-service-with-loco)
// for full coverage of the API-driven paths.
//
// The suite carries a stub `__Host-mxi_session` cookie
// (`SMOKE_STORAGE_STATE` in playwright.config.ts), so the pages PRO-H10
// put behind `requireSignedIn` render instead of 303ing to /signin. The
// guard is presence-only, so a stub is enough; the "page-visit guard"
// describe at the bottom drops the cookie and pins the redirect itself.

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

    // Pins T-28: the credential-expiry calendar (`/expiry`) renders and its
    // one calendar event navigates to the owning worker's detail page.
    // Previously the only route in the map with zero e2e coverage.
    test("expiry calendar renders and navigates to the worker on select", async ({
        page,
    }) => {
        const id = "0c4f1e2a-0000-4000-8000-00000000000e";
        // The Calendar's month view defaults to the current month, so the
        // fixture's expiry date must fall inside it (a fixed future date
        // would eventually drift out of view and stop rendering the event).
        const today = new Date();
        const dayInThisMonth = new Date(today.getFullYear(), today.getMonth(), 15)
            .toISOString()
            .slice(0, 10);
        const worker = {
            id,
            name: { family: "Bennet", given: ["Jane"] },
            gender: "female",
            documents: [
                {
                    document_type: "Passport",
                    number: "X1234567",
                    expiry_date: dayInThisMonth,
                },
            ],
        };
        await page.route("**/api/workers/search**", async (route) => {
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify({ success: true, data: [worker], error: null }),
            });
        });
        await page.route(`**/api/workers/${id}`, async (route) => {
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify({ success: true, data: worker, error: null }),
            });
        });

        await page.goto("/expiry");
        await expect(page.getByRole("heading", { name: "Expiries" })).toBeVisible();
        const calendar = page.getByTestId("expiry-calendar");
        await expect(calendar).toBeVisible();
        // total === items.length (1) here: no truncation notice.
        await expect(page.getByTestId("expiry-truncation-notice")).toBeHidden();

        // The one event this fixture produces: "Bennet — Passport".
        await calendar.getByText("Bennet", { exact: false }).click();
        await expect(page).toHaveURL(`/workers/${id}`);
    });

    // Pins T-29: `/expiry`'s search is a capped window (limit 200), not a
    // promise of completeness. When the service reports more workers than
    // fit in that window (`total > items.length`), a truncation notice
    // must say so rather than silently showing a partial calendar.
    test("expiry calendar shows a truncation notice when the window is partial", async ({
        page,
    }) => {
        const id = "0c4f1e2a-0000-4000-8000-00000000000f";
        const today = new Date();
        const dayInThisMonth = new Date(today.getFullYear(), today.getMonth(), 16)
            .toISOString()
            .slice(0, 10);
        const worker = {
            id,
            name: { family: "Osei", given: ["Ama"] },
            gender: "female",
            documents: [
                {
                    document_type: "Passport",
                    number: "Y7654321",
                    expiry_date: dayInThisMonth,
                },
            ],
        };
        await page.route("**/api/workers/search**", async (route) => {
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify({
                    success: true,
                    data: { items: [worker], total: 250 },
                    error: null,
                }),
            });
        });

        await page.goto("/expiry");
        await expect(page.getByRole("heading", { name: "Expiries" })).toBeVisible();
        const notice = page.getByTestId("expiry-truncation-notice");
        await expect(notice).toBeVisible();
        await expect(notice).toHaveText("Showing up to 1 of 250 workers");
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

    // Pins: the masked-view toggle (T-19) re-fetches through
    // GET /api/workers/{id}/masked and back, rather than redacting
    // client-side — the two stubs return visibly different tax_id values
    // so the test can tell which endpoint actually answered.
    test("worker detail toggles between the plain and masked view", async ({
        page,
    }) => {
        const id = "0c4f1e2a-0000-4000-8000-00000000000a";
        const base = {
            id,
            name: { family: "Smith", given: ["John"] },
            gender: "male",
            active: true,
        };
        await page.route("**/api/workers/**", async (route) => {
            const url = route.request().url();
            const envelope = url.includes("/links")
                ? { success: true, data: [], error: null }
                : url.includes("/masked")
                  ? { success: true, data: { ...base, tax_id: "***-**-****" }, error: null }
                  : { success: true, data: { ...base, tax_id: "123-45-6789" }, error: null };
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify(envelope),
            });
        });

        await page.goto(`/workers/${id}`);
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

    // Pins: the GDPR export button (T-20) fetches GET /api/workers/{id}/export
    // and saves what came back as `worker-<id>-export.json` — a real browser
    // download (Blob object URL + synthetic anchor), asserted through
    // Playwright's download event, with the saved bytes compared to the
    // stubbed payload so a silently-empty file cannot pass.
    test("worker detail downloads the GDPR export as JSON", async ({ page }) => {
        const id = "0c4f1e2a-0000-4000-8000-0000000000cc";
        const payload = { subject: id, exported_at: "2026-09-03T00:00:00Z", records: [] };
        const record = { id, name: { family: "Smith", given: ["John"] }, gender: "male", active: true };
        await page.route("**/api/workers/**", async (route) => {
            const url = route.request().url();
            const envelope = url.includes("/links")
                ? { success: true, data: [], error: null }
                : url.includes("/export")
                  ? { success: true, data: payload, error: null }
                  : { success: true, data: record, error: null };
            await route.fulfill({
                status: 200,
                contentType: "application/json",
                body: JSON.stringify(envelope),
            });
        });

        await page.goto(`/workers/${id}`);
        const downloadPromise = page.waitForEvent("download");
        await page.getByRole("button", { name: "Export data (GDPR)" }).click();
        const download = await downloadPromise;
        expect(download.suggestedFilename()).toBe(`worker-${id}-export.json`);
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
test.describe("Worker front-end page-visit guard", () => {
    test.use({ storageState: { cookies: [], origins: [] } });

    const guarded = [
        "/workers/new",
        "/workers/0c4f1e2a-0000-4000-8000-000000000001/edit",
        "/workers/merge",
        "/review",
    ];

    for (const path of guarded) {
        test(`anonymous visit to ${path} redirects to /signin`, async ({ page }) => {
            await page.goto(path);
            await expect(page).toHaveURL(/\/signin(\?|$)/);
        });
    }
});
