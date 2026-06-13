import { test, expect, type Page } from "@playwright/test";

// Smoke tests over the four routes. The backend is stubbed so a broken
// endpoint contract (wrong path / method) surfaces as an unhandled
// request and a failing assertion, without needing the Rust service.

const PID = "11111111-1111-4111-8111-111111111111";

const PATHWAY = {
  name: "Acute Stroke Care Pathway",
  pathway_code: "STROKE-01",
  provider_id: "trust-1",
  alternate_names: [],
  condition_codes: [{ system: "Icd10", code: "I63.9" }],
  interventions: [],
  keywords: ["stroke"],
  identifiers: [{ scheme: "GuidelineId", value: "NICE-NG128" }],
  same_as: [],
  in_language: [],
};

/** Stub every `/api/care-pathways*` call so the SPA renders offline. */
async function stubApi(page: Page) {
  await page.route("**/api/care-pathways**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const method = req.method();
    const path = url.pathname;

    if (path === "/api/care-pathways/search" && method === "GET") {
      const q = (url.searchParams.get("q") ?? "").toLowerCase();
      const hit = PATHWAY.name.toLowerCase().includes(q);
      return route.fulfill({
        json: hit ? [{ pid: PID, name: PATHWAY.name }] : [],
      });
    }
    if (path === "/api/care-pathways" && method === "GET") {
      return route.fulfill({ json: [{ pid: PID, name: PATHWAY.name }] });
    }
    if (path === "/api/care-pathways" && method === "POST") {
      return route.fulfill({ json: { pid: PID, name: PATHWAY.name } });
    }
    if (path.endsWith("/check-duplicates")) {
      return route.fulfill({ json: [] });
    }
    if (path === `/api/care-pathways/${PID}` && method === "GET") {
      return route.fulfill({ json: PATHWAY });
    }
    if (path === `/api/care-pathways/${PID}` && method === "PUT") {
      return route.fulfill({ json: { pid: PID, name: PATHWAY.name } });
    }
    if (path === `/api/care-pathways/${PID}` && method === "DELETE") {
      return route.fulfill({ status: 200, body: "" });
    }
    return route.fulfill({ status: 404, json: { error: "unhandled in stub" } });
  });
}

test.beforeEach(async ({ page }) => {
  await stubApi(page);
});

test("list page renders the seeded pathway", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Care pathways" }),
  ).toBeVisible();
  await expect(page.getByText("Acute Stroke Care Pathway")).toBeVisible();
});

test("search box filters the list via the search endpoint", async ({
  page,
}) => {
  await page.goto("/", { waitUntil: "networkidle" });
  const box = page.getByRole("searchbox", {
    name: "Search care pathways by name",
  });

  // A matching query keeps the seeded pathway.
  await box.fill("stroke");
  await page.getByRole("button", { name: "Search" }).click();
  await expect(page.getByText("Acute Stroke Care Pathway")).toBeVisible();

  // A non-matching query yields the empty-result message.
  await box.fill("nomatch");
  await page.getByRole("button", { name: "Search" }).click();
  await expect(page.getByText(/No care pathways match/)).toBeVisible();
});

test("new page shows the create form", async ({ page }) => {
  await page.goto("/new", { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "New care pathway" }),
  ).toBeVisible();
});

test("detail page renders the fetched pathway", async ({ page }) => {
  await page.goto(`/${PID}`, { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Acute Stroke Care Pathway" }),
  ).toBeVisible();
});

test("edit page renders the edit form", async ({ page }) => {
  await page.goto(`/${PID}/edit`, { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Edit care pathway" }),
  ).toBeVisible();
});
