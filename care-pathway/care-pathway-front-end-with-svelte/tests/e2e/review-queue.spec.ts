// Duplicate review queue (/review-queue, T-10) smoke test over a
// page.route-stubbed API (mirroring the service contract; unmatched
// calls 404 so drift fails loud). No Rust service needed.

import { test, expect, type Page } from "@playwright/test";

const PID_A = "11111111-1111-4111-8111-111111111111";
const PID_B = "22222222-2222-4222-8222-222222222222";
const ITEM_ID = "33333333-3333-4333-8333-333333333333";

interface StubReviewItem {
  id: string;
  pathway_id_a: string;
  pathway_id_b: string;
  match_score: number;
  match_quality: string;
  detection_method: string;
  score_breakdown: null;
  status: string;
  provenance: string;
  reviewed_by: string | null;
  created_at: string;
  reviewed_at: string | null;
}

const PENDING_ITEM: StubReviewItem = {
  id: ITEM_ID,
  pathway_id_a: PID_A,
  pathway_id_b: PID_B,
  match_score: 0.82,
  match_quality: "high",
  detection_method: "import_duplicate_detection",
  score_breakdown: null,
  status: "pending",
  provenance: "import",
  reviewed_by: null,
  created_at: "2026-09-13T00:00:00Z",
  reviewed_at: null,
};

/** The queue mutated by the stub as decisions land, so a re-fetch after
 *  a decision reflects it (no server round trip needed to keep state). */
let queue: StubReviewItem[] = [];

async function stubReviewQueue(page: Page) {
  queue = [PENDING_ITEM];
  await page.route("**/api/**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const path = url.pathname.startsWith("/api/proxy")
      ? url.pathname.slice("/api/proxy".length)
      : url.pathname;
    const method = req.method();
    const json = (body: unknown, status = 200) =>
      route.fulfill({ status, json: body });

    if (path === "/api/care-pathways/review-queue" && method === "GET") {
      const status = url.searchParams.get("status");
      const items = status ? queue.filter((i) => i.status === status) : queue;
      return json({ items, total: items.length });
    }
    if (
      path === `/api/care-pathways/review-queue/${ITEM_ID}/decision` &&
      method === "POST"
    ) {
      const body = req.postDataJSON() as { status: string };
      queue = queue.map((i) =>
        i.id === ITEM_ID
          ? {
              ...i,
              status: body.status,
              reviewed_by: "operator-1",
              reviewed_at: "2026-09-13T01:00:00Z",
            }
          : i,
      );
      return json(queue.find((i) => i.id === ITEM_ID));
    }
    return json({ error: "unhandled in stub", path }, 404);
  });
}

test.describe("duplicate review queue", () => {
  test("lists a pending pair with links to both pathways", async ({ page }) => {
    await stubReviewQueue(page);
    await page.goto("/review-queue");
    await expect(
      page.getByRole("heading", { name: /Duplicate review queue/ }),
    ).toBeVisible();

    await expect(page.locator(`a[href="/${PID_A}"]`)).toBeVisible();
    await expect(page.locator(`a[href="/${PID_B}"]`)).toBeVisible();
    await expect(page.getByText("82% (high)")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Confirm duplicate" }),
    ).toBeVisible();
    await expect(page.getByRole("button", { name: "Reject" })).toBeVisible();
  });

  test("confirming a pair records the reviewer and removes it from the pending filter", async ({
    page,
  }) => {
    await stubReviewQueue(page);
    await page.goto("/review-queue");

    await page.getByRole("button", { name: "Confirm duplicate" }).click();

    // Under the default "Pending" filter, a decided row drops out of view
    // rather than lingering with stale action buttons.
    await expect(
      page.getByText("No candidate pairs match this filter."),
    ).toBeVisible();

    // Switching to "Confirmed" shows it, with the reviewer recorded and
    // no action buttons (it is no longer pending).
    await page.getByLabel("Status").selectOption("confirmed");
    await expect(page.getByText("operator-1")).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Confirm duplicate" }),
    ).toHaveCount(0);
  });
});
