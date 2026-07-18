// E2E walk over the whiteboard flows (PF-T16). The backend is stubbed
// with `page.route` mirroring the real endpoint contract (case
// front-end precedent), so a wrong path/method fails loud without the
// Rust service running.

import { test, expect, type Page } from "@playwright/test";

const WARD = "11111111-1111-4111-8111-111111111111";
const STAY = "22222222-2222-4222-8222-222222222222";
const BED_FREE = "33333333-3333-4333-8333-333333333333";
const REQUEST = "44444444-4444-4444-8444-444444444444";
const PERSON = "person:55555555-5555-4555-8555-555555555555";

const CARD_OCCUPIED = {
  bed_pid: "66666666-6666-4666-8666-666666666666",
  bay_name: "Bay A",
  number: "W7-A-1",
  state: "occupied",
  state_since: "2026-07-18T08:00:00Z",
  closure_reason: null,
  deep_clean_required: false,
  side_room: false,
  stay_pid: STAY,
  display_name: "Test Patient 001",
  named_nurse_ref: null,
  consultant_ref: null,
  edd: "2026-07-20",
  edd_missing: false,
  edd_overdue: false,
  ccd_met: true,
  discharge_pathway: "p1",
  discharge_ready: true,
  dtoc: false,
  senior_review_today: true,
  red_green_today: "green",
  infection: [
    { precaution: "droplet", organism: "covid-19", status: "suspected" },
  ],
  alerts: ["falls risk"],
};

const CARD_FREE = {
  ...CARD_OCCUPIED,
  bed_pid: BED_FREE,
  number: "W7-A-2",
  state: "available",
  stay_pid: null,
  display_name: null,
  infection: [],
  alerts: [],
  red_green_today: null,
  discharge_ready: false,
  ccd_met: false,
  discharge_pathway: null,
  senior_review_today: false,
  edd: null,
};

const WHITEBOARD = {
  ward_pid: WARD,
  ward_name: "Ward 7 — Respiratory",
  ward_code: "W7",
  kind: "inpatient",
  closed_to_admissions: false,
  escalation: false,
  as_of: "2026-07-18T09:00:00Z",
  masked: false,
  cards: [CARD_OCCUPIED, CARD_FREE],
};

const GLANCE = {
  as_of: "2026-07-18T09:00:00Z",
  wards: [
    {
      ward_pid: WARD,
      site_pid: "s",
      name: "Ward 7 — Respiratory",
      code: "W7",
      kind: "inpatient",
      escalation: false,
      closed_to_admissions: false,
      beds_total: 14,
      occupied: 10,
      available: 4,
      reserved: 0,
      awaiting_clean: 0,
      cleaning: 0,
      closed: 0,
      closed_for_infection: 0,
      occupancy_pct: 71.4,
      expected_discharges_today: 3,
      edd_overdue: 1,
      discharge_ready: 2,
      dtoc: 1,
      open_requests_targeting: 1,
      long_stay_7: 2,
      long_stay_21: 0,
    },
  ],
  site_tiles: {
    available_now: 16,
    predicted_available_by_midnight: 24,
    open_requests: { emergency: 0, urgent: 1, routine: 0 },
    dtoc: 1,
    virtual_ward_census: 14,
    escalation_beds_open: 14,
  },
};

const STAY_DETAIL = {
  stay: {
    pid: STAY,
    person_ref: PERSON,
    display_name: "Test Patient 001",
    status: "admitted",
    admitted_at: "2026-07-15T12:00:00Z",
    source: "ed",
    ward_pid: WARD,
    bed_pid: CARD_OCCUPIED.bed_pid,
    home_location_note: null,
    named_nurse_ref: null,
    consultant_ref: null,
    senior_review_at: "2026-07-18T08:30:00Z",
    edd: "2026-07-20",
    ccd: "clinically stable",
    ccd_met: true,
    discharge_pathway: null,
    discharge_ready_at: null,
    discharged_at: null,
    discharge_destination: null,
    alerts: ["falls risk"],
  },
  transfers: [
    {
      pid: "t1",
      stay_pid: STAY,
      from_bed_pid: null,
      to_bed_pid: CARD_OCCUPIED.bed_pid,
      reason: "admission",
      moved_at: "2026-07-15T12:00:00Z",
    },
  ],
  red_green: [
    { stay_pid: STAY, day: "2026-07-16", classification: "red", delay_reasons: ["awaiting_diagnostics"], note: null },
    { stay_pid: STAY, day: "2026-07-17", classification: "green", delay_reasons: [], note: null },
  ],
  infection_flags: [],
  length_of_stay_days: 3,
  dtoc: false,
};

const OPEN_REQUEST = {
  pid: REQUEST,
  person_ref: PERSON,
  origin: "ed",
  target_ward_pid: WARD,
  specialty: null,
  priority: "urgent",
  status: "open",
  allocated_bed_pid: null,
  requested_at: "2026-07-18T08:45:00Z",
  resolved_at: null,
  eligible_beds: 1,
};

const ELIGIBLE = [
  {
    bed_pid: BED_FREE,
    number: "W7-A-2",
    ward_pid: WARD,
    ward_code: "W7",
    bay_name: "Bay A",
    side_room: false,
    right_ward: true,
  },
];

/** Stub the proxied patient-flow API; unmatched calls 404 loudly. */
async function stubApi(page: Page) {
  await page.route("**/api/proxy/**", async (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname.replace("/api/proxy", "");
    const method = route.request().method();

    if (path === "/api/at-a-glance") return route.fulfill({ json: GLANCE });
    if (path === `/api/whiteboard/${WARD}`)
      return route.fulfill({ json: WHITEBOARD });
    if (path === `/api/stays/${STAY}` && method === "GET")
      return route.fulfill({ json: STAY_DETAIL });
    if (path === "/api/wards")
      return route.fulfill({
        json: [
          {
            pid: WARD,
            site_pid: "s",
            name: "Ward 7 — Respiratory",
            code: "W7",
            kind: "inpatient",
            specialty: "respiratory",
            open: true,
            escalation: false,
            closed_to_admissions: false,
          },
        ],
      });
    if (path === "/api/bed-requests" && method === "GET")
      return route.fulfill({ json: [OPEN_REQUEST] });
    if (path === `/api/bed-requests/${REQUEST}/eligible`)
      return route.fulfill({ json: ELIGIBLE });
    if (path === `/api/bed-requests/${REQUEST}/allocate` && method === "POST")
      return route.fulfill({
        json: { ...OPEN_REQUEST, status: "allocated", allocated_bed_pid: BED_FREE },
      });
    if (path.startsWith("/api/locate/"))
      return route.fulfill({
        json: {
          person_ref: PERSON,
          display_name: "Test Patient 001",
          status: "admitted",
          stay_pid: STAY,
          site: "St Elsewhere General",
          ward: { pid: WARD, name: "Ward 7 — Respiratory", code: "W7", kind: "inpatient" },
          bay: "Bay A",
          bed: "W7-A-1",
          home_location_note: null,
          discharged_at: null,
        },
      });
    if (path === "/api/audits/recent") return route.fulfill({ json: [] });

    return route.fulfill({ status: 404, json: { error: `unstubbed ${method} ${path}` } });
  });
}

test("home lists wards and links to the whiteboard", async ({ page }) => {
  await stubApi(page);
  await page.goto("/");
  await expect(page.getByRole("cell", { name: "Ward 7 — Respiratory" })).toBeVisible();
  await page.getByRole("link", { name: "whiteboard" }).click();
  await expect(page.getByRole("heading", { name: /whiteboard/ })).toBeVisible();
  await expect(page.getByTestId("as-of")).toBeVisible();
});

test("whiteboard renders bed cards with journey chips", async ({ page }) => {
  await stubApi(page);
  await page.goto(`/wards/${WARD}/whiteboard`);
  const occupiedCard = page.locator('[data-bed="W7-A-1"]');
  await expect(occupiedCard.getByText("Test Patient 001")).toBeVisible();
  await expect(occupiedCard.getByText("EDD 2026-07-20")).toBeVisible();
  await expect(occupiedCard.getByText("Ready")).toBeVisible();
  await expect(occupiedCard.getByText(/covid-19/)).toBeVisible();
  await expect(occupiedCard.getByText("falls risk")).toBeVisible();
  const freeCard = page.locator('[data-bed="W7-A-2"]');
  await expect(freeCard.getByText("Available")).toBeVisible();
});

test("tapping a patient opens the stay detail", async ({ page }) => {
  await stubApi(page);
  await page.goto(`/wards/${WARD}/whiteboard`);
  await page.getByText("Test Patient 001").click();
  await expect(page).toHaveURL(`/stays/${STAY}`);
  await expect(page.getByRole("heading", { name: "Test Patient 001" })).toBeVisible();
  await expect(page.getByText("LOS 3d")).toBeVisible();
  await expect(page.locator("span.chip", { hasText: "CCD met" })).toBeVisible();
  // The Red2Green run renders one chip per day.
  await expect(page.getByTitle("2026-07-16")).toHaveText("R");
  await expect(page.getByTitle("2026-07-17")).toHaveText("G");
});

test("kiosk masked mode hides names but keeps bed states", async ({ page }) => {
  await stubApi(page);
  await page.goto(`/wards/${WARD}/kiosk?masked=1`);
  await expect(page.locator("body")).toHaveClass(/kiosk/);
  await expect(page.getByText("•••")).toBeVisible();
  await expect(page.getByText("Test Patient 001")).not.toBeVisible();
  await expect(page.getByText("falls risk")).not.toBeVisible();
  await expect(page.locator('[data-bed="W7-A-2"]').getByText("Available")).toBeVisible();
});

test("at-a-glance shows the site tiles", async ({ page }) => {
  await stubApi(page);
  await page.goto("/at-a-glance");
  await expect(page.getByText("Beds available now")).toBeVisible();
  await expect(page.getByText("Virtual ward census")).toBeVisible();
  await expect(page.getByRole("cell", { name: "71.4%" })).toBeVisible();
});

test("bed-request board shows eligible beds and allocates", async ({ page }) => {
  await stubApi(page);
  await page.goto("/bed-requests");
  await expect(page.locator("span.chip", { hasText: "urgent" })).toBeVisible();
  await page.getByRole("button", { name: "Show beds" }).click();
  const bedButton = page.getByRole("button", { name: /W7 · W7-A-2/ });
  await expect(bedButton).toBeVisible();
  await bedButton.click(); // allocate; the stub answers and the list reloads
  await expect(page.getByRole("heading", { name: "Bed requests" })).toBeVisible();
});

test("locate finds a patient and links to the stay", async ({ page }) => {
  await stubApi(page);
  await page.goto("/locate");
  await page.getByPlaceholder("person:<uuid>").fill(PERSON);
  await page.getByRole("button", { name: "Locate" }).click();
  const result = page.getByTestId("locate-result");
  await expect(result.getByText("W7 — Ward 7 — Respiratory")).toBeVisible();
  await expect(result.getByText("bed W7-A-1")).toBeVisible();
  await result.getByRole("link", { name: "Open stay" }).click();
  await expect(page).toHaveURL(`/stays/${STAY}`);
});
