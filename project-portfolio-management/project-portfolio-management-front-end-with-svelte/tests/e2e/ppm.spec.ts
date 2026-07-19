// PPM-view smoke tests: dashboard, intake board, and ideas over a
// page.route-stubbed API (mirroring the service contract; unmatched
// calls 404 so drift fails loud). No Rust service needed.

import { test, expect, type Page } from "@playwright/test";

const PROPOSAL = "11111111-1111-4111-8111-111111111111";
const IDEA = "22222222-2222-4222-8222-222222222222";

const DASHBOARD = {
  as_of: "2026-07-18T12:00:00Z",
  collections: [
    {
      collection: "Project",
      total: 5,
      rag: { red: 1, amber: 2, green: 2 },
      stages: { pre_gate: 3, g1_feasibility: 2 },
    },
  ],
  site_tiles: {
    work_items: 5,
    proposals_open: 2,
    materialised_risks: 1,
    open_risk_exposure: 34,
    schedule_violations: 1,
    over_allocated_people: 1,
  },
};

const PROPOSALS = [
  {
    pid: PROPOSAL,
    title: "Website replatform",
    summary: "Modernise the stack",
    kind_target: "projects",
    sponsor_ref: null,
    strategic_rationale: null,
    requested_minor: 25_000_000,
    currency: "GBP",
    status: "in_review",
    promoted_work_item_pid: null,
  },
];

const IDEAS = [
  {
    pid: IDEA,
    title: "Self-service portal",
    pitch: "Cut ticket volume",
    tags: [],
    votes: 4,
    status: "open",
    converted_proposal_pid: null,
  },
];

async function stubPpm(page: Page) {
  await page.route("**/api/**", async (route) => {
    const url = new URL(route.request().url());
    const method = route.request().method();
    const path = url.pathname;
    if (path === "/api/at-a-glance") return route.fulfill({ json: DASHBOARD });
    if (path === "/api/proposals" && method === "GET")
      return route.fulfill({ json: PROPOSALS });
    if (path === `/api/proposals/${PROPOSAL}/approve` && method === "POST")
      return route.fulfill({ json: { ...PROPOSALS[0], status: "approved" } });
    if (path === `/api/proposals/${PROPOSAL}/duplicates`)
      return route.fulfill({
        json: [{ source: "work_item", pid: "w1", name: "Website rebuild", score: 0.91 }],
      });
    if (path === "/api/ideas" && method === "GET") return route.fulfill({ json: IDEAS });
    if (path === `/api/ideas/${IDEA}/vote` && method === "POST")
      return route.fulfill({ json: { ...IDEAS[0], votes: 5 } });
    if (path === "/api/executive/health")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", derivation: "worst member RAG",
        portfolios: [{
          portfolio: { pid: "pf-1", name: "Transformation", kind: "Portfolio" },
          status: "red", rag: { red: 1, amber: 0, green: 2 }, members: 3,
          overdue_milestones: 1, escalated_risks: 0, open_risk_exposure: 12,
          overrun_currencies: ["GBP"], days_since_last_update: 2,
        }],
      } });
    if (path === "/api/executive/decisions")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", total: 1, returned: 1,
        decisions: [{ kind: "gate_review", at: "2026-07-18T10:00:00Z",
          decision: "approved", gate: "g0_concept",
          subject: { pid: "w-1", name: "Platform rebuild" }, actor: "worker:x" }],
      } });
    if (path === "/api/executive/benefits")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", note: "never merged",
        portfolios: [{
          portfolio: { pid: "pf-1", name: "Transformation", kind: "Portfolio" },
          benefits: 1, non_financial: 0, statuses: { planned: 1 },
          financial: [{ currency: "GBP", target_minor: 100000,
                        realized_minor: 25000, realization_ratio: 0.25 }],
        }],
      } });
    if (path === "/api/financials/variance")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", note: "no FX conversion",
        by_collection: [{ collection: "Project", variance: [{ currency: "GBP",
          planned_minor: 100000, actual_minor: 150000, remaining_minor: -50000,
          overrun: true, line_count: 1 }] }],
        by_category: [{ category: "capex", variance: [{ currency: "GBP",
          planned_minor: 100000, actual_minor: 150000, remaining_minor: -50000,
          overrun: true, line_count: 1 }] }],
        by_portfolio: [],
      } });
    if (path === "/api/financials/exposure")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", note: "currencies never converted",
        currencies: [{ currency: "GBP", planned_minor: 100000,
          actual_minor: 150000, remaining_minor: -50000, overrun: true,
          line_count: 1, work_items: 1 }],
      } });
    if (path === "/api/technology/dependency-risk")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", edges: 1,
        top_fan_out: [{ item: { pid: "w-1", name: "Platform rebuild", kind: "Project" },
                        dependents: 2, rag: "red" }],
        cross_portfolio: [],
        red_predecessor_edges: [{ edge: "e-1",
          predecessor: { pid: "w-1", name: "Platform rebuild", kind: "Project" },
          successor: { pid: "w-2", name: "Portal", kind: "Product" } }],
      } });
    if (path === "/api/technology/radar")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", convention: "tech:<name>[:<ring>]",
        technologies: [{ technology: "rust", ring: "adopt", ring_votes: 2,
          per_collection: { Project: 1, Product: 1 },
          items: [{ pid: "w-1", name: "Platform rebuild", kind: "Project" }] }],
      } });
    return route.fulfill({ status: 404, json: { error: `unstubbed ${method} ${path}` } });
  });
}

test("dashboard renders tiles and RAG rollups", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/dashboard");
  const tiles = page.getByTestId("site-tiles");
  await expect(tiles.getByText("open proposals")).toBeVisible();
  await expect(page.getByRole("cell", { name: "Project" })).toBeVisible();
  // RAG counts render in their columns.
  await expect(page.locator("td.rag.red")).toHaveText("1");
  await expect(page.locator("td.rag.green")).toHaveText("2");
});

test("intake board lists proposals with pipeline actions and duplicate hits", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/proposals");
  await expect(page.getByText("Website replatform")).toBeVisible();
  await expect(page.getByText("250,000.00 GBP")).toBeVisible();
  // in_review offers approve/reject; approve round-trips the stub.
  await expect(page.getByRole("button", { name: "reject" })).toBeVisible();
  await page.getByRole("button", { name: "approve" }).click();
  // The duplicate check surfaces the matcher hit.
  await page.getByRole("button", { name: "duplicates?" }).click();
  await expect(page.getByText(/Website rebuild \(0\.91\)/)).toBeVisible();
});

test("idea board votes", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/ideas");
  await expect(page.getByText("Self-service portal")).toBeVisible();
  await page.getByRole("button", { name: "▲ 4" }).click();
  await expect(page.getByRole("button", { name: "▲ 4" })).toBeVisible(); // list re-fetch stubbed static
});

test("executive area renders health, benefits, and the decision log", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/executive");
  await expect(page.getByTestId("exec-health").getByText("Transformation")).toBeVisible();
  await expect(page.getByTestId("exec-health").getByText("red", { exact: true })).toBeVisible();
  await expect(page.getByText("25%")).toBeVisible(); // realization ratio served, not computed
  await expect(page.getByTestId("exec-decisions").getByText("gate_review · g0_concept")).toBeVisible();
});

test("financial area shows per-currency exposure with overrun highlighted", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/financials");
  await expect(page.getByText("currencies never converted")).toBeVisible();
  await expect(page.getByTestId("fin-exposure").getByText("GBP", { exact: true })).toBeVisible();
  await expect(page.getByTestId("fin-by-category").getByText("capex")).toBeVisible();
});

test("technology area renders the radar rings and dependency lens", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/technology");
  await expect(page.getByTestId("radar-adopt").getByText("rust")).toBeVisible();
  await expect(page.getByTestId("tech-fan-out").getByText("Platform rebuild")).toBeVisible();
  await expect(page.getByTestId("tech-red-edges").getByText("Portal")).toBeVisible();
});
