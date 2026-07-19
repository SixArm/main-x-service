// Playwright smoke over a page.route-stubbed API (family pattern):
// the stubs mirror the service contract; any unstubbed /api/proxy
// call 404s loudly, so contract drift fails the suite.

import { expect, test } from "@playwright/test";

const STAGES = [
  { pid: "s1", name: "Qualification", position: 0, probability_percent: 10, is_won: false, is_lost: false },
  { pid: "s2", name: "Proposal", position: 1, probability_percent: 50, is_won: false, is_lost: false },
  { pid: "s3", name: "Won", position: 2, probability_percent: 100, is_won: true, is_lost: false },
];

const DEAL = {
  pid: "d1",
  name: "Big Deal",
  pipeline_pid: "p1",
  stage_pid: "s2",
  amount_minor: 5000000,
  currency: "GBP",
  won: false,
  closed_at: null,
};

test.beforeEach(async ({ page }) => {
  await page.route("**/api/proxy/**", (route) =>
    route.fulfill({ status: 404, body: "unstubbed: " + route.request().url() }),
  );
  await page.route("**/api/proxy/dashboards/sales", (route) =>
    route.fulfill({
      json: {
        win_rate: { numerator: 2, denominator: 4, value: 0.5 },
        open_deals: 8,
        pipeline_by_stage: {},
      },
    }),
  );
  await page.route("**/api/proxy/dashboards/sla", (route) =>
    route.fulfill({
      json: { open_tickets: 3, by_priority: [{ priority: "normal", open: 3, breached: 1 }] },
    }),
  );
  await page.route("**/api/proxy/forecast", (route) =>
    route.fulfill({
      json: { as_of: "2026-07-18T12:00:00Z", open_deals: 8, totals_minor: { GBP: 4887500 } },
    }),
  );
});

test("dashboard renders honest KPIs from the stubbed API", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("tile-winrate")).toContainText("50% (2/4)");
  await expect(page.getByTestId("tile-deals")).toContainText("8");
  await expect(page.getByTestId("tile-tickets")).toContainText("3");
  await expect(page.getByTestId("tile-forecast")).toContainText("£48,875.00");
});

test("deal board renders stage columns and the forecast strip", async ({ page }) => {
  await page.route("**/api/proxy/pipelines", (route) =>
    route.fulfill({ json: [{ pipeline: { pid: "p1", name: "New Business" }, stages: STAGES }] }),
  );
  await page.route("**/api/proxy/deals?pipeline=p1", (route) =>
    route.fulfill({ json: [DEAL] }),
  );
  await page.goto("/deals");
  const board = page.getByTestId("deal-board");
  await expect(board).toContainText("Qualification");
  await expect(board).toContainText("Proposal");
  await expect(board).toContainText("Big Deal");
  await expect(board).toContainText("£50,000.00");
  await expect(page.getByTestId("forecast")).toContainText("£48,875.00");
});

test("lead queue expands the score breakdown", async ({ page }) => {
  const lead = {
    pid: "l1",
    source: "referral",
    display_name: "Hot Prospect",
    email: null,
    score: 45,
    status: "contacted",
  };
  await page.route("**/api/proxy/leads", (route) => route.fulfill({ json: [lead] }));
  await page.route("**/api/proxy/leads/l1", (route) =>
    route.fulfill({
      json: {
        lead,
        score: {
          score: 45,
          label: "warm",
          rules: [
            { rule: "source_referral", points: 20 },
            { rule: "known_contact", points: 15 },
            { rule: "corporate_domain", points: 10 },
          ],
        },
      },
    }),
  );
  await page.goto("/leads");
  await expect(page.getByTestId("lead-queue")).toContainText("Hot Prospect");
  await page.getByRole("button", { name: "Score breakdown" }).click();
  const breakdown = page.getByTestId("breakdown");
  await expect(breakdown).toContainText("45 · warm");
  await expect(breakdown).toContainText("source_referral: 20");
});

test("ticket queue flags breaches; locale switch retranslates + flips RTL", async ({
  page,
}) => {
  await page.route("**/api/proxy/tickets", (route) =>
    route.fulfill({
      json: [
        {
          pid: "t1",
          title: "Login broken",
          priority: "urgent",
          status: "open",
          first_response_due_at: "2026-07-18T10:00:00Z",
          live_first_response_breached: true,
          live_resolution_breached: false,
        },
      ],
    }),
  );
  await page.goto("/tickets");
  await expect(page.getByTestId("breached")).toBeVisible();
  await expect(page.locator("nav.top")).toContainText("Tickets");
  await page.locator("nav.top select.locale-select").selectOption("de");
  await expect(page.locator("nav.top")).toContainText("Kontakte");
  await page.locator("nav.top select.locale-select").selectOption("ar");
  await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
});
