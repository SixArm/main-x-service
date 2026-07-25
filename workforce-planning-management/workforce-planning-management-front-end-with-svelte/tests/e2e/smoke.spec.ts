// Playwright smoke over a page.route-stubbed API (family pattern):
// the stubs mirror the service contract; any unstubbed /api/proxy
// call 404s loudly, so contract drift fails the suite.

import { expect, test } from "@playwright/test";

const EMPLOYEE = {
  pid: "11111111-1111-4111-8111-111111111111",
  person_ref: "person:22222222-2222-4222-8222-222222222222",
  worker_ref: null,
  organization_ref: "organization:33333333-3333-4333-8333-333333333333",
  employee_number: "E-0001",
  display_name: "Test Employee 001",
  status: "active",
  employment_type: "permanent",
  fte_percent: 100,
  department: "engineering",
  job_title: "Engineer",
  manager_pid: null,
  salary_minor: 3600000,
  salary_currency: "GBP",
  hired_on: "2026-01-05",
  terminated_on: null,
};

const MASKED_EMPLOYEE = {
  ...EMPLOYEE,
  pid: "44444444-4444-4444-8444-444444444444",
  employee_number: "E-0002",
  display_name: "Masked Person",
  salary_minor: null,
  salary_currency: null,
};

test.beforeEach(async ({ page }) => {
  // Unstubbed API calls fail loudly.
  await page.route("**/api/proxy/**", (route) =>
    route.fulfill({ status: 404, body: "unstubbed: " + route.request().url() }),
  );
  await page.route("**/api/proxy/employees", (route) =>
    route.fulfill({ json: [EMPLOYEE, MASKED_EMPLOYEE] }),
  );
  await page.route("**/api/proxy/employees?status=active", (route) =>
    route.fulfill({ json: [EMPLOYEE, MASKED_EMPLOYEE] }),
  );
  await page.route("**/api/proxy/requisitions?status=open", (route) =>
    route.fulfill({
      json: [
        {
          pid: "55555555-5555-4555-8555-555555555555",
          organization_ref: EMPLOYEE.organization_ref,
          department: "engineering",
          job_title: "Platform Engineer",
          headcount: 1,
          salary_min_minor: null,
          salary_max_minor: null,
          salary_currency: null,
          status: "open",
          opened_on: "2026-06-01",
        },
      ],
    }),
  );
  await page.route("**/api/proxy/succession-plans/gaps", (route) =>
    route.fulfill({ json: { gaps: [] } }),
  );
});

test("dashboard renders live tiles from the stubbed API", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("tile-active")).toContainText("2");
  await expect(page.getByTestId("tile-open")).toContainText("1");
  await expect(page.getByTestId("tile-gaps")).toContainText("0");
});

test("employee list shows money for visible salaries and Hidden for masked", async ({
  page,
}) => {
  await page.goto("/employees");
  const table = page.getByTestId("employee-table");
  await expect(table).toContainText("E-0001");
  await expect(table).toContainText("£36,000.00");
  await expect(table).toContainText("Hidden");
});

test("payroll run detail drives the lifecycle actions", async ({ page }) => {
  const run = {
    pid: "66666666-6666-4666-8666-666666666666",
    organization_ref: EMPLOYEE.organization_ref,
    period_start: "2026-07-01",
    period_end: "2026-07-31",
    status: "calculated",
  };
  await page.route(`**/api/proxy/payroll-runs/${run.pid}`, (route) =>
    route.fulfill({ json: run }),
  );
  await page.route(`**/api/proxy/payroll-runs/${run.pid}/payslips`, (route) =>
    route.fulfill({
      json: [
        {
          pid: "77777777-7777-4777-8777-777777777777",
          run_pid: run.pid,
          employee_pid: EMPLOYEE.pid,
          currency: "GBP",
          gross_minor: 300000,
          deductions: [{ label: "tax", amount_minor: 39050 }],
          net_minor: 260950,
        },
      ],
    }),
  );
  await page.goto(`/payroll/${run.pid}`);
  await expect(page.getByTestId("run-status")).toHaveText("calculated");
  await expect(page.getByTestId("action-approve")).toBeVisible();
  const payslips = page.getByTestId("payslips");
  await expect(payslips).toContainText("£3,000.00");
  await expect(payslips).toContainText("£2,609.50");
  await expect(payslips).toContainText("tax");
});

test("locale switcher retranslates the chrome (and ar flips direction)", async ({
  page,
}) => {
  await page.goto("/employees");
  await expect(page.locator("nav.top")).toContainText("Employees");
  await page.locator("nav.top select.locale-select").selectOption("de");
  await expect(page.locator("nav.top")).toContainText("Mitarbeiter");
  await page.locator("nav.top select.locale-select").selectOption("ar");
  await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
});

test("requisition board renders SVAR Kanban columns and cards", async ({ page }) => {
  await page.route("**/api/proxy/requisitions", (route) =>
    route.fulfill({
      json: [
        {
          pid: "99999999-9999-4999-8999-999999999999",
          organization_ref: EMPLOYEE.organization_ref,
          department: "engineering",
          job_title: "Platform Engineer",
          headcount: 2,
          salary_min_minor: 3000000,
          salary_max_minor: 5000000,
          salary_currency: "GBP",
          status: "interviewing",
          opened_on: "2026-06-01",
        },
      ],
    }),
  );
  await page.goto("/requisitions");
  const board = page.getByTestId("requisition-board");
  await expect(board).toContainText("interviewing");
  await expect(board).toContainText("Platform Engineer");
  await expect(board).toContainText("£30,000.00");
});

test("learning area renders skills matrix, analytics, and path progress", async ({ page }) => {
  await page.route("**/api/proxy/learning/skills-matrix", (route) =>
    route.fulfill({
      json: {
        as_of: "2026-07-20T00:00:00Z",
        note: "coverage over declared proficiencies only",
        matrix: [{ department: "engineering", skill: "Rust", employees: 2,
          average_proficiency: 3.5, below_target: 1 }],
        gaps: [{ employee_pid: "e2", department: "engineering", skill: "Rust",
          proficiency: 2, target: 4 }],
      },
    }),
  );
  await page.route("**/api/proxy/learning/training-analytics", (route) =>
    route.fulfill({
      json: {
        as_of: "2026-07-20", horizon: "2026-10-18",
        note: "completion rate = completed / non-failed",
        departments: [{ department: "engineering", by_status: { completed: 1 },
          completion_rate: { numerator: 1, denominator: 1, value: 1 }, certs_expiring: 0 }],
      },
    }),
  );
  await page.route("**/api/proxy/learning-paths", (route) =>
    route.fulfill({ json: [{ pid: "path1", name: "Backend basics", summary: null, steps: 2 }] }),
  );
  await page.route("**/api/proxy/learning-paths/path1/progress", (route) =>
    route.fulfill({
      json: {
        as_of: "2026-07-20T00:00:00Z",
        path: { pid: "path1", name: "Backend basics" },
        steps: [{ course_ref: "course:a", title: "Intro", position: 0 }],
        derivation: "a step is complete iff a completed training enrolment matches",
        members: [{ employee_pid: "e2", display_name: "Sam Mentee",
          completed_steps: 1, total_steps: 2 }],
      },
    }),
  );
  await page.goto("/learning");
  await expect(page.getByTestId("skills-matrix").getByText("Rust")).toBeVisible();
  await expect(page.getByTestId("skills-gaps").getByText(/Rust in engineering/)).toBeVisible();
  await expect(page.getByTestId("training-analytics").getByText("100% (1/1)")).toBeVisible();
  await expect(page.getByTestId("path-progress").getByText("Sam Mentee")).toBeVisible();
});

test("mentorship area renders load, unmatched, and stale", async ({ page }) => {
  await page.route("**/api/proxy/learning/mentorship-overview**", (route) =>
    route.fulfill({
      json: {
        as_of: "2026-07-20",
        active_pairings: 1,
        mentor_load: [{ mentor_pid: "e1", mentor: "Ada Mentor", active_mentees: 1 }],
        unmatched_employees: [{ pid: "e3", display_name: "Solo Dev", department: "engineering" }],
        stale_days: 30,
        stale_mentorships: [{ pid: "m1", mentor: "Ada Mentor", mentee: "Sam Mentee",
          last_session: "2026-05-01" }],
      },
    }),
  );
  await page.goto("/mentorship");
  await expect(page.getByTestId("mentor-load").getByText("Ada Mentor")).toBeVisible();
  await expect(page.getByTestId("unmatched").getByText("Solo Dev")).toBeVisible();
  await expect(page.getByTestId("stale-mentorships").getByText("2026-05-01")).toBeVisible();
});

test("wellbeing area renders rules and aggregate-only uptake", async ({ page }) => {
  await page.route("**/api/proxy/wellbeing-entitlements", (route) =>
    route.fulfill({
      json: [{
        pid: "w1", name: "Seasonal flu vaccination", kind: "health",
        benefit_plan_pid: null,
        description: "Free NHS flu jab for frontline staff.",
        info_url: null, min_age: null, max_age: null,
        departments: ["engineering"], job_titles: [], doses: 2,
        active_from: null, active_until: null,
      }, {
        pid: "w2", name: "Cycle-to-work scheme", kind: "benefit",
        benefit_plan_pid: "bp1",
        description: "Save on a bike through salary sacrifice.",
        info_url: null, min_age: null, max_age: null,
        departments: [], job_titles: [], doses: 1,
        active_from: null, active_until: null,
      }],
    }),
  );
  await page.route("**/api/proxy/wellbeing/uptake", (route) =>
    route.fulfill({
      json: {
        as_of: "2026-07-24T00:00:00Z",
        derivation: "uptake = (booked + done) / all acknowledgements; counts only",
        entitlements: [{
          entitlement_pid: "w1", name: "Seasonal flu vaccination", kind: "health",
          by_response: { booked: 3, done: 1, declined: 1, dismissed: 0 },
          uptake_rate: { numerator: 4, denominator: 5, value: 0.8 },
          enrolment_conversion: null,
        }, {
          entitlement_pid: "w2", name: "Cycle-to-work scheme", kind: "benefit",
          by_response: { booked: 0, done: 2, declined: 0, dismissed: 2 },
          uptake_rate: { numerator: 2, denominator: 4, value: 0.5 },
          enrolment_conversion: { numerator: 2, denominator: 4, value: 0.5 },
        }],
      },
    }),
  );
  await page.route("**/api/proxy/pulse-surveys", (route) =>
    route.fulfill({
      json: [{
        pid: "s1", name: "July pulse", question: "How are you doing this week?",
        active_from: null, active_until: null, open: true,
      }],
    }),
  );
  await page.route("**/api/proxy/pulse-surveys/s1/results", (route) =>
    route.fulfill({
      json: {
        as_of: "2026-07-25T00:00:00Z",
        survey: { pid: "s1", name: "July pulse", question: "How are you doing this week?" },
        overall: { suppressed: false, count: 6, distribution: [1, 1, 1, 1, 2], mean: 3.5 },
        departments: [
          { department: "engineering", suppressed: false, count: 5,
            distribution: [1, 1, 1, 1, 1], mean: 3.0 },
          { department: "finance", suppressed: true },
        ],
        derivation: "anonymous by construction; cells under 5 responses are suppressed",
      },
    }),
  );
  await page.goto("/wellbeing");
  await expect(page.getByTestId("wellbeing-rules").getByText("Seasonal flu vaccination")).toBeVisible();
  await expect(page.getByTestId("wellbeing-rules").getByText("engineering")).toBeVisible();
  await expect(page.getByTestId("wellbeing-rules").getByText("Cycle-to-work scheme")).toBeVisible();
  await expect(page.getByTestId("wellbeing-rules").getByText("Benefit")).toBeVisible();
  await expect(page.getByTestId("wellbeing-uptake").getByText("80% (4/5)")).toBeVisible();
  await expect(page.getByTestId("wellbeing-uptake").getByText(/Enrolled after prompt/)).toBeVisible();
  await expect(page.getByTestId("wellbeing-uptake").getByText(/50% \(2\/4\)/).first()).toBeVisible();
  await expect(page.getByTestId("pulse-overall")).toContainText("3.5");
  await expect(page.getByTestId("pulse-results").getByText("Hidden below 5 responses")).toBeVisible();
});
