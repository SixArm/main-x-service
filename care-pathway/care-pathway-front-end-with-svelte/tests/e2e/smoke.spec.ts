import { test, expect, type Page } from "@playwright/test";

// Smoke tests over the registry grid, detail, insights, board (Kanban),
// and gantt routes. The backend is stubbed so a broken endpoint contract
// (wrong path / method) surfaces as an unhandled 404 request and a failing
// assertion, without needing the Rust service. Unstubbed calls are 404-loud.

const PID = "11111111-1111-4111-8111-111111111111";
const INSTANCE_PID = "33333333-3333-4333-8333-333333333333";
const SUBJECT = "person:44444444-4444-4444-8444-444444444444";

const PATHWAY = {
  name: "Acute Stroke Care Pathway",
  pathway_code: "STROKE-01",
  provider_id: "trust-1",
  provider_name: "Example NHS Trust",
  care_setting: "EmergencyDepartment",
  alternate_names: [],
  condition_codes: [{ system: "Icd10", code: "I63.9" }],
  interventions: ["Thrombolysis", "Rehabilitation"],
  keywords: ["stroke"],
  identifiers: [{ scheme: "GuidelineId", value: "NICE-NG128" }],
  same_as: [],
  in_language: ["en"],
};

const INSTANCE = {
  pid: INSTANCE_PID,
  pathway_pid: PID,
  subject_ref: SUBJECT,
  status: "active",
  urgency: "urgent",
  enrolled_on: "2026-06-01",
  next_review_on: "2026-08-01",
  closed_on: null,
  closure_reason: null,
  outcome: null,
};

/** Stub every `/api/**` call so the SPA renders offline; 404 otherwise. */
async function stubApi(page: Page) {
  await page.route("**/api/**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    // The browser calls the same-origin BFF proxy
    // (`/api/proxy/<service path>`, see `src/lib/config.ts` +
    // `src/routes/api/proxy/[...path]/+server.ts`), not the bare service
    // path — strip the proxy prefix so the matches below stay written
    // against the service's own paths (2026-08-03 ApiClient fix,
    // tasks.md FE-2; this stub predated it and matched on the bare path).
    const path = url.pathname.startsWith("/api/proxy")
      ? url.pathname.slice("/api/proxy".length)
      : url.pathname;
    const method = req.method();
    const json = (body: unknown) => route.fulfill({ json: body });

    // Instances endpoints.
    if (path === "/api/instances/caseload" && method === "GET") {
      return json({ note: "derived caseload", total: 1, by_status: {}, by_urgency: {} });
    }
    if (path === `/api/instances/${INSTANCE_PID}/status` && method === "POST") {
      return json({ ...INSTANCE, status: "on_hold" });
    }

    // Insight lenses.
    if (path === "/api/care-pathways/insights/directory") {
      return json({
        as_of: "2026-07-20T00:00:00Z",
        note: "settings from the DTO's care_setting",
        total: 1,
        by_setting: { EmergencyDepartment: [{ pid: PID, name: PATHWAY.name, specialty: "stroke" }] },
        by_specialty: { stroke: 1 },
      });
    }
    if (path === "/api/care-pathways/insights/coverage") {
      return json({
        as_of: "2026-07-20T00:00:00Z",
        note: "coverage over condition_codes × care_setting",
        conditions: [{ condition: "Icd10:I63.9", settings: ["EmergencyDepartment"] }],
        gaps: [{ rule: "no_primary_care_pathway", detail: "no primary care", condition: "Icd10:I63.9" }],
      });
    }
    if (path === "/api/care-pathways/insights/variants") {
      return json({ as_of: "x", note: "cross-provider variants", variants: [] });
    }
    if (path === "/api/care-pathways/insights/providers") {
      return json({
        as_of: "x",
        note: "provider directory",
        providers: [{ provider: "trust-1", pathways: 1, by_setting: { EmergencyDepartment: 1 } }],
      });
    }
    if (path === "/api/care-pathways/insights/languages") {
      return json({
        as_of: "x",
        note: "per-language counts over in_language",
        by_language: { en: 1 },
        single_language_conditions: [{ condition: "Icd10:I63.9", language: "en" }],
      });
    }

    // Registry endpoints.
    if (path === `/api/care-pathways/${PID}/instances` && method === "GET") {
      return json([INSTANCE]);
    }
    if (path === `/api/care-pathways/${PID}` && method === "GET") {
      return json(PATHWAY);
    }
    if (path === "/api/care-pathways" && method === "GET") {
      return json([{ pid: PID, name: PATHWAY.name }]);
    }
    if (path === "/api/care-pathways/search" && method === "GET") {
      return json([{ pid: PID, name: PATHWAY.name }]);
    }

    return route.fulfill({ status: 404, json: { error: "unhandled in stub", path } });
  });
}

test.beforeEach(async ({ page }) => {
  await stubApi(page);
});

test("registry grid renders the seeded pathway", async ({ page }) => {
  await page.goto("/", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Care pathways" })).toBeVisible();
  await expect(page.getByTestId("pathway-grid")).toBeVisible();
  await expect(page.getByText(PATHWAY.name)).toBeVisible();
});

test("detail page renders the fetched pathway and its instances", async ({ page }) => {
  await page.goto(`/${PID}`, { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: PATHWAY.name })).toBeVisible();
  await expect(page.getByTestId("pathway-instances")).toBeVisible();
  await expect(page.getByText(SUBJECT).first()).toBeVisible();
});

test("insights page renders the five lens tables", async ({ page }) => {
  await page.goto("/insights", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Insights" })).toBeVisible();
  for (const id of [
    "insight-directory",
    "insight-coverage",
    "insight-variants",
    "insight-providers",
    "insight-languages",
  ]) {
    await expect(page.getByTestId(id)).toBeVisible();
  }
  // A service-derived note string is shown verbatim.
  await expect(page.getByText("provider directory")).toBeVisible();
});

test("board renders the pathway's instances as Kanban cards", async ({ page }) => {
  await page.goto("/board", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Board" })).toBeVisible();
  await expect(page.getByTestId("instance-board")).toBeVisible();
  await expect(page.getByText(SUBJECT).first()).toBeVisible();
});

test("gantt renders the instance timeline", async ({ page }) => {
  await page.goto("/gantt", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Gantt" })).toBeVisible();
  await expect(page.getByTestId("instance-gantt")).toBeVisible();
});

test("sequence renders the intervention-sequence gantt", async ({ page }) => {
  await page.goto("/sequence", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Sequence" })).toBeVisible();
  await expect(page.getByTestId("pathway-sequence")).toBeVisible();
});
