// Playwright smoke over a page.route-stubbed API (family pattern):
// the stubs mirror the service contract; any unstubbed /api/proxy
// call 404s loudly, so contract drift fails the suite.

import { expect, test, type Page } from "@playwright/test";

/**
 * Every page but /signin and /verify is gated on a session (CRM-T26),
 * so the smoke suite injects a fake `__Host-mxi_session` cookie
 * directly into the browser context before each test — the server
 * only checks the cookie's *presence*, never its validity
 * (`locals.sessionId` is set straight from `cookies.get`), so a
 * fabricated value is enough to pass the gate without a real
 * authentication-service round trip.
 */
async function signIn(page: Page) {
  await page.context().addCookies([
    {
      name: "__Host-mxi_session",
      value: "smoke-test-session",
      domain: "localhost",
      path: "/",
      httpOnly: true,
      secure: true,
      sameSite: "Lax",
    },
  ]);
}

const STAGES = [
  {
    pid: "s1",
    name: "Qualification",
    position: 0,
    probability_percent: 10,
    is_won: false,
    is_lost: false,
  },
  {
    pid: "s2",
    name: "Proposal",
    position: 1,
    probability_percent: 50,
    is_won: false,
    is_lost: false,
  },
  {
    pid: "s3",
    name: "Won",
    position: 2,
    probability_percent: 100,
    is_won: true,
    is_lost: false,
  },
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

/**
 * Choose a locale from the Lily locale picker.
 *
 * Three things make this more than a `selectOption`:
 *
 * 1. The picker renders a button plus a `ul` listbox, not a `<select>`.
 * 2. The theme picker on the same page also renders
 *    `li[role="option"]`, so the list has to be scoped — unscoped, the
 *    selector matches 58 elements here.
 * 3. The listbox is **still expanded after a pointer selection**
 *    (verified in a browser against Lily as of 2026-07-31), so
 *    clicking the button unconditionally would close it rather than
 *    open it. Open only when collapsed, which is correct either way.
 */
async function chooseLocale(page: Page, label: string) {
  const button = page.locator("nav.top .locale-picker-button");
  const list = page.locator("ul.locale-picker-list");
  if ((await button.getAttribute("aria-expanded")) !== "true") {
    await button.click();
  }
  await expect(list).toBeVisible();
  await list
    .locator('li[role="option"]')
    .filter({ hasText: label })
    .first()
    .click();
}

test.describe("sign-in gate (CRM-T26)", () => {
  // These tests run with NO session cookie — the opposite of every
  // other test in this file — so they get their own describe block
  // rather than relying on the shared beforeEach below.
  test("a signed-out visitor is redirected from a protected page to /signin", async ({
    page,
  }) => {
    await page.goto("/deals");
    await expect(page).toHaveURL(/\/signin$/);
  });

  test("a signed-out visitor is redirected from the dashboard too", async ({
    page,
  }) => {
    await page.goto("/");
    await expect(page).toHaveURL(/\/signin$/);
  });

  test("/signin itself stays reachable with no session", async ({ page }) => {
    const response = await page.goto("/signin");
    expect(response?.status()).toBe(200);
    await expect(page).toHaveURL(/\/signin$/);
  });
});

// Every test below exercises a gated page, so it needs a session
// already present.
test.describe("signed-in smoke coverage", () => {
  test.beforeEach(async ({ page }) => {
    await signIn(page);
    await page.route("**/api/proxy/**", (route) =>
      route.fulfill({
        status: 404,
        body: "unstubbed: " + route.request().url(),
      }),
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
        json: {
          open_tickets: 3,
          by_priority: [{ priority: "normal", open: 3, breached: 1 }],
        },
      }),
    );
    await page.route("**/api/proxy/forecast", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-18T12:00:00Z",
          open_deals: 8,
          totals_minor: { GBP: 4887500 },
        },
      }),
    );
  });

  test("dashboard renders honest KPIs from the stubbed API", async ({
    page,
  }) => {
    await page.goto("/");
    await expect(page.getByTestId("tile-winrate")).toContainText("50% (2/4)");
    await expect(page.getByTestId("tile-deals")).toContainText("8");
    await expect(page.getByTestId("tile-tickets")).toContainText("3");
    await expect(page.getByTestId("tile-forecast")).toContainText("£48,875.00");
  });

  test("deal board renders stage columns and the forecast strip", async ({
    page,
  }) => {
    await page.route("**/api/proxy/pipelines", (route) =>
      route.fulfill({
        json: [
          { pipeline: { pid: "p1", name: "New Business" }, stages: STAGES },
        ],
      }),
    );
    await page.route("**/api/proxy/insights/funnel**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          pipeline: { pid: "p1", name: "New Business" },
          derivation: "entered from recorded to_stage audits",
          stages: [
            {
              stage: "Qualification",
              position: 0,
              is_won: false,
              is_lost: false,
              entered: 4,
              conversion_from_previous: null,
            },
            {
              stage: "Proposal",
              position: 1,
              is_won: false,
              is_lost: false,
              entered: 2,
              conversion_from_previous: {
                numerator: 2,
                denominator: 4,
                value: 0.5,
              },
            },
          ],
        },
      }),
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
    await page.route("**/api/proxy/leads", (route) =>
      route.fulfill({ json: [lead] }),
    );
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
    await chooseLocale(page, "Deutsch");
    await expect(page.locator("nav.top")).toContainText("Kontakte");
    await chooseLocale(page, "العربية");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
  });

  test("lead board renders columns with scored cards", async ({ page }) => {
    await page.route("**/api/proxy/leads", (route) =>
      route.fulfill({
        json: [
          {
            pid: "l1",
            source: "web form",
            display_name: "Alix Chen",
            email: null,
            score: 42,
            status: "new",
          },
          {
            pid: "l2",
            source: "referral",
            display_name: "Sam Ortiz",
            email: null,
            score: 65,
            status: "qualified",
          },
        ],
      }),
    );
    await page.goto("/leads/board");
    await expect(
      page.getByTestId("lead-board").getByText("Alix Chen"),
    ).toBeVisible();
    await expect(
      page.getByTestId("lead-board").getByText("referral · score 65"),
    ).toBeVisible();
  });

  test("ticket board renders columns with SLA badges", async ({ page }) => {
    await page.route("**/api/proxy/tickets", (route) =>
      route.fulfill({
        json: [
          {
            pid: "t1",
            title: "Login broken",
            priority: "high",
            status: "open",
            first_response_due_at: null,
            live_first_response_breached: true,
          },
        ],
      }),
    );
    await page.goto("/tickets/board");
    await expect(
      page.getByTestId("ticket-board").getByText("Login broken"),
    ).toBeVisible();
    await expect(
      page.getByTestId("ticket-board").getByText("high · SLA breached"),
    ).toBeVisible();
  });

  test("follow-ups page renders overdue aging and the calendar", async ({
    page,
  }) => {
    // The calendar always opens on today's month, so the stubbed
    // upcoming follow-up's due date is computed relative to the
    // actual test-run date — a fixed date would eventually scroll
    // outside the widget's default view.
    const today = new Date();
    const dueInThisMonth = new Date(today.getFullYear(), today.getMonth(), 18)
      .toISOString()
      .slice(0, 10);
    await page.route("**/api/proxy/insights/followups", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          note: "actor_ref is the recording actor",
          overdue: [
            {
              pid: "a1",
              kind: "call",
              summary: "Chase the proposal",
              subject_kind: "deal",
              subject_pid: "d1",
              actor_ref: "worker:x",
              due_on: "2026-07-01",
              overdue_days: 19,
            },
          ],
          upcoming_30d: [
            {
              pid: "a2",
              kind: "meeting",
              summary: "QBR",
              subject_kind: "account",
              subject_pid: "ac1",
              actor_ref: null,
              due_on: dueInThisMonth,
              overdue_days: null,
            },
          ],
          open_by_recorder: { "worker:x": 1, unattributed: 1 },
        },
      }),
    );
    await page.goto("/followups");
    await expect(
      page.getByTestId("followups-overdue").getByText("Chase the proposal"),
    ).toBeVisible();
    await expect(
      page.getByTestId("followups-overdue").getByText("19"),
    ).toBeVisible();
    const calendar = page.getByTestId("followups-calendar");
    await expect(calendar).toBeVisible();
    // Pins the widget actually rendering the event, not just its
    // wrapper — `@svar-ui/calendar-store` silently drops an all-day
    // event whose `end` is not strictly after `start` (see the fix in
    // +page.svelte), and the wrapper alone renders regardless.
    await expect(calendar.getByText("meeting: QBR", { exact: false })).toBeVisible();
  });

  test("executive area renders the pack, hygiene findings, and trends", async ({
    page,
  }) => {
    await page.route("**/api/proxy/insights/executive", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          window: { from: "2026-06-20T00:00:00Z", to: "2026-07-20T00:00:00Z" },
          deals_won: 3,
          deals_lost: 1,
          won_value_by_currency_minor: { GBP: 750000 },
          lost_reasons: { price: 1 },
          new_leads: 12,
          tickets_opened: 5,
          tickets_resolved: 4,
          campaigns_started: 1,
          activities_logged: 40,
          consent_withdrawals: 2,
          note: "per-currency won value is never merged",
        },
      }),
    );
    await page.route("**/api/proxy/insights/stale-deals", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          derivation: "stage entry = newest deal_stage_changed audit",
          threshold_days: 14,
          open_deals: 2,
          stale_deals: 1,
          deals: [
            {
              pid: "d9",
              name: "Sleepy deal",
              stage: "Proposal",
              owner_ref: null,
              amount_minor: 100000,
              currency: "GBP",
              days_in_stage: 30,
              stale: true,
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/insights/pipeline-hygiene", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          threshold_days: 14,
          findings: [
            {
              rule: "open_deal_without_amount",
              detail: "an open deal carries no amount (forecast blind spot)",
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/insights/forecast-trends", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          note: "stored snapshots only; no interpolated history",
          series: [{ taken_on: "2026-07-19", totals: { GBP: 4887500 } }],
        },
      }),
    );
    await page.goto("/executive");
    await expect(
      page.getByTestId("exec-won-value").getByText("£7,500.00"),
    ).toBeVisible();
    await expect(
      page.getByTestId("exec-stale").getByText("Sleepy deal"),
    ).toBeVisible();
    await expect(
      page.getByTestId("exec-hygiene").getByText("open_deal_without_amount"),
    ).toBeVisible();
    await expect(
      page.getByTestId("exec-trends").getByText("£48,875.00"),
    ).toBeVisible();
  });

  test("dpo area renders coverage, sources, and duplicate hygiene", async ({
    page,
  }) => {
    await page.route("**/api/proxy/insights/consent-by-account", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          window_days: 30,
          note: "coverage counts verbatim",
          accounts: [
            {
              pid: "ac1",
              display_name: "Meridian University",
              consent_coverage: { granted: 3 },
              withdrawals_in_window: 1,
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/insights/dpo", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          note: "identity dedup stays upstream in the person service",
          contacts: 3,
          consent_coverage: { granted: 2, withdrawn: 1 },
          window_days: 30,
          withdrawals_in_window: 1,
          consent_events_by_source: { "web form": 2, "email link": 1 },
          duplicate_person_refs: [
            {
              person_ref: "person:abc",
              contacts: [
                { pid: "c1", display_name: "Row One" },
                { pid: "c2", display_name: "Row Two" },
              ],
            },
          ],
        },
      }),
    );
    await page.goto("/dpo");
    await expect(
      page.getByTestId("dpo-tiles").getByText("consent: granted"),
    ).toBeVisible();
    await expect(
      page.getByTestId("dpo-sources").getByText("web form"),
    ).toBeVisible();
    await expect(
      page.getByTestId("dpo-duplicates").getByText("person:abc"),
    ).toBeVisible();
  });

  test("deal board funnel strip shows honest conversion ratios", async ({
    page,
  }) => {
    await page.route("**/api/proxy/pipelines", (route) =>
      route.fulfill({
        json: [
          { pipeline: { pid: "p1", name: "New Business" }, stages: STAGES },
        ],
      }),
    );
    await page.route("**/api/proxy/deals**", (route) =>
      route.fulfill({ json: [DEAL] }),
    );
    await page.route("**/api/proxy/insights/funnel**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          pipeline: { pid: "p1", name: "New Business" },
          derivation: "entered from recorded to_stage audits",
          stages: [
            {
              stage: "Qualification",
              position: 0,
              is_won: false,
              is_lost: false,
              entered: 4,
              conversion_from_previous: null,
            },
            {
              stage: "Proposal",
              position: 1,
              is_won: false,
              is_lost: false,
              entered: 2,
              conversion_from_previous: {
                numerator: 2,
                denominator: 4,
                value: 0.5,
              },
            },
          ],
        },
      }),
    );
    await page.goto("/deals");
    await expect(page.getByTestId("pipeline-select")).toBeVisible();
    await expect(
      page.getByTestId("deal-funnel").getByText("50% (2/4)"),
    ).toBeVisible();
  });

  test("engagement area renders cadence, workload, and member health", async ({
    page,
  }) => {
    await page.route("**/api/proxy/insights/cadence**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          derivation: "touch = a recorded activity",
          threshold_days: 30,
          untouched_contacts: [
            {
              pid: "c9",
              display_name: "Silent Sam",
              stakeholder_role: "regulator",
              days_since_touch: 44,
              has_next_touch: false,
            },
          ],
          untouched_accounts: [],
          contacts_without_next_touch: 5,
        },
      }),
    );
    await page.route("**/api/proxy/insights/engagement**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          window_days: 90,
          touches: 12,
          per_recorder_month: { "worker:x 2026-07": 12 },
          per_kind: { meeting: 7, call: 5 },
          sentiment: { positive: 4, unrecorded: 8 },
          note: "recorded declarations only",
        },
      }),
    );
    await page.route("**/api/proxy/insights/members**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          derivation: "account touch includes its contacts' activities",
          threshold_days: 30,
          silent_accounts: 1,
          accounts: [
            {
              pid: "ac1",
              display_name: "Meridian University",
              tier: "standard",
              stakeholder_role: "partner",
              membership: {
                status: "active",
                joined_on: "2024-01-01",
                renewal_on: "2026-08-01",
              },
              contacts: 3,
              days_since_touch: 2,
              silent: false,
              open_followups: 1,
              open_tickets: 0,
            },
          ],
        },
      }),
    );
    await page.goto("/engagement");
    await expect(
      page.getByTestId("cadence-contacts").getByText("Silent Sam"),
    ).toBeVisible();
    await expect(
      page.getByTestId("workload-sentiment").getByText("unrecorded"),
    ).toBeVisible();
    await expect(
      page.getByTestId("member-health").getByText("Meridian University"),
    ).toBeVisible();
  });

  test("partners area renders the register, grid, partnerships, and renewals", async ({
    page,
  }) => {
    await page.route("**/api/proxy/insights/stakeholders", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          note: "declared, never inferred",
          by_role: {
            partner: [
              {
                pid: "c1",
                display_name: "Prof Reyes",
                marketing_consent: "granted",
                influence: 4,
                interest: 5,
                days_since_touch: 3,
              },
            ],
          },
          grid: { p4i5: 1 },
          stakeholders_without_grid_scores: 0,
          undeclared_contacts: 7,
          account_roles: [
            {
              pid: "ac1",
              display_name: "Meridian University",
              role: "partner",
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/insights/partnerships", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          by_kind: { university: 1 },
          by_stage: { pilot: 1 },
          register: [
            {
              pid: "pn1",
              account_pid: "ac1",
              account: "Meridian University",
              kind: "university",
              stage: "pilot",
              summary: "Joint ML lab",
              started_on: null,
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/insights/memberships**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          window_days: 90,
          memberships: 1,
          renewals_due: [
            {
              pid: "m1",
              account: "Meridian University",
              status: "active",
              joined_on: "2024-01-01",
              renewal_on: "2026-08-01",
            },
          ],
          lapsed: [],
        },
      }),
    );
    await page.goto("/partners");
    await expect(
      page.getByTestId("stakeholder-register").getByText("Prof Reyes"),
    ).toBeVisible();
    await expect(page.getByTestId("stakeholder-grid")).toBeVisible();
    await expect(
      page.getByTestId("partnership-register").getByText("Joint ML lab"),
    ).toBeVisible();
    await expect(
      page.getByTestId("membership-renewals").getByText("2026-08-01"),
    ).toBeVisible();
  });
});
