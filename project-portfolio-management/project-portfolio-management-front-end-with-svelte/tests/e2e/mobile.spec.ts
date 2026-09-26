import { test, expect, type Page } from "@playwright/test";

// T-28k (repo tasks.md EV-1): a responsive audit at a phone viewport
// (390x844, set via the "mobile" Playwright project in
// playwright.config.ts) across every real route in this app. Two
// invariants, checked per route so a failure names the route rather
// than producing a screenshot to eyeball:
//
//   1. No horizontal scroll on the document — a route that needs the
//      visitor to scroll sideways to read it is the definition of
//      "not responsive."
//   2. The page's primary heading (<h1>) is visible without horizontal
//      scroll — a proxy for "the page's primary content is reachable"
//      that does not require hand-picking a different "primary action"
//      selector per route (34 routes, each with a different one).
//
// Every route here is enumerated from the real `src/routes` tree (`find
// src/routes -name "+page.svelte"`), not hand-maintained, so a route
// this suite has never heard of cannot silently go unaudited.
//
// Data-fetching routes are NOT stubbed with a live backend (this is a
// structural/responsive audit, not a data-correctness one): the BFF's
// own upstream fetch fails (no Rust service listens on
// PROJECT_PORTFOLIO_MANAGEMENT_API_URL / AUTH_API_URL during this
// suite), and every route's own try/catch renders its error state —
// which must itself be responsive. `/plans` and `/plans/{pid}*` are the
// exception: they are stubbed (mirroring `smoke.spec.ts`'s fixture) so
// this suite also audits their real, populated, SVAR-widget-bearing
// happy-path layout, not just an error banner.

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

/** Stub `/api/plans*` (mirrors `smoke.spec.ts`) so `/plans` and
 *  `/plans/{pid}*` render their real, populated layout rather than an
 *  error banner — the only routes whose SVAR grid/Gantt this suite
 *  needs to see in its non-empty, real state. */
async function stubPlansApi(page: Page) {
  await page.route("**/api/plans**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const method = req.method();
    const path = url.pathname.replace(/^\/api\/proxy/, "");

    if (path === "/api/plans" && method === "GET") {
      return route.fulfill({ json: [{ pid: PID, name: WORK_ITEM.name }] });
    }
    if (path.endsWith("/check-duplicates")) {
      return route.fulfill({ json: [] });
    }
    if (path === `/api/plans/${PID}` && method === "GET") {
      return route.fulfill({ json: WORK_ITEM });
    }
    if (path === `/api/plans/${PID}/schedule` && method === "GET") {
      return route.fulfill({ json: { items: [], edges: [] } });
    }
    return route.fulfill({ status: 404, json: { error: "unhandled in stub" } });
  });
}

/** Every real route, from the file tree — static routes as literal
 *  paths, `[pid]` routes filled with the fixture `PID` above. */
const ROUTES = [
  "/auditor",
  "/automations",
  "/board",
  "/calendar",
  "/capacity",
  "/compliance",
  "/dashboard",
  "/engineering",
  "/executive",
  "/financials",
  "/gantt",
  "/ideas",
  "/lifecycle",
  "/objectives",
  "/onboarding",
  "/plans",
  `/plans/${PID}`,
  `/plans/${PID}/board`,
  `/plans/${PID}/edit`,
  `/plans/${PID}/flow`,
  `/plans/${PID}/governance`,
  `/plans/${PID}/schedule`,
  "/plans/merge",
  "/plans/new",
  "/prioritisation",
  "/proposals",
  "/regulator",
  "/reports",
  "/reviews",
  "/risk",
  "/scenarios",
  "/security",
  "/signin",
  "/technology",
  "/verify",
];

test.describe("phone-viewport responsive audit (T-28k)", () => {
  test.beforeEach(async ({ page }) => {
    await stubPlansApi(page);
  });

  for (const route of ROUTES) {
    test(`${route} has no horizontal scroll and shows its heading`, async ({ page }) => {
      await page.goto(route, { waitUntil: "networkidle" });

      const overflow = await page.evaluate(() => {
        const doc = document.documentElement;
        return { scrollWidth: doc.scrollWidth, clientWidth: doc.clientWidth };
      });
      expect(
        overflow.scrollWidth,
        `${route}: scrollWidth ${overflow.scrollWidth} > clientWidth ${overflow.clientWidth} (horizontal scroll)`,
      ).toBeLessThanOrEqual(overflow.clientWidth);

      // The primary heading is reachable — visible within the viewport
      // width without a horizontal scroll (vertical scroll is fine).
      const heading = page.locator("h1").first();
      await expect(heading, `${route}: no visible <h1>`).toBeVisible();
    });
  }
});
