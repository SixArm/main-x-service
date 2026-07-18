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
  await page.locator("nav.top select").selectOption("de");
  await expect(page.locator("nav.top")).toContainText("Mitarbeiter");
  await page.locator("nav.top select").selectOption("ar");
  await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
});
