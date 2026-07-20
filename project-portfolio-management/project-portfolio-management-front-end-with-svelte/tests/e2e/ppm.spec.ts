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
    if (path === "/api/executive/alignment")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", derivation: "aligned = has an OKR mapping",
        by_collection: [{ collection: "Project", total: 2, aligned: 1, unaligned: 1 }],
        unaligned_spend: [{ currency: "GBP", planned_minor: 500000,
          actual_minor: 0, remaining_minor: 500000, overrun: false, line_count: 1 }],
        unaligned_items: [{ item: { pid: "w-9", name: "Shadow initiative", kind: "Project" },
          planned: [{ currency: "GBP", planned_minor: 500000 }] }],
      } });
    if (path === "/api/technology/debt")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", note: "risks with category tech_debt",
        open_exposure: 16, statuses: { open: 1 },
        register: [{ pid: "r-1", title: "Legacy adapter unmaintained",
          status: "open", exposure: 16, escalated: false, owner_ref: null,
          item: { pid: "w-1", name: "Platform rebuild", kind: "Project" } }],
      } });
    if (path === "/api/technology/flow")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", window_months: 6,
        derivation: "throughput by done_at; lead = done_at - created_at",
        throughput_by_month: { "2026-07": 3 }, timed_completions: 3,
        median_lead_days: 12, undated_completions: 1,
      } });
    if (path.startsWith("/api/scenarios/compare"))
      return route.fulfill({ json: {
        a: { pid: "s-1", name: "Roomy", status: "draft", feasible: true,
             evaluation: { planned_by_currency: [["GBP", 100000]],
               total_exposure: 4, total_alignment: 8, violations: [] } },
        b: { pid: "s-2", name: "Tight", status: "draft", feasible: false,
             evaluation: { planned_by_currency: [["GBP", 100000]],
               total_exposure: 4, total_alignment: 8,
               violations: ["over budget cap"] } },
        deltas: { planned_by_currency: [{ currency: "GBP", a_minor: 100000,
          b_minor: 100000, delta_minor: 0 }], exposure: 0, alignment: 0 },
        note: "b minus a; per-currency deltas only",
      } });
    if (path === "/api/board/pack")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z",
        window: { from: "2026-04-20T00:00:00Z", to: "2026-07-19T00:00:00Z" },
        health_now: { portfolios: { red: 1, amber: 0, green: 2 }, note: "as-of-now" },
        decisions: [{ kind: "gate_review", at: "2026-07-01T09:00:00Z",
          decision: "approved", gate: "g1_feasibility",
          subject: { pid: "w-1", name: "Platform rebuild" } }],
        benefits_realized: { events: 2, per_currency_minor: { GBP: 50000 },
          unattributed_events: 0 },
        milestones_completed: 4,
        tranches_released: { count: 1, per_currency: [{ currency: "GBP",
          planned_minor: 500000, actual_minor: 0, remaining_minor: 500000,
          overrun: false, line_count: 1 }] },
      } });
    if (path === "/api/board/investments")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z",
        investments: [
          { kind: "scenario_commit", at: "2026-07-02T09:00:00Z", name: "FY27 mix",
            budget_cap_minor: 1000000, currency: "GBP" },
          { kind: "tranche_release", at: "2026-07-01T09:00:00Z",
            description: "Tranche 2", gate: "g1_feasibility",
            planned_minor: 500000, currency: "GBP",
            item: { pid: "w-1", name: "Platform rebuild", kind: "Project" } },
        ],
      } });
    if (path === "/api/board/snapshots" && method === "POST")
      return route.fulfill({ json: { id: 2 } });
    if (path === "/api/board/trends")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", note: "stored snapshots only",
        series: [{ taken_at: "2026-07-01T00:00:00Z",
          body: { work_items: 12, portfolios: 3, open_exposure: 44, money: [] } }],
      } });
    if (path === "/api/auditor/trail")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", returned: 1,
        stats: { per_day: { "2026-07-01": 1 }, distinct_actors: 1, actorless: 0 },
        rows: [{ created_at: "2026-07-01T09:00:00Z", actor: "user:ops",
          action: "budget_line_released", entity_pid: "11111111-1111-4111-8111-111111111111" }],
      } });
    if (path === "/api/auditor/findings")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", note: "audit actors only",
        findings: [{ rule: "merge_without_reason",
          detail: "a record merge was performed with no recorded reason" }],
        actorless_actions: 3,
      } });
    if (path === "/api/compliance/register")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", note: "category compliance",
        open_exposure: 20, statuses: { open: 1 },
        register: [{ pid: "r-2", title: "GDPR basis unclear", status: "open",
          exposure: 20, escalated: false, owner_ref: null,
          item: { pid: "w-1", name: "Platform rebuild", kind: "Project" } }],
      } });
    if (path === "/api/compliance/findings")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", review_days: 90,
        findings: [{ rule: "risk_past_review_date",
          detail: "an open risk is past its scheduled review date" }],
      } });
    if (path === "/api/risk/heatmap")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", open_risks: 2, estate_open_exposure: 45,
        cells: { p5i5: 1, p4i5: 1 },
        top_risks: [{ pid: "r-3", title: "Unpatched edge box", exposure: 25,
          category: "security", item: { pid: "w-1", name: "Platform rebuild", kind: "Project" } }],
        posture: [{ portfolio: { pid: "pf-1", name: "Transformation", kind: "Portfolio" },
          open_exposure: 45, escalated: 1, materialised: 0 }],
        concentration: [{ pid: "r-3", title: "Unpatched edge box", exposure: 25 }],
        overdue_reviews: [],
        appetite: null,
        appetite_note: "no risk appetite configured; no thresholds invented",
        breaches: [],
      } });
    if (path === "/api/security/register")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", note: "category security",
        open_exposure: 25, statuses: { open: 1 },
        register: [{ pid: "r-3", title: "Unpatched edge box", status: "open",
          exposure: 25, escalated: true, owner_ref: "worker:x",
          item: { pid: "w-1", name: "Platform rebuild", kind: "Project" } }],
        unreviewed_at_late_stage: { heuristic: "proxy, not proof",
          items: [{ item: { pid: "w-2", name: "Portal", kind: "Product" }, stage: "g3_delivery" }] },
      } });
    if (path === "/api/regulator/extract")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", masked: false,
        note: "deliberately coarse",
        portfolios: [{ pid: "pf-1", name: "Transformation", stage: "g2_definition",
          members: { Portfolio: 1, Project: 2 },
          gate_decisions: { approved: 3 },
          spend: [{ currency: "GBP", planned_minor: 2000000, actual_minor: 900000 }],
          benefits: [{ currency: "GBP", target_minor: 750000, realized_minor: 250000 }] }],
      } });
    if (/^\/api\/projects\/w-1\/tasks$/.test(path) && method === "GET")
      return route.fulfill({ json: {
        tasks: [
          { pid: "t-1", title: "Wire the API", description: null, status: "in_progress",
            assignee_ref: "worker:x", sprint_pid: "sp-1",
            created_at: "2026-07-19T00:00:00Z", status_changed_at: "2026-07-19T01:00:00Z",
            done_at: null, blocked_days: null },
          { pid: "t-2", title: "Fix the build", description: null, status: "blocked",
            assignee_ref: null, sprint_pid: null,
            created_at: "2026-07-18T00:00:00Z", status_changed_at: "2026-07-18T01:00:00Z",
            done_at: null, blocked_days: 2 },
        ],
        counts: { todo: 0, in_progress: 1, in_review: 0, done: 0, blocked: 1 },
      } });
    if (/^\/api\/projects\/w-1\/tasks\/t-1$/.test(path) && method === "PATCH")
      return route.fulfill({ json: {
        pid: "t-1", title: "Wire the API", description: null, status: "in_review",
        assignee_ref: "worker:x", sprint_pid: "sp-1",
        created_at: "2026-07-19T00:00:00Z", status_changed_at: "2026-07-19T02:00:00Z",
        done_at: null, blocked_days: null } });
    if (/^\/api\/projects\/w-1\/sprints$/.test(path) && method === "GET")
      return route.fulfill({ json: [
        { pid: "sp-1", name: "Sprint 1", starts_on: "2026-07-13", ends_on: "2026-07-26" },
      ] });
    if (/^\/api\/projects\/w-1\/burndown/.test(path))
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z",
        sprint: { pid: "sp-1", name: "Sprint 1", starts_on: "2026-07-13", ends_on: "2026-07-15" },
        total_tasks: 2,
        derivation: "no ideal line, no interpolation",
        points: [
          { date: "2026-07-13", remaining: 2 },
          { date: "2026-07-14", remaining: 1 },
          { date: "2026-07-15", remaining: 1 },
        ],
      } });
    if (/^\/api\/projects\/w-1\/standup$/.test(path))
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", since: "2026-07-18T00:00:00Z",
        item: { pid: "w-1", name: "Platform rebuild", kind: "Project" },
        tasks_created: [{ at: "2026-07-19T00:00:00Z", task: "Wire the API", actor: null }],
        tasks_moved: [{ at: "2026-07-19T01:00:00Z", task: "Wire the API", actor: null,
          move: { from: "todo", to: "in_progress" } }],
        blocked_now: [{ pid: "t-2", title: "Fix the build", description: null,
          status: "blocked", assignee_ref: null, sprint_pid: null,
          created_at: "2026-07-18T00:00:00Z", status_changed_at: "2026-07-18T01:00:00Z",
          done_at: null, blocked_days: 2 }],
        risks_raised_estate_wide: 0,
      } });
    if (path === "/api/engineering/blocked")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z",
        derivation: "age = days since the task entered blocked",
        blocked: [{ pid: "t-2", title: "Fix the build", description: null,
          status: "blocked", assignee_ref: null, sprint_pid: null,
          created_at: "2026-07-18T00:00:00Z", status_changed_at: "2026-07-18T01:00:00Z",
          done_at: null, blocked_days: 2,
          item: { pid: "w-1", name: "Platform rebuild", kind: "Project" } }],
      } });
    if (path === "/api/engineering/moscow")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z", convention: "tag moscow:<band>",
        bands: { must: [{ pid: "w-1", name: "Platform rebuild", kind: "Project" }],
                 should: [], could: [], wont: [] },
        untagged: 3,
      } });
    if (path === "/api/engineering/delivery-links")
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z",
        schemes: ["JiraProjectKey", "GitHubProjectId"],
        tracked: [{ item: { pid: "w-1", name: "Platform rebuild", kind: "Project" },
          links: [{ scheme: "GitHubProjectId", value: "42" }] }],
        untracked: [{ pid: "w-3", name: "Untracked idea", kind: "Product" }],
      } });
    if (path.startsWith("/api/engineering/milestone-calendar"))
      return route.fulfill({ json: {
        as_of: "2026-07-19T00:00:00Z",
        kinds: ["milestone", "demo", "release", "checkpoint"],
        milestones: [{ pid: "m-1", name: "Sprint demo", kind: "demo",
          due: "2026-07-24", done: false,
          item: { pid: "w-1", name: "Platform rebuild", kind: "Project" } }],
      } });
    if (path === "/api/scenarios" && method === "GET")
      return route.fulfill({ json: [
        { pid: "s-1", name: "Roomy", status: "draft", members: { work_item_pids: [] },
          budget_cap_minor: 1000000, currency: "GBP", committed_at: null },
        { pid: "s-2", name: "Tight", status: "draft", members: { work_item_pids: [] },
          budget_cap_minor: 100000, currency: "GBP", committed_at: null },
      ] });
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
  await expect(page.getByTestId("exec-alignment").getByText("Project")).toBeVisible();
  await expect(page.getByTestId("exec-unaligned-items").getByText("Shadow initiative")).toBeVisible();
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
  await expect(page.getByTestId("tech-debt").getByText("Legacy adapter unmaintained")).toBeVisible();
  await expect(page.getByTestId("tech-flow").getByText("2026-07")).toBeVisible();
  await expect(page.getByText("12 days")).toBeVisible();
});

test("scenario compare renders side-by-side deltas", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/scenarios");
  const selects = page.locator("form.compare select");
  await selects.nth(0).selectOption({ label: "Roomy" });
  await selects.nth(1).selectOption({ label: "Tight" });
  await page.getByRole("button", { name: "Compare" }).click();
  const table = page.getByTestId("scenario-compare");
  await expect(table.getByText("Roomy")).toBeVisible();
  await expect(table.getByText("no", { exact: true })).toBeVisible();
});

test("board area renders the pack, investments, and trends", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/board");
  await expect(page.getByTestId("board-tiles").getByText("milestones completed")).toBeVisible();
  await expect(page.getByTestId("board-realized").getByText("500.00 GBP")).toBeVisible();
  await expect(page.getByTestId("board-investments").getByText("FY27 mix")).toBeVisible();
  await expect(page.getByTestId("board-trends").getByText("44")).toBeVisible();
});

test("auditor area renders findings and the filtered trail", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/auditor");
  await expect(page.getByTestId("auditor-findings").getByText("merge_without_reason")).toBeVisible();
  await expect(page.getByTestId("auditor-trail").getByText("budget_line_released")).toBeVisible();
  await expect(page.getByRole("link", { name: "Evidence pack (CSV)" })).toBeVisible();
});

test("compliance area renders findings and the register", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/compliance");
  await expect(page.getByTestId("compliance-findings").getByText("risk_past_review_date")).toBeVisible();
  await expect(page.getByTestId("compliance-register").getByText("GDPR basis unclear")).toBeVisible();
});

test("risk area renders the heatmap with an honest no-appetite note", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/risk");
  await expect(page.getByTestId("risk-summary").getByText("45")).toBeVisible();
  await expect(page.getByTestId("risk-appetite")).toContainText("no risk appetite configured");
  await expect(page.getByTestId("risk-top").getByText("Unpatched edge box")).toBeVisible();
});

test("security area renders the register and the late-stage heuristic", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/security");
  await expect(page.getByTestId("security-register").getByText("Unpatched edge box")).toBeVisible();
  await expect(page.getByTestId("security-unreviewed").getByText("Portal")).toBeVisible();
});

test("regulator area renders coarse aggregates", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/regulator");
  await expect(page.getByRole("heading", { name: "Transformation" })).toBeVisible();
  await expect(page.getByTestId("regulator-portfolio").getByText("20,000.00 GBP")).toBeVisible();
});

test("task board renders columns, burndown, and the standup digest", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/projects/w-1/board");
  await expect(page.getByTestId("task-board").getByText("Wire the API")).toBeVisible();
  await expect(page.getByTestId("task-board").getByText("blocked 2d")).toBeVisible();
  await expect(page.getByText("no ideal line, no interpolation")).toBeVisible();
  await expect(page.getByTestId("burndown")).toBeVisible();
  await expect(page.getByTestId("standup").getByText("1 blocked now")).toBeVisible();
});

test("engineering estate area renders blocked, moscow, and delivery links", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/engineering");
  await expect(page.getByTestId("eng-blocked").getByText("Fix the build")).toBeVisible();
  await expect(page.getByTestId("eng-moscow").getByText("must (1)")).toBeVisible();
  await expect(page.getByTestId("eng-links").getByText("GitHubProjectId:42")).toBeVisible();
  await expect(page.getByTestId("eng-untracked")).toContainText("Untracked idea");
});

test("delivery calendar lists milestones with kinds", async ({ page }) => {
  await stubPpm(page);
  await page.goto("/calendar");
  await expect(page.getByTestId("milestone-calendar")).toBeVisible();
  await expect(page.getByTestId("milestone-list").getByText(/demo: Sprint demo/)).toBeVisible();
});
