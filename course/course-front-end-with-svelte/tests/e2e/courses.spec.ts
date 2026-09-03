// Playwright smoke suite: each test loads a route and asserts its key
// landmarks render, catching gross routing/mount regressions across the
// dashboard, list, new, match, and merge pages.
//
// The suite carries a stub `__Host-mxi_session` cookie
// (`SMOKE_STORAGE_STATE` in playwright.config.ts), so the pages PRO-H10
// put behind `requireSignedIn` render instead of 303ing to /signin. The
// guard is presence-only, so a stub is enough; the "page-visit guard"
// describe at the bottom drops the cookie and pins the redirect itself.
import { expect, test } from "@playwright/test";

test.describe("Course front-end smoke", () => {
    // Pins: "/" shows the Dashboard heading and the sidebar nav links.
    test("dashboard renders nav and heading", async ({ page }) => {
        await page.goto("/");
        await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
        // The nav is a hamburger dropdown at every viewport width
        // (deliberate layout design) — open it before asserting links.
        await page.getByRole("button", { name: "Toggle navigation" }).click();
        await expect(page.getByRole("link", { name: "Courses" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
    });

    // Pins: "/courses" shows the heading, the search box, and the New course link.
    test("courses list renders search box and new course link", async ({ page }) => {
        await page.goto("/courses");
        await expect(page.getByRole("heading", { name: "Courses" })).toBeVisible();
        await expect(page.getByRole("searchbox")).toBeVisible();
        await expect(page.getByRole("main").getByRole("link", { name: "New course" })).toBeVisible();
    });

    // Pins: "/courses/new" shows the required Name field and the Create button.
    test("new course form renders required name field", async ({ page }) => {
        await page.goto("/courses/new");
        await expect(page.getByRole("heading", { name: "New course" })).toBeVisible();
        await expect(page.getByLabel(/^Name/)).toBeVisible();
        await expect(page.getByRole("button", { name: "Create" })).toBeVisible();
    });

    // Pins: "/courses/match" shows the heading and the Find matches button.
    test("match check form renders", async ({ page }) => {
        await page.goto("/courses/match");
        await expect(page.getByRole("heading", { name: "Match check" })).toBeVisible();
        await expect(page.getByRole("button", { name: /Find matches/ })).toBeVisible();
    });

    // Pins: "/courses/merge" shows the heading and both the Main and Duplicate ID inputs.
    test("merge form renders both ID inputs", async ({ page }) => {
        await page.goto("/courses/merge");
        await expect(page.getByRole("heading", { name: "Merge courses" })).toBeVisible();
        await expect(page.getByLabel(/Main course ID/)).toBeVisible();
        await expect(page.getByLabel(/Duplicate course ID/)).toBeVisible();
    });
});

// Pins the PRO-H10 page-visit guard itself (WEB-1): with NO session
// cookie, every mutation page 303s to /signin rather than rendering a
// form whose submit would fail. This is the one place the smoke suite
// runs anonymous — the stub cookie above would otherwise make a removed
// or broken guard invisible. Read/list/view pages stay public and are
// deliberately not listed here; see AGENTS.md "Page-visit guard".
test.describe("Course front-end page-visit guard", () => {
    test.use({ storageState: { cookies: [], origins: [] } });

    const guarded = [
        "/courses/new",
        "/courses/0c4f1e2a-0000-4000-8000-000000000001/edit",
        "/courses/merge",
    ];

    for (const path of guarded) {
        test(`anonymous visit to ${path} redirects to /signin`, async ({ page }) => {
            await page.goto(path);
            await expect(page).toHaveURL(/\/signin(\?|$)/);
        });
    }
});
