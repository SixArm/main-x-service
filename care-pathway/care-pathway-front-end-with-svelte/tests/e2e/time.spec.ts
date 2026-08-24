// Time-based-analysis view smoke test over a page.route-stubbed API
// (mirroring the service contract; unmatched calls 404 so drift fails
// loud). No Rust service needed.

import { test, expect, type Page } from "@playwright/test";

const PATHWAY = "44444444-4444-4444-8444-444444444444";
const INSTANCE = "55555555-5555-4555-8555-555555555555";
const DAY = 86_400_000;
const T0 = Date.UTC(2026, 0, 1);

const STANDARDS = {
  note: "reference data with a citation date",
  standards: [
    {
      id: "rtt_18_weeks",
      label: "Referral to treatment, incomplete pathway",
      threshold_ms: 126 * DAY,
      target_ratio: 0.92,
      authority: "NHS England",
      as_of: "2026-08",
      note: "18 weeks",
    },
  ],
  vocabularies: { stages: [], categories: [], wastes: [] },
};

/** A cohort large enough that percentile detail is not suppressed. */
const COHORT = {
  as_of: "2026-08-23T12:00:00Z",
  pathway: { pid: PATHWAY, name: "Suspected stroke" },
  note: "coverage over condition codes",
  suppressed: false,
  suppression_note: null,
  cohort: {
    instances: 22,
    lead_time: {
      n: 22,
      min_ms: 4 * DAY,
      p50_ms: 61 * DAY,
      p75_ms: 90 * DAY,
      p90_ms: 141 * DAY,
      p95_ms: 180 * DAY,
      max_ms: 240 * DAY,
      mean_ms: 78 * DAY,
      p50_days: 61,
      p90_days: 141,
      method: "nearest_rank",
    },
    aggregate_value_adding_ratio: {
      value: 0.098,
      numerator_ms: 168 * DAY,
      denominator_ms: 1716 * DAY,
    },
    median_value_adding_ratio: 0.41,
    waste_shape: "concentrated",
    coverage_ratio: { value: 0.72, numerator_ms: 1, denominator_ms: 1 },
    by_stage: [],
    by_waste: [],
  },
  compliance: {
    standard: "rtt_18_weeks",
    threshold_ms: 126 * DAY,
    threshold_days: 126,
    within: 18,
    breached: 4,
    achieved_ratio: 0.818,
    target_ratio: 0.92,
    target_met: false,
    as_of: "2026-08",
  },
};

const CONSTRAINTS = {
  as_of: "2026-08-23T12:00:00Z",
  pathway: { pid: PATHWAY, name: "Suspected stroke" },
  note: "findings ordered by recoverable time",
  instances: 22,
  findings: [
    {
      rule: "stage_dominates_waste",
      subject: "diagnostics",
      detail:
        "58.0% of the cohort's non-value-adding time sits in `diagnostics`",
      recoverable_ms: 900 * DAY,
      recoverable_days: 900,
    },
    {
      rule: "longest_gap",
      subject: "referral received → triaged",
      detail:
        "the longest single stretch in which nothing was recorded: 44.0 days",
      recoverable_ms: 44 * DAY,
      recoverable_days: 44,
    },
  ],
};

const FLOW = {
  as_of: "2026-08-23T12:00:00Z",
  window_since: "2026-05-25",
  note: "Little's Law used as a consistency check",
  instances_considered: 22,
  flow: {
    window_days: 90,
    arrivals: 30,
    closures: 18,
    arrival_rate_per_day: 0.33,
    service_rate_per_day: 0.2,
    utilisation: 1.65,
    utilisation_reason: null,
    work_in_progress: 12,
    implied_lead_time_days: 36.4,
    observed_p50_lead_time_days: 61,
    interpretation: "queue_draining",
    detail: "Little's Law implies 36.4 days against 61.0 observed.",
  },
};

const INSTANCES = [
  {
    pid: INSTANCE,
    pathway_pid: PATHWAY,
    subject_ref: "person:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
    status: "completed",
    urgency: "urgent",
    enrolled_on: "2026-01-01",
    next_review_on: null,
    closed_on: "2026-04-11",
    closure_reason: null,
    outcome: "improved",
  },
];

const CLOCK = {
  start_ms: T0,
  stop_ms: T0 + 100 * DAY,
  start_source: "clock_start_at",
  stop_source: "clock_stop_at",
  running: false,
};

/** A 100-day journey with 14 days of care — the Barker case. */
const TIMELINE = {
  as_of: "2026-08-23T12:00:00Z",
  instance: { pid: INSTANCE, status: "completed" },
  clock: CLOCK,
  note: "segments and gaps interleaved in time order",
  totals: {
    lead_time_ms: 100 * DAY,
    lead_time_days: 100,
    value_adding_ratio: {
      value: 0.14,
      numerator_ms: 14 * DAY,
      denominator_ms: 100 * DAY,
    },
    coverage_ratio: {
      value: 0.67,
      numerator_ms: 67 * DAY,
      denominator_ms: 100 * DAY,
    },
    confidence: "partial",
  },
  wall: [
    {
      kind: "segment",
      pid: "s1",
      label: "First consultation",
      stage: "treatment",
      category: "value_adding",
      waste: null,
      started_at: "2026-01-01T00:00:00Z",
      ended_at: "2026-01-08T00:00:00Z",
      open: false,
      actor_ref: "worker:gp",
      location_ref: null,
      duration_ms: 7 * DAY,
      duration_days: 7,
    },
    {
      kind: "segment",
      pid: "s2",
      label: "Wait for scan slot",
      stage: "diagnostics",
      category: "unnecessary_non_value_adding",
      waste: "waiting",
      started_at: "2026-01-08T00:00:00Z",
      ended_at: "2026-03-01T00:00:00Z",
      open: false,
      actor_ref: null,
      location_ref: null,
      duration_ms: 53 * DAY,
      duration_days: 53,
    },
    {
      kind: "segment",
      pid: "s3",
      label: "Scan",
      stage: "diagnostics",
      category: "value_adding",
      waste: null,
      started_at: "2026-03-01T00:00:00Z",
      ended_at: "2026-03-08T00:00:00Z",
      open: false,
      actor_ref: "worker:rad",
      location_ref: "place:x",
      duration_ms: 7 * DAY,
      duration_days: 7,
    },
    {
      kind: "gap",
      label: "Scan → clock stop",
      stage: "diagnostics",
      duration_ms: 33 * DAY,
      duration_days: 33,
      at_handoff: false,
    },
  ],
};

const ANALYSIS = {
  as_of: "2026-08-23T12:00:00Z",
  instance: { pid: INSTANCE, status: "completed" },
  note: "value_adding_ratio is value time over elapsed calendar time",
  analysis: {
    clock: CLOCK,
    lead_time_ms: 100 * DAY,
    lead_time_days: 100,
    value_time_ms: 14 * DAY,
    process_time_ms: 14 * DAY,
    waste_time_ms: 53 * DAY,
    touch_time_ms: 67 * DAY,
    wait_time_ms: 86 * DAY,
    unrecorded_ms: 33 * DAY,
    value_adding_ratio: {
      value: 0.14,
      numerator_ms: 14 * DAY,
      denominator_ms: 100 * DAY,
    },
    activity_ratio: {
      value: 0.14,
      numerator_ms: 14 * DAY,
      denominator_ms: 100 * DAY,
    },
    coverage_ratio: {
      value: 0.67,
      numerator_ms: 67 * DAY,
      denominator_ms: 100 * DAY,
    },
    confidence: "partial",
    segments: 3,
    by_category: [
      { category: "value_adding", ms: 14 * DAY, days: 14, share: 0.14 },
      { category: "necessary_non_value_adding", ms: 0, days: 0, share: 0 },
      {
        category: "unnecessary_non_value_adding",
        ms: 53 * DAY,
        days: 53,
        share: 0.53,
      },
      { category: "unrecorded", ms: 33 * DAY, days: 33, share: 0.33 },
    ],
    by_stage: [],
    by_waste: [{ waste: "waiting", ms: 53 * DAY, segments: 1 }],
    handoffs: {
      actor_changes: 2,
      location_changes: 1,
      total: 2,
      distinct_actors: 2,
      distinct_locations: 1,
      gap_ms_at_handoffs: 0,
    },
    gaps: [
      {
        start_ms: T0 + 67 * DAY,
        end_ms: T0 + 100 * DAY,
        duration_ms: 33 * DAY,
        days: 33,
        after: "Scan",
        before: null,
        stage: "diagnostics",
        at_handoff: false,
      },
    ],
    reason: null,
  },
};

async function stubTime(page: Page) {
  await page.route("**/api/**", async (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname.startsWith("/api/proxy")
      ? url.pathname.slice("/api/proxy".length)
      : url.pathname;
    const json = (body: unknown) => route.fulfill({ json: body });

    if (path === "/api/care-pathways")
      return json([{ pid: PATHWAY, name: "Suspected stroke" }]);
    if (path === "/api/instances/time-standards") return json(STANDARDS);
    if (path === `/api/care-pathways/${PATHWAY}/time-analysis`) {
      // The service scores a standard only when one is asked for, so
      // the stub does too — otherwise the test would pass against a
      // contract the service does not have.
      const asked = url.searchParams.get("standard");
      return json(asked ? COHORT : { ...COHORT, compliance: null });
    }
    if (path === `/api/care-pathways/${PATHWAY}/constraints`)
      return json(CONSTRAINTS);
    if (path === "/api/instances/flow") return json(FLOW);
    if (path === `/api/care-pathways/${PATHWAY}/instances`)
      return json(INSTANCES);
    if (path === `/api/instances/${INSTANCE}/timeline`) return json(TIMELINE);
    if (path === `/api/instances/${INSTANCE}/time-analysis`)
      return json(ANALYSIS);
    return route.fulfill({
      status: 404,
      json: { error: "unhandled in stub", path },
    });
  });
}

test.describe("time-based analysis", () => {
  test("shows the cohort, the standard, and the journey wall", async ({
    page,
  }) => {
    await stubTime(page);
    await page.goto("/time");

    // The cohort headline, and the typical journey beside it.
    await expect(page.getByTestId("cohort-ratio")).toContainText("9.8%");
    await expect(page.getByTestId("cohort-ratio")).toContainText("41.0%");
    await expect(page.getByText(/concentrated/)).toBeVisible();

    // No standard is selected by default, so nothing is scored — a
    // pathway is not automatically subject to any of them.
    await expect(page.getByTestId("compliance")).toHaveCount(0);

    // Scored against a named NHS standard, with the target verdict and
    // the date the threshold was last checked — targets move.
    await page.getByLabel("Access standard").selectOption("rtt_18_weeks");
    const compliance = page.getByTestId("compliance");
    await expect(compliance).toContainText("Referral to treatment");
    await expect(compliance).toContainText("82%");
    await expect(compliance).toContainText("not met");
    await expect(compliance).toContainText("2026-08");

    // Lead time by percentile, never by mean.
    await expect(page.getByTestId("lead-time")).toContainText("61.0d");

    // The journey: 14 days of care in 100.
    await expect(page.getByTestId("journey-ratio")).toContainText("14.0%");
    await expect(page.getByTestId("journey-ratio")).toContainText("8–14%");

    // Coverage is 67%, so the figure is a floor, and the page says so
    // rather than presenting it as fully evidenced.
    await expect(page.getByTestId("confidence")).toContainText("67%");
    await expect(page.getByTestId("confidence")).toContainText("floor");

    // The wall names its longest band without labelling every band.
    await expect(page.getByText(/Longest single stretch/)).toContainText(
      "Wait for scan slot",
    );

    // The biggest queue is named by what it sits between.
    await expect(page.getByTestId("gaps")).toContainText("Scan → clock stop");

    // Little's Law is stated in words.
    await expect(page.getByTestId("littles-law")).toContainText(
      "Queue draining",
    );
  });

  test("the wall carries a table view — the palette's relief rule", async ({
    page,
  }) => {
    await stubTime(page);
    await page.goto("/time");

    await expect(page.getByTestId("wall-table")).toHaveCount(0);
    await page.getByRole("button", { name: /Show the numbers/ }).click();
    const table = page.getByTestId("wall-table");
    await expect(table).toBeVisible();
    // Unrecorded time is a row like any other: dropping it is how an
    // unmapped journey comes to look efficient.
    await expect(table).toContainText("Unrecorded");
    await expect(table).toContainText("33.0");
  });

  test("every band is keyboard reachable, not hover-only", async ({ page }) => {
    await stubTime(page);
    await page.goto("/time");
    const bands = page.locator(".band");
    await expect(bands).toHaveCount(4);
    await bands.first().focus();
    await expect(page.getByRole("status")).toContainText("First consultation");
  });
});
