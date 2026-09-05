// Dashboard smoke over a stubbed API (CMS-T25).
//
// Every route the page needs is stubbed explicitly; anything else 404s
// loudly, so a page that starts calling an endpoint nobody declared
// fails here rather than in production.

import { expect, test, type Page } from "@playwright/test";

/**
 * Every page but /signin and /verify is gated on a session (CMS-T31).
 * The server only checks the cookie's *presence*, never its validity,
 * so a fabricated value passes the gate without a real
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

test.beforeEach(async ({ page }) => {
  await signIn(page);
});

const SITE = {
  pid: "site-1",
  key: "demo",
  name: "Demo site",
  default_locale: "en",
  locales: ["en", "fr"],
  fallback_chains: { fr: ["en"] },
  visibility: "restricted",
  base_url: null,
};

const HEALTH = {
  as_of: "2026-07-31T09:00:00Z",
  site: "demo",
  entries: 29,
  published_variants: 39,
  findings_total: 15,
  orphan_bytes: 26_788_000,
  by_rule: [
    {
      rule: "image_alt_text_missing",
      explanation: "a published page shows an image whose alt text is empty",
      count: 1,
      findings: [],
    },
  ],
};

const BACKLOG = {
  as_of: "2026-07-31T09:00:00Z",
  pending_review: [{}],
  pending_schedule: [],
  open_translations: [{}, {}],
};

/**
 * Stub the API.
 *
 * The catch-all is registered **first** on purpose: Playwright matches
 * routes in reverse registration order, so registering it last would
 * shadow every specific stub and 404 the whole page — which is exactly
 * what happened the first time this suite ran.
 */
async function stub(page: Page, health: unknown) {
  // Anything not stubbed below is a test failure, loudly.
  await page.route("**/api/proxy/**", (route) =>
    route.fulfill({ status: 404, json: { error: "unstubbed" } }),
  );
  await page.route("**/api/proxy/api/sites", (route) =>
    route.fulfill({ json: [SITE] }),
  );
  await page.route("**/api/proxy/api/sites/site-1/insights/health", (route) =>
    route.fulfill({ json: health }),
  );
  await page.route("**/api/proxy/api/sites/site-1/insights/backlog", (route) =>
    route.fulfill({ json: BACKLOG }),
  );
}

test("the dashboard renders what the service reports, and names the rule", async ({
  page,
}) => {
  await stub(page, HEALTH);

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
  await expect(page.getByText("29")).toBeVisible();
  await expect(page.getByText("15")).toBeVisible();
  // The rule that produced a finding travels with the finding.
  await expect(page.getByText("image_alt_text_missing")).toBeVisible();
  await expect(
    page.getByText("a published page shows an image whose alt text is empty"),
  ).toBeVisible();
  // A derived view says when it was derived.
  await expect(page.getByText(/As of/)).toBeVisible();
});

test("an empty health view says so instead of showing nothing", async ({
  page,
}) => {
  await stub(page, { ...HEALTH, findings_total: 0, by_rule: [] });

  await page.goto("/");
  await expect(page.getByText("Nothing to report")).toBeVisible();
});
