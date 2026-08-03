import { test, expect, type Page } from "@playwright/test";

// Smoke tests over the collection routes. The backend is stubbed so a
// broken endpoint contract (wrong path / method / field) surfaces as
// an unhandled request and a failing assertion, without needing the
// Rust service. (Fixed 2026-07-18: these were stale copies of the
// case front-end's suite — asserting "Cases" headings, stubbing
// `title` instead of the work-item `name`, and using the case app's
// detail routes — and had never been adapted to this app.)

// Fixed pid + canned work item used by stubs and assertions alike.
const PID = "11111111-1111-4111-8111-111111111111";

const WORK_ITEM = {
  kind: "Project",
  name: "Website replatform",
  alternate_names: [],
  code: "WEB-42",
  status: "Active",
  goals: [],
  keywords: ["platform"],
  tags: [],
  identifiers: [],
  relationships: [],
  same_as: [],
  in_language: ["en"],
};

/** Stub every `/api/projects*` call so the SPA renders offline. */
async function stubApi(page: Page) {
  await page.route("**/api/projects**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const method = req.method();
    const path = url.pathname;

    // Dispatch by (path, method) mirroring the real endpoint contract; any
    // unmatched request falls through to a 404 so contract drift fails loud.
    if (path === "/api/projects" && method === "GET") {
      return route.fulfill({ json: [{ pid: PID, name: WORK_ITEM.name }] });
    }
    if (path === "/api/projects" && method === "POST") {
      return route.fulfill({ json: { pid: PID, name: WORK_ITEM.name } });
    }
    if (path.endsWith("/check-duplicates")) {
      return route.fulfill({ json: [] });
    }
    if (path === `/api/projects/${PID}` && method === "GET") {
      return route.fulfill({ json: WORK_ITEM });
    }
    if (path === `/api/projects/${PID}` && method === "PUT") {
      return route.fulfill({ json: { pid: PID, name: WORK_ITEM.name } });
    }
    if (path === `/api/projects/${PID}` && method === "DELETE") {
      return route.fulfill({ status: 200, body: "" });
    }
    return route.fulfill({ status: 404, json: { error: "unhandled in stub" } });
  });
}

test.beforeEach(async ({ page }) => {
  await stubApi(page);
});

// Pins: the list route fetches and shows the seeded work item.
test("list page renders the seeded case", async ({ page }) => {
  await page.goto("/projects", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: /Work items/ })).toBeVisible();
  await expect(page.getByText("Website replatform")).toBeVisible();
});

// Pins: the create route renders the empty form.
test("new page shows the create form", async ({ page }) => {
  await page.goto("/projects/new", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: /New case/ })).toBeVisible();
});

// Pins: the detail route fetches the work item and shows its name.
test("detail page renders the fetched case", async ({ page }) => {
  await page.goto(`/projects/${PID}`, { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Website replatform" }),
  ).toBeVisible();
  // The governance panel is linked from the detail page (PPM views).
  await expect(page.getByRole("link", { name: "Governance" })).toBeVisible();
});

// Pins: the edit route loads the work item and renders the edit form.
test("edit page renders the edit form", async ({ page }) => {
  await page.goto(`/projects/${PID}/edit`, { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: /Edit case/ })).toBeVisible();
});

// Pins: the merge route renders its form (both pid inputs) and reads the
// merge history. The history endpoint is stubbed empty so the table shows
// its empty row rather than failing the load.
test("merge page renders the form and the recent-merges table", async ({
  page,
}) => {
  await page.route("**/api/plans/merges/recent", async (route) =>
    route.fulfill({ json: [] }),
  );
  await page.goto("/plans/merge", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Merge plans" })).toBeVisible();
  await expect(page.locator("#merge-main")).toBeVisible();
  await expect(page.locator("#merge-dup")).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Recent merges" }),
  ).toBeVisible();
  await expect(page.getByText("No merges recorded yet.")).toBeVisible();
});

// Pins: the merge destination is reachable from the nav on another page
// (the nav lives behind the hamburger at every width).
test("nav exposes the merge link", async ({ page }) => {
  await page.goto("/plans", { waitUntil: "networkidle" });
  await page.getByRole("button", { name: /Toggle navigation/i }).click();
  await expect(
    page.getByRole("link", { name: "Merge", exact: true }),
  ).toBeVisible();
});

// Pins: spec §6.6 — check-duplicates excludes the record itself. The stub
// returns the current record (same pid) plus one other hit; only the other
// must surface in the "Potential duplicates" list.
test("detail check-duplicates hides the record itself (self-exclusion)", async ({
  page,
}) => {
  const OTHER = "22222222-2222-4222-8222-222222222222";
  await page.route("**/api/projects/check-duplicates", async (route) =>
    route.fulfill({
      json: [
        // The record itself — must be filtered out (h.pid === pid).
        {
          pid: PID,
          name: WORK_ITEM.name,
          score: 1.0,
          confidence: "Certain",
          is_match: true,
        },
        // A genuine other candidate — must remain.
        {
          pid: OTHER,
          name: "Website rebuild",
          score: 0.91,
          confidence: "Probable",
          is_match: true,
        },
      ],
    }),
  );
  await page.goto(`/projects/${PID}`, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Check duplicates" }).click();
  await expect(
    page.getByRole("heading", { name: "Potential duplicates" }),
  ).toBeVisible();
  // The other candidate is listed.
  await expect(page.getByText("Website rebuild")).toBeVisible();
  // The record itself is not echoed back as its own duplicate. Scope to the
  // duplicates list so the page's own <h1> title is not matched.
  await expect(page.locator("h2 ~ ul a")).toHaveCount(1);
  await expect(page.locator("h2 ~ ul a")).toHaveText("Website rebuild");
});
