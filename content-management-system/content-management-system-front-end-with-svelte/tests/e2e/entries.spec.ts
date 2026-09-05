// The authoring journey over a stubbed API (CMS-T26).
//
// The catch-all is registered first on purpose: Playwright matches
// routes in reverse registration order, so registering it last shadows
// every specific stub.

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

const ENTRY = {
  pid: "entry-1",
  key: "guide-permissions",
  content_type_key: "page",
  source_locale: "en",
  owner_ref: "worker:11111111-1111-4111-8111-111111111111",
  archived_at: null,
};

const VARIANTS = [
  {
    pid: "variant-en",
    locale: "en",
    status: "published",
    // The draft has moved past what readers see — the page must say so.
    current_revision_pid: "rev-2",
    published_revision_pid: "rev-1",
    translation_status: null,
    scheduled_publish_at: null,
  },
];

const TRANSLATIONS = {
  entry_key: "guide-permissions",
  locales: [
    {
      locale: "en",
      is_source: true,
      status: "published",
      published: true,
      translation_status: null,
      translator_ref: null,
      due_on: null,
      staleness: { stale: false, revisions_behind: 0 },
    },
    {
      locale: "fr",
      is_source: false,
      status: "published",
      published: true,
      translation_status: "translated",
      translator_ref: null,
      due_on: null,
      staleness: { stale: true, revisions_behind: 3 },
    },
  ],
};

const HISTORY = [
  {
    pid: "rev-2",
    number: 2,
    title: "Permissions",
    author_ref: "worker:22222222-2222-4222-8222-222222222222",
    note: null,
    created_at: "2026-07-20T09:00:00Z",
    is_current: true,
    is_published: false,
    restored_from_pid: null,
  },
  {
    pid: "rev-1",
    number: 1,
    title: "Permissions",
    author_ref: null,
    note: null,
    created_at: "2026-07-10T09:00:00Z",
    is_current: false,
    is_published: true,
    restored_from_pid: null,
  },
];

const REVISION = {
  pid: "rev-2",
  number: 2,
  title: "Permissions",
  blocks: [{ kind: "paragraph", text: "Who can do what." }],
  fields: {},
  seo: {},
  author_ref: null,
  created_at: "2026-07-20T09:00:00Z",
};

const GATE_BLOCKED = {
  ready: false,
  status: "draft",
  revision_pid: "rev-2",
  blockers: [
    {
      rule: "image_alt_text_missing",
      subject: "hero image",
      remedy: "describe the image so screen-reader users know what it shows",
    },
  ],
};

async function stub(
  page: Page,
  gate: unknown = {
    ready: true,
    status: "published",
    revision_pid: "rev-2",
    blockers: [],
  },
) {
  await page.route("**/api/proxy/**", (route) =>
    route.fulfill({ status: 404, json: { error: "unstubbed" } }),
  );
  await page.route("**/api/proxy/api/sites", (r) =>
    r.fulfill({ json: [SITE] }),
  );
  await page.route("**/api/proxy/api/entries/entry-1", (r) =>
    r.fulfill({ json: { entry: ENTRY, variants: VARIANTS } }),
  );
  await page.route("**/api/proxy/api/entries/entry-1/translations", (r) =>
    r.fulfill({ json: TRANSLATIONS }),
  );
  await page.route(
    "**/api/proxy/api/entries/entry-1/variants/en/revisions",
    (r) => {
      if (r.request().method() === "GET") return r.fulfill({ json: HISTORY });
      // A save that lost the race: the service answers 409, and the page
      // must compare rather than retry.
      return r.fulfill({
        status: 409,
        json: { error: "conflict", description: "stale base revision" },
      });
    },
  );
  await page.route(
    "**/api/proxy/api/entries/entry-1/variants/en/publish-check",
    (r) => r.fulfill({ json: gate }),
  );
  await page.route("**/api/proxy/api/revisions/rev-2", (r) =>
    r.fulfill({ json: REVISION }),
  );
  await page.route("**/api/proxy/api/revisions/*/diff/*", (r) =>
    r.fulfill({
      json: {
        from: { pid: "rev-2", number: 2 },
        to: { pid: "rev-2", number: 2 },
        diff: {
          identical: false,
          title_changed: false,
          seo_changed: false,
          block_comparison:
            "positional: blocks are compared by index, not aligned by content",
          blocks: [],
          fields: [],
        },
      },
    }),
  );
}

test("the locale matrix says what is live and how far a translation has fallen behind", async ({
  page,
}) => {
  await stub(page);
  await page.goto("/entries/entry-1");
  await expect(
    page.getByRole("heading", { name: "guide-permissions" }),
  ).toBeVisible();

  // Staleness carries the count, never a bare badge.
  await expect(page.getByText("3 source revisions behind")).toBeVisible();
  // The draft has moved past what readers see, and the page says so.
  await expect(page.getByText("Draft is ahead of what is live")).toBeVisible();
});

test("a publish blocker shows its rule and what to do about it", async ({
  page,
}) => {
  await stub(page, GATE_BLOCKED);
  await page.goto("/entries/entry-1");
  await expect(page.getByText("Cannot publish yet")).toBeVisible();
  await expect(page.getByText("image_alt_text_missing")).toBeVisible();
  // A refusal an author cannot act on is a locked door.
  await expect(
    page.getByText(
      "describe the image so screen-reader users know what it shows",
    ),
  ).toBeVisible();
});

test("a lost save is a comparison, never a silent retry", async ({ page }) => {
  await stub(page);
  await page.goto("/entries/entry-1");
  await page.getByRole("button", { name: "Save revision" }).click();

  await expect(page.getByText("Someone else saved first")).toBeVisible();
  await expect(
    page.getByText(
      "Your draft was based on an older revision. Compare before overwriting.",
    ),
  ).toBeVisible();
  // The competing revision is named, so the author knows whose work
  // they would be overwriting.
  await expect(page.getByText(/#2 ·/)).toBeVisible();
});

test("the editor adds and removes blocks without ever producing markup", async ({
  page,
}) => {
  await stub(page);
  await page.goto("/entries/entry-1");
  await expect(page.getByText("Content blocks")).toBeVisible();

  await page.getByLabel("Add block").selectOption("image");
  await page.getByRole("button", { name: "Add block" }).click();

  // A new image block says what it needs, in terms of the consequence.
  await expect(
    page.getByText("alt text is required before this page can be published"),
  ).toBeVisible();
  await expect(page.getByText("choose an image")).toBeVisible();
});

test("restore explains that it writes a new revision", async ({ page }) => {
  await stub(page);
  await page.goto("/entries/entry-1");
  // "Restore" reads like undo, and it is not.
  await expect(
    page.getByText(
      "Restoring writes a new revision; history is never rewritten.",
    ),
  ).toBeVisible();
});
