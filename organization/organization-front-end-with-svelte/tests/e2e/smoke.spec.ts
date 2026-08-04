import { test, expect, type Page } from "@playwright/test";

// Smoke tests over the four routes. The backend is stubbed so a broken
// endpoint contract (wrong path / method) surfaces as an unhandled
// request and a failing assertion, without needing the Rust service.

const PID = "11111111-1111-4111-8111-111111111111";
const DUP_PID = "22222222-2222-4222-8222-222222222222";

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

// The review board's other half of the stubbed pending pair: close enough
// to ORG that a real matcher would flag it, distinct enough to render
// differently in the comparison panel.
const ORG_DUP = {
  name: "Acme Inc",
  legal_name: "Acme Incorporated",
  alternate_names: [],
  identifiers: [],
  url: "https://acme.example",
  same_as: [],
  address: null,
  jurisdiction: "US",
  founding_date: "1985-04-01",
  telephone: null,
  email: null,
  keywords: ["anvils", "hardware"],
};

/** Stub every `/api/organizations*` call so the SPA renders offline. */
async function stubApi(page: Page) {
  await page.route("**/api/organizations**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const method = req.method();
    // The browser calls the same-origin BFF proxy (`/api/proxy/...`), which
    // forwards to the organization service — so a request's real `pathname`
    // is `/api/proxy/api/organizations`, not `/api/organizations`. Strip the
    // proxy prefix before dispatching so the (path, method) comparisons
    // below match what the browser actually sends.
    const path = url.pathname.replace(/^\/api\/proxy/, "");

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
              organization_id_b: DUP_PID,
              match_score: 0.91,
              match_quality: "high",
              detection_method: "batch_deduplication",
              status: "pending",
              provenance: "import",
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
          organization_id_b: DUP_PID,
          match_score: 0.91,
          match_quality: "high",
          detection_method: "batch_deduplication",
          status: "confirmed",
          provenance: "import",
          reviewed_by: "tester",
          created_at: "2026-07-19T00:00:00Z",
          reviewed_at: "2026-07-19T00:01:00Z",
        },
      });
    }
    if (path.endsWith("/match") && method === "POST") {
      return route.fulfill({
        json: [
          [
            0,
            {
              score: 0.91,
              is_match: true,
              confidence: "High",
              breakdown: {
                name_score: 0.94,
                address_score: null,
                url_score: 1.0,
                jurisdiction_score: 1.0,
                founding_date_score: 1.0,
                keywords_score: 0.5,
                deterministic_match: false,
              },
            },
          ],
        ],
      });
    }
    if (path.endsWith("/merges/recent") && method === "GET") {
      return route.fulfill({
        json: [
          {
            created_at: "2026-08-01T09:30:00Z",
            updated_at: "2026-08-01T09:30:00Z",
            id: 1,
            main_pid: PID,
            duplicate_pid: "33333333-3333-4333-8333-333333333333",
            reason: "same company, duplicate registration",
            actor: "tester",
            transferred: null,
          },
        ],
      });
    }
    if (path.endsWith("/merge") && method === "POST") {
      return route.fulfill({
        json: {
          main_pid: PID,
          duplicate_pid: "33333333-3333-4333-8333-333333333333",
          main: ORG,
        },
      });
    }
    if (path === `/api/organizations/${PID}` && method === "GET") {
      return route.fulfill({ json: ORG });
    }
    if (path === `/api/organizations/${DUP_PID}` && method === "GET") {
      return route.fulfill({ json: ORG_DUP });
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
  // score, detection method, and provenance.
  await expect(
    page.getByText("high · 0.91 · batch_deduplication · Import"),
  ).toBeVisible();
  // Loading the page must never fire the destructive-classed scan.
  expect(scans).toHaveLength(0);
});

test("review board offers a keyboard-reachable table with the same item", async ({
  page,
}) => {
  await page.goto("/review", { waitUntil: "networkidle" });
  const list = page.getByTestId("review-list");
  await expect(list).toBeVisible();
  // Provenance is visible at a glance in the queue, not only in the
  // detail panel.
  await expect(list.getByText("Import")).toBeVisible();
  await expect(
    list.getByRole("button", { name: /^Compare/ }),
  ).toBeVisible();
});

test("review board compares a pair on demand with a live score breakdown", async ({
  page,
}) => {
  const matchCalls: string[] = [];
  page.on("request", (req) => {
    if (req.method() === "POST" && req.url().endsWith("/match")) {
      matchCalls.push(req.url());
    }
  });
  await page.goto("/review", { waitUntil: "networkidle" });

  // The keyboard-reachable path into the comparison: a real button, not
  // a drag.
  await page
    .getByTestId("review-list")
    .getByRole("button", { name: /^Compare/ })
    .click();

  const panel = page.getByTestId("review-compare");
  await expect(panel).toBeVisible();
  // Both stubbed records, side by side. Exact match: "Acme Inc" is a
  // substring of "Acme Incorporated", which also appears in this panel.
  await expect(panel.getByText("Acme, Inc.")).toBeVisible();
  await expect(panel.getByText("Acme Inc", { exact: true })).toBeVisible();

  // The stored review item never carries `score_breakdown` on the wire
  // (see `$lib/review`'s doc comment) — the panel fetches a live one from
  // `POST /api/organizations/match` instead.
  await expect(matchCalls).toHaveLength(1);
  const breakdown = page.getByTestId("review-breakdown");
  await expect(breakdown).toBeVisible();
  await expect(breakdown.getByText("0.94")).toBeVisible();
  // `address_score: null` in the stub is omitted, not shown as 0.00.
  await expect(breakdown.getByText("Address")).not.toBeVisible();

  // Both decisions are offered as real buttons for a pending item.
  await expect(
    panel.getByRole("button", { name: "Confirm duplicate" }),
  ).toBeEnabled();
  await expect(panel.getByRole("button", { name: "Reject" })).toBeEnabled();
});

test("merge page shows both id fields and the merge history", async ({
  page,
}) => {
  const merges: string[] = [];
  page.on("request", (req) => {
    if (req.method() === "POST" && req.url().endsWith("/merge")) {
      merges.push(req.url());
    }
  });
  await page.goto("/merge", { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Merge organizations" }),
  ).toBeVisible();
  await expect(page.locator("#merge-main")).toBeVisible();
  await expect(page.locator("#merge-dup")).toBeVisible();
  // The history is a safe GET and loads on mount.
  await expect(page.getByTestId("merge-recent")).toBeVisible();
  await expect(
    page.getByText("same company, duplicate registration"),
  ).toBeVisible();
  // Loading the page must never fire the destructive merge.
  expect(merges).toHaveLength(0);
});

test("merge page pre-fills both IDs from the query string", async ({
  page,
}) => {
  // What the review board's confirmed-item deep link relies on
  // (`$lib/review`'s `mergeHref`).
  await page.goto(`/merge?main=${PID}&duplicate=${DUP_PID}`, {
    waitUntil: "networkidle",
  });
  await expect(page.locator("#merge-main")).toHaveValue(PID);
  await expect(page.locator("#merge-dup")).toHaveValue(DUP_PID);
});

test("the nav offers Merge from another page", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });
  // The primary nav is always collapsed behind the hamburger.
  await page.getByRole("button", { name: "Toggle navigation" }).click();
  await expect(page.getByRole("link", { name: "Merge" })).toBeVisible();
});
