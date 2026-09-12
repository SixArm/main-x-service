// Native bulk import/export (/bulk, §13 T-10) smoke test over a
// page.route-stubbed API (mirroring the service contract; unmatched
// calls 404 so drift fails loud). No Rust service needed.

import { test, expect, type Page } from "@playwright/test";

const IMPORT_JOB = "11111111-1111-4111-8111-111111111111";
const EXPORT_JOB = "22222222-2222-4222-8222-222222222222";

const IMPORT_JOB_VIEW = {
  id: IMPORT_JOB,
  kind: "import",
  entity: "care_pathway",
  format: "jsonl",
  status: "completed",
  rows_total: 3,
  rows_processed: 3,
  rows_created: 2,
  rows_upserted: 1,
  rows_to_review: 0,
  rows_errored: 0,
  download_url: null,
  errors_url: null,
};

const EXPORT_JOB_VIEW = {
  id: EXPORT_JOB,
  kind: "export",
  entity: "care_pathway",
  format: "jsonl",
  status: "completed",
  rows_total: 5,
  rows_processed: 5,
  rows_created: 0,
  rows_upserted: 0,
  rows_to_review: 0,
  rows_errored: 0,
  download_url: "file:///tmp/care-pathway-bulk-artifacts/jobs/2/export.jsonl",
  errors_url: null,
};

/** Recent-jobs list mutated by the stub as jobs are submitted, so
 *  `GET .../bulk-jobs` reflects what the test just did. */
let jobs: Array<typeof IMPORT_JOB_VIEW | typeof EXPORT_JOB_VIEW> = [];

async function stubBulk(page: Page) {
  jobs = [];
  await page.route("**/api/**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const path = url.pathname.startsWith("/api/proxy")
      ? url.pathname.slice("/api/proxy".length)
      : url.pathname;
    const method = req.method();
    const json = (body: unknown, status = 200) =>
      route.fulfill({ status, json: body });

    if (path === "/api/care-pathways/import" && method === "POST") {
      jobs = [IMPORT_JOB_VIEW, ...jobs];
      return json({ job_id: IMPORT_JOB }, 202);
    }
    if (path === "/api/care-pathways/export" && method === "POST") {
      jobs = [EXPORT_JOB_VIEW, ...jobs];
      return json({ job_id: EXPORT_JOB }, 202);
    }
    if (
      path === `/api/care-pathways/import/${IMPORT_JOB}` &&
      method === "GET"
    ) {
      return json(IMPORT_JOB_VIEW);
    }
    if (
      path === `/api/care-pathways/export/${EXPORT_JOB}` &&
      method === "GET"
    ) {
      return json(EXPORT_JOB_VIEW);
    }
    if (path === "/api/care-pathways/bulk-jobs" && method === "GET") {
      // Pin the server-side kind/status filtering contract: the stub
      // itself filters, so a UI bug that never sends the params (and
      // relies on client-side filtering instead) would show every job
      // regardless of the selected filter.
      const kind = url.searchParams.get("kind");
      const status = url.searchParams.get("status");
      const filtered = jobs.filter(
        (j) => (!kind || j.kind === kind) && (!status || j.status === status),
      );
      return json(filtered);
    }
    return json({ error: "unhandled in stub", path }, 404);
  });
}

test.describe("bulk import/export", () => {
  test("submits an import, shows the completed job, and lists it", async ({
    page,
  }) => {
    await stubBulk(page);
    await page.goto("/bulk");
    await expect(
      page.getByRole("heading", { name: /Bulk import/ }),
    ).toBeVisible();

    // No file chosen yet: submitting reports the client-side guard, no
    // request sent.
    await page.getByRole("button", { name: "Start import" }).click();
    await expect(page.getByText("Choose a file to import.")).toBeVisible();

    // Choose a file, then submit for real.
    await page.setInputFiles('input[type="file"]', {
      name: "pathways.jsonl",
      mimeType: "application/jsonl",
      buffer: Buffer.from('{"name":"Stroke pathway"}\n'),
    });
    await page.getByRole("button", { name: "Start import" }).click();

    // The job panel fills in from the (immediately terminal) status poll:
    // full progress (3/3 · 100%), scoped to the "Job" region so the
    // status filter's own <option data-*> text cannot collide.
    const panel = page.getByRole("region", { name: "Job" });
    await expect(panel.getByText(IMPORT_JOB)).toBeVisible();
    await expect(panel.getByText("Completed", { exact: true })).toBeVisible();
    await expect(panel.getByText("3 / 3 · 100%")).toBeVisible();

    // The recent-jobs table (loaded after submit) shows it too.
    await expect(page.locator("table.jobs")).toContainText(IMPORT_JOB);
  });

  test("submits an export and renders its output reference as plain text, not a link", async ({
    page,
  }) => {
    await stubBulk(page);
    await page.goto("/bulk");

    await page.getByRole("button", { name: "Start export" }).click();
    const panel = page.getByRole("region", { name: "Job" });
    await expect(panel.getByText(EXPORT_JOB)).toBeVisible();
    await expect(panel.getByText("Completed", { exact: true })).toBeVisible();

    // The download reference is an opaque artifact-store string, rendered
    // as <code> text — never an <a href> the sandboxed browser could try
    // to follow (the service exposes no endpoint that serves the bytes).
    const artifact = panel.locator("code.artifact");
    await expect(artifact).toContainText("file://");
    await expect(
      page.locator(`a[href*="${EXPORT_JOB_VIEW.download_url}"]`),
    ).toHaveCount(0);
  });

  test("the recent-jobs kind filter re-queries the server rather than filtering client-side", async ({
    page,
  }) => {
    await stubBulk(page);
    await page.goto("/bulk");

    // Seed both an import and an export job.
    await page.setInputFiles('input[type="file"]', {
      name: "pathways.jsonl",
      mimeType: "application/jsonl",
      buffer: Buffer.from('{"name":"A"}\n'),
    });
    await page.getByRole("button", { name: "Start import" }).click();
    await page.getByRole("button", { name: "Start export" }).click();
    await expect(page.locator("table.jobs tbody tr")).toHaveCount(2);

    // Filtering to "Export" leaves only the export job — proving the
    // filter reached the server (the stub itself does the filtering).
    await page.getByLabel("Kind").selectOption("export");
    await expect(page.locator("table.jobs tbody tr")).toHaveCount(1);
    await expect(page.locator("table.jobs")).toContainText(EXPORT_JOB);
    await expect(page.locator("table.jobs")).not.toContainText(IMPORT_JOB);
  });
});
