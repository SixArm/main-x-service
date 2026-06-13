import { test, expect, type Page } from "@playwright/test";

// Smoke tests over the four routes. The backend is stubbed so a broken
// endpoint contract (wrong path / method) surfaces as an unhandled
// request and a failing assertion, without needing the Rust service.

const PID = "11111111-1111-4111-8111-111111111111";

const ORG = {
  name: "Acme, Inc.",
  legal_name: "Acme Incorporated",
  alternate_names: [],
  identifiers: [{ scheme: "Lei", value: "5493001KJTIIGC8Y1R12" }],
  url: "https://acme.example",
  same_as: [],
  address: null,
  jurisdiction: "US",
  founding_date: "1985-04-01",
  telephone: null,
  email: null,
  keywords: ["anvils"],
};

/** Stub every `/api/organizations*` call so the SPA renders offline. */
async function stubApi(page: Page) {
  await page.route("**/api/organizations**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const method = req.method();
    const path = url.pathname;

    if (path === "/api/organizations" && method === "GET") {
      return route.fulfill({ json: [{ pid: PID, name: ORG.name }] });
    }
    if (path === "/api/organizations" && method === "POST") {
      return route.fulfill({ json: { pid: PID, name: ORG.name } });
    }
    if (path.endsWith("/check-duplicates")) {
      return route.fulfill({ json: [] });
    }
    if (path === `/api/organizations/${PID}` && method === "GET") {
      return route.fulfill({ json: ORG });
    }
    if (path === `/api/organizations/${PID}` && method === "PUT") {
      return route.fulfill({ json: { pid: PID, name: ORG.name } });
    }
    if (path === `/api/organizations/${PID}` && method === "DELETE") {
      return route.fulfill({ status: 200, body: "" });
    }
    return route.fulfill({ status: 404, json: { error: "unhandled in stub" } });
  });
}

test.beforeEach(async ({ page }) => {
  await stubApi(page);
});

test("list page renders the seeded organization", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Organizations" }),
  ).toBeVisible();
  await expect(page.getByText("Acme, Inc.")).toBeVisible();
});

test("new page shows the create form", async ({ page }) => {
  await page.goto("/new", { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "New organization" }),
  ).toBeVisible();
});

test("detail page renders the fetched organization", async ({ page }) => {
  await page.goto(`/${PID}`, { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Acme, Inc." })).toBeVisible();
});

test("edit page renders the edit form", async ({ page }) => {
  await page.goto(`/${PID}/edit`, { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Edit organization" }),
  ).toBeVisible();
});
