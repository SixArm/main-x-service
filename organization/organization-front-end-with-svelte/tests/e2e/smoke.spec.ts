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
    if (path.endsWith("/review-queue") && method === "GET") {
      return route.fulfill({
        json: {
          items: [
            {
              id: "rq-1",
              organization_id_a: PID,
              organization_id_b: "22222222-2222-4222-8222-222222222222",
              match_score: 0.91,
              match_quality: "high",
              detection_method: "batch_deduplication",
              status: "pending",
              reviewed_by: null,
              created_at: "2026-07-19T00:00:00Z",
              reviewed_at: null,
            },
          ],
          total: 1,
        },
      });
    }
    if (path.endsWith("/review-queue/rq-1/decision") && method === "POST") {
      return route.fulfill({
        json: {
          id: "rq-1",
          organization_id_a: PID,
          organization_id_b: "22222222-2222-4222-8222-222222222222",
          match_score: 0.91,
          match_quality: "high",
          detection_method: "batch_deduplication",
          status: "confirmed",
          reviewed_by: "tester",
          created_at: "2026-07-19T00:00:00Z",
          reviewed_at: "2026-07-19T00:01:00Z",
        },
      });
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

test("review board renders the stored queue on load (no scan side effect)", async ({
  page,
}) => {
  const scans: string[] = [];
  page.on("request", (req) => {
    if (req.url().includes("/deduplicate")) scans.push(req.url());
  });
  await page.goto("/review", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Review" })).toBeVisible();
  await expect(page.getByTestId("review-board")).toBeVisible();
  // The stored pending card is on the board, described by quality,
  // score, and detection method.
  await expect(page.getByText("high · 0.91 · batch_deduplication")).toBeVisible();
  // Loading the page must never fire the destructive-classed scan.
  expect(scans).toHaveLength(0);
});
