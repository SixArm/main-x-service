import { test, expect, type Page } from "@playwright/test";

// Smoke tests over the four routes. The backend is stubbed so a broken
// endpoint contract (wrong path / method) surfaces as an unhandled
// request and a failing assertion, without needing the Rust service.

// Fixed pid + canned case used by both the route stubs and the assertions.
const PID = "11111111-1111-4111-8111-111111111111";

const CASE = {
  title: "Housing benefit appeal",
  case_number: "2026-HB-0042",
  agency_id: "agency-1",
  agency_name: "City Housing Office",
  case_type: "Housing",
  status: "Open",
  priority: "Normal",
  opened_date: "2026-01-15",
  alternate_titles: [],
  subjects: ["claimant-7"],
  keywords: ["housing"],
  identifiers: [{ scheme: "Docket", value: "HB-2026-0042" }],
  same_as: [],
  in_language: ["en"],
};

/** Stub every `/api/cases*` call so the SPA renders offline. */
async function stubApi(page: Page) {
  await page.route("**/api/cases**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const method = req.method();
    const path = url.pathname;

    // Dispatch by (path, method) mirroring the real endpoint contract; any
    // unmatched request falls through to a 404 so contract drift fails loud.
    if (path === "/api/cases" && method === "GET") {
      return route.fulfill({ json: [{ pid: PID, title: CASE.title }] });
    }
    if (path === "/api/cases" && method === "POST") {
      return route.fulfill({ json: { pid: PID, title: CASE.title } });
    }
    if (path.endsWith("/check-duplicates")) {
      return route.fulfill({ json: [] });
    }
    if (path === `/api/cases/${PID}` && method === "GET") {
      return route.fulfill({ json: CASE });
    }
    if (path === `/api/cases/${PID}` && method === "PUT") {
      return route.fulfill({ json: { pid: PID, title: CASE.title } });
    }
    if (path === `/api/cases/${PID}` && method === "DELETE") {
      return route.fulfill({ status: 200, body: "" });
    }
    return route.fulfill({ status: 404, json: { error: "unhandled in stub" } });
  });
}

test.beforeEach(async ({ page }) => {
  await stubApi(page);
});

// Pins: the list route fetches and shows the seeded case title.
test("list page renders the seeded case", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Cases" })).toBeVisible();
  await expect(page.getByText("Housing benefit appeal")).toBeVisible();
});

// Pins: the create route renders the empty form.
test("new page shows the create form", async ({ page }) => {
  await page.goto("/new", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "New case" })).toBeVisible();
});

// Pins: the detail route fetches the case and shows its title heading.
test("detail page renders the fetched case", async ({ page }) => {
  await page.goto(`/${PID}`, { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Housing benefit appeal" }),
  ).toBeVisible();
});

// Pins: the edit route loads the case and renders the edit form.
test("edit page renders the edit form", async ({ page }) => {
  await page.goto(`/${PID}/edit`, { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Edit case" })).toBeVisible();
});
