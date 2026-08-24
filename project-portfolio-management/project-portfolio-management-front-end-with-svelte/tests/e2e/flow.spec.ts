// Time-based-analysis view smoke test over a page.route-stubbed API
// (mirroring the service contract; unmatched calls 404 so drift fails
// loud). No Rust service needed.

import { test, expect, type Page } from "@playwright/test";

const PLAN = "33333333-3333-4333-8333-333333333333";
const DAY = 86_400_000;
const T0 = Date.UTC(2026, 6, 1);

/** A plan whose waste is concentrated: most items flow, one is stuck. */
const PLAN_ANALYSIS = {
  as_of: "2026-08-23T12:00:00Z",
  plan: { pid: PLAN, name: "Platform rebuild" },
  note: "cycle-time percentiles are nearest-rank",
  classification: {
    classes: {
      todo: "unnecessary_non_value_adding",
      in_progress: "value_adding",
      in_review: "necessary_non_value_adding",
      blocked: "unnecessary_non_value_adding",
    },
    overridden: false,
    source: "the disclosed default",
  },
  service_level_expectation: {
    percentile: 0.85,
    within_ms: 11 * DAY,
    within_days: 11,
    sample: 14,
    reason: null,
    target_days: null,
    target_achieved_ratio: null,
    target_met: null,
  },
  plan_analysis: {
    tasks: 20,
    finished: 14,
    work_in_progress: 4,
    not_started: 2,
    cycle_time: {
      n: 14,
      min_ms: DAY,
      p50_ms: 4 * DAY,
      p75_ms: 8 * DAY,
      p85_ms: 11 * DAY,
      p95_ms: 30 * DAY,
      max_ms: 44 * DAY,
      mean_ms: 9 * DAY,
      p50_days: 4,
      p85_days: 11,
      method: "nearest_rank",
    },
    lead_time: {
      n: 14,
      min_ms: 2 * DAY,
      p50_ms: 12 * DAY,
      p75_ms: 20 * DAY,
      p85_ms: 26 * DAY,
      p95_ms: 50 * DAY,
      max_ms: 70 * DAY,
      mean_ms: 19 * DAY,
      p50_days: 12,
      p85_days: 26,
      method: "nearest_rank",
    },
    aggregate_flow_efficiency: {
      value: 0.071,
      numerator_ms: 10 * DAY,
      denominator_ms: 141 * DAY,
    },
    median_flow_efficiency: 0.34,
    waste_shape: "concentrated",
    rolled_first_pass_yield: 0.79,
    rework_count: 3,
    by_status: [],
    backfilled_ratio: 0.12,
  },
};

const AGING = {
  as_of: "2026-08-23T12:00:00Z",
  plan: { pid: PLAN, name: "Platform rebuild" },
  note: "open items ranked by age, scored against the plan's own expectation",
  classification: PLAN_ANALYSIS.classification,
  service_level_expectation: PLAN_ANALYSIS.service_level_expectation,
  aging: [
    {
      task: {
        pid: "t1",
        title: "Migrate the auth adapter",
        status: "blocked",
        assignee_ref: "worker:ada",
      },
      status: "blocked",
      aging: { age_ms: 31 * DAY, age_days: 31, past_sle: true, sle_ratio: 2.8 },
      blocked_time_ms: 19 * DAY,
      rework_count: 2,
    },
    {
      task: {
        pid: "t2",
        title: "Backfill the search index",
        status: "in_review",
        assignee_ref: null,
      },
      status: "in_review",
      aging: { age_ms: 6 * DAY, age_days: 6, past_sle: false, sle_ratio: 0.55 },
      blocked_time_ms: 0,
      rework_count: 0,
    },
  ],
};

const CONSTRAINTS = {
  as_of: "2026-08-23T12:00:00Z",
  plan: { pid: PLAN, name: "Platform rebuild" },
  note: "findings ordered by recoverable time",
  classification: PLAN_ANALYSIS.classification,
  tasks: 20,
  findings: [
    {
      rule: "status_dominates_wait",
      subject: "in_review",
      detail: "52.0% of the plan's non-value-adding time sits in `in_review`",
      recoverable_ms: 62 * DAY,
      recoverable_days: 62,
    },
    {
      rule: "blocked_time",
      subject: "blocked",
      detail: "time halted on something external",
      recoverable_ms: 24 * DAY,
      recoverable_days: 24,
    },
    {
      rule: "rework",
      subject: "3 backwards moves",
      detail: "rolled first pass yield 79%",
      recoverable_ms: 0,
      recoverable_days: 0,
    },
  ],
};

const FLOW = {
  as_of: "2026-08-23T12:00:00Z",
  plan: { pid: PLAN, name: "Platform rebuild" },
  window_since: "2026-06-24T12:00:00Z",
  note: "Little's Law used as a consistency check",
  flow: {
    window_days: 60,
    arrivals: 22,
    completions: 14,
    arrival_rate_per_day: 0.37,
    throughput_per_day: 0.23,
    utilisation: 1.61,
    utilisation_reason: null,
    work_in_progress: 4,
    implied_cycle_time_days: 17.4,
    observed_p50_cycle_time_days: 4,
    interpretation: "wip_growing",
    detail:
      "Little's Law implies 17.4 days for an item starting now, against 4.0 days observed.",
  },
  columns: [
    { status: "todo", count: 2, limit: null, over_limit: false },
    { status: "in_progress", count: 2, limit: 3, over_limit: false },
    { status: "in_review", count: 1, limit: 2, over_limit: false },
    { status: "done", count: 14, limit: null, over_limit: false },
    { status: "blocked", count: 1, limit: 1, over_limit: false },
  ],
};

/** Sixty daily samples of a board that grows, flows, and accumulates done. */
function cumulativeFlow() {
  const samples = [];
  for (let day = 0; day < 60; day += 1) {
    const total = Math.min(20, 4 + Math.floor(day / 3));
    const done = Math.max(0, Math.floor((day - 8) / 3));
    const inReview = day > 6 ? 2 : 0;
    const blocked = day > 20 && day < 45 ? 1 : 0;
    const inProgress = Math.min(
      3,
      Math.max(0, total - done - inReview - blocked),
    );
    const todo = Math.max(0, total - done - inReview - blocked - inProgress);
    samples.push({
      at_ms: T0 + day * DAY,
      counts: {
        todo,
        in_progress: inProgress,
        in_review: inReview,
        done,
        blocked,
      },
      total,
      done,
      work_in_progress: inProgress + inReview + blocked,
    });
  }
  return {
    as_of: "2026-08-23T12:00:00Z",
    plan: { pid: PLAN, name: "Platform rebuild" },
    days: 60,
    note: "the board's composition sampled daily",
    classification: PLAN_ANALYSIS.classification,
    samples,
  };
}

async function stubFlow(page: Page) {
  await page.route("**/api/**", async (route) => {
    const url = new URL(route.request().url());
    // Strip the BFF proxy prefix: the browser's real request is
    // `/api/proxy/api/...`, not the bare `/api/...` this stub matches.
    const path = url.pathname.replace(/^\/api\/proxy/, "");
    if (path === `/api/plans/${PLAN}/time-analysis`)
      return route.fulfill({ json: PLAN_ANALYSIS });
    if (path === `/api/plans/${PLAN}/aging-wip`)
      return route.fulfill({ json: AGING });
    if (path === `/api/plans/${PLAN}/constraints`)
      return route.fulfill({ json: CONSTRAINTS });
    if (path === `/api/plans/${PLAN}/flow`)
      return route.fulfill({ json: FLOW });
    if (path === `/api/plans/${PLAN}/cumulative-flow`)
      return route.fulfill({ json: cumulativeFlow() });
    return route.fulfill({ status: 404, json: { error: `unstubbed ${path}` } });
  });
}

test.describe("time-based analysis", () => {
  test("shows the expectation, the aging items, and the chart", async ({
    page,
  }) => {
    await stubFlow(page);
    await page.goto(`/plans/${PLAN}/flow`);

    // The expectation is the number a team can actually quote.
    await expect(page.getByTestId("sle-badge")).toContainText("11.0d");
    await expect(page.getByTestId("sle-badge")).toContainText(
      "14 finished items",
    );

    // A 7.1% flow efficiency is typical, not alarming — the copy must
    // say so rather than leaving the reader to panic.
    await expect(page.getByTestId("flow-efficiency")).toContainText("7.1%");
    await expect(page.getByTestId("flow-efficiency")).toContainText("Typical");

    // The concentrated-waste finding fires, because that is a different
    // fix from uniformly slow delivery.
    await expect(page.getByText(/concentrated/)).toBeVisible();

    // Aging WIP flags the item past the expectation.
    const aging = page.getByTestId("aging-wip");
    await expect(aging).toContainText("Migrate the auth adapter");
    await expect(aging).toContainText("31.0d");
    await expect(aging).toContainText("280%");

    // Little's Law is stated in words, not just as a ratio.
    await expect(page.getByTestId("littles-law")).toContainText(
      "Work in progress is growing",
    );

    // Constraints are ranked by recoverable time.
    await expect(page.getByTestId("constraints")).toContainText(
      "status_dominates_wait",
    );

    // Cycle and lead time are always shown together.
    const distributions = page.getByTestId("distributions");
    await expect(distributions).toContainText("Cycle time");
    await expect(distributions).toContainText("Lead time");
  });

  test("the chart carries a table view — the palette's relief rule", async ({
    page,
  }) => {
    await stubFlow(page);
    await page.goto(`/plans/${PLAN}/flow`);

    // Three light-mode series sit below 3:1 against the surface, so the
    // numbers must be reachable without reading colour.
    await expect(page.getByTestId("cfd-table")).toHaveCount(0);
    await page.getByRole("button", { name: /Show the numbers/ }).click();
    await expect(page.getByTestId("cfd-table")).toBeVisible();

    // The chart itself is labelled for a screen reader.
    await expect(
      page.getByRole("img", { name: /Cumulative flow over/ }),
    ).toBeVisible();
  });
});
