import { test, expect, type Page } from "@playwright/test";

// Smoke tests over the collection routes. The backend is stubbed so a
// broken endpoint contract (wrong path / method / field) surfaces as
// an unhandled request and a failing assertion, without needing the
// Rust service. (Fixed 2026-07-18: these were stale copies of the
// case front-end's suite — asserting "Cases" headings, stubbing
// `title` instead of the work-item `name`, and using the case app's
// detail routes — and had never been adapted to this app.)
//
// Fixed 2026-08-04 (DOC-4 audit): this file was never updated for the
// 2026-07-20 route unification (the four `/portfolios`, `/projects`,
// `/products`, `/programs` collections became one `/plans` collection) —
// every test here still navigated to `/projects*`, which has not existed
// for two weeks, so 5 of 7 tests failed. `pnpm test` / `pnpm run check` /
// `pnpm run build` never caught it because none of them runs Playwright;
// only `pnpm test:e2e` does. Also applied the BFF-proxy-prefix fix
// (ad95088e): the browser's real request is `/api/proxy/api/...`, not
// the bare `/api/...` this stub matched on.

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

/** Stub every `/api/plans*` call so the SPA renders offline. */
async function stubApi(page: Page) {
  await page.route("**/api/plans**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const method = req.method();
    // Strip the BFF proxy prefix (ad95088e): the browser's real request is
    // `/api/proxy/api/plans...`, not the bare `/api/plans...` below.
    const path = url.pathname.replace(/^\/api\/proxy/, "");

    // Dispatch by (path, method) mirroring the real endpoint contract; any
    // unmatched request falls through to a 404 so contract drift fails loud.
    if (path === "/api/plans" && method === "GET") {
      return route.fulfill({ json: [{ pid: PID, name: WORK_ITEM.name }] });
    }
    if (path === "/api/plans" && method === "POST") {
      return route.fulfill({ json: { pid: PID, name: WORK_ITEM.name } });
    }
    if (path.endsWith("/check-duplicates")) {
      return route.fulfill({ json: [] });
    }
    if (path === `/api/plans/${PID}` && method === "GET") {
      return route.fulfill({ json: WORK_ITEM });
    }
    if (path === `/api/plans/${PID}` && method === "PUT") {
      return route.fulfill({ json: { pid: PID, name: WORK_ITEM.name } });
    }
    if (path === `/api/plans/${PID}` && method === "DELETE") {
      return route.fulfill({ status: 200, body: "" });
    }
    return route.fulfill({ status: 404, json: { error: "unhandled in stub" } });
  });
}

test.beforeEach(async ({ page }) => {
  await stubApi(page);
});

// Pins: the list route fetches and shows the seeded plan. Heading text is
// "Plans" (`list.title`) — the i18n catalogue still carries a handful of
// unrelated "case"/"work item" leftovers from the copy-adapt source
// (`nav.cases`, `list.loadFailed: "Failed to load cases"`, …), but none of
// those keys are rendered by these routes.
test("list page renders the seeded plan", async ({ page }) => {
  await page.goto("/plans", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Plans" })).toBeVisible();
  await expect(page.getByText("Website replatform")).toBeVisible();
});

// Pins: the create route renders the empty form. Heading text is "New
// case" (`new.title`) — a real, if oddly-worded, i18n value; not a typo
// in this test.
test("new page shows the create form", async ({ page }) => {
  await page.goto("/plans/new", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: /New case/ })).toBeVisible();
});

// Pins: the detail route fetches the plan and shows its name.
test("detail page renders the fetched plan", async ({ page }) => {
  await page.goto(`/plans/${PID}`, { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Website replatform" }),
  ).toBeVisible();
  // The governance panel is linked from the detail page (PPM views).
  await expect(page.getByRole("link", { name: "Governance" })).toBeVisible();
});

// Pins: the edit route loads the plan and renders the edit form. Heading
// text is "Edit case" (`edit.title`) — see the `new.title` note above.
test("edit page renders the edit form", async ({ page }) => {
  await page.goto(`/plans/${PID}/edit`, { waitUntil: "networkidle" });
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
  await page.route("**/api/plans/check-duplicates", async (route) =>
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
  await page.goto(`/plans/${PID}`, { waitUntil: "networkidle" });
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
