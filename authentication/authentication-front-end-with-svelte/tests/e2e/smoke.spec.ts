// Pins the end-to-end SPA behaviour with the auth API stubbed: the
// signup/signin forms render and submit, the verify route consumes a token
// and lands on the dashboard, the signed-in/signed-out home views, and —
// crucially — the cross-origin handoff: an allowlisted (self-origin)
// return_to receives the token in the URL fragment, while a foreign origin
// is ignored and the token is never handed off.
import { test, expect, type Page } from "@playwright/test";

// Smoke tests over the auth SPA routes. The auth API is stubbed per-test
// via `page.route`, so a broken endpoint contract (wrong path / method)
// surfaces as an unhandled request and a failing assertion, without
// needing the Rust service.

const PID = "11111111-1111-4111-8111-111111111111";
const EMAIL = "alice@example.com";
const NAME = "Alice";
const TOKEN = "test-access-token";

const CURRENT_USER = { pid: PID, name: NAME, email: EMAIL };
const LOGIN_RESPONSE = {
  token: TOKEN,
  pid: PID,
  name: NAME,
  email: EMAIL,
  is_verified: true,
};

/** Stub every `/api/auth/*` call so the SPA renders offline. */
async function stubApi(page: Page) {
  await page.route("**/api/auth/**", async (route) => {
    const req = route.request();
    const url = new URL(req.url());
    const method = req.method();
    const path = url.pathname;

    if (path === "/api/auth/signup" && method === "POST") {
      return route.fulfill({ status: 200, body: "" });
    }
    if (path === "/api/auth/magic-link" && method === "POST") {
      return route.fulfill({ status: 200, body: "" });
    }
    if (path.startsWith("/api/auth/magic-link/") && method === "GET") {
      return route.fulfill({ json: LOGIN_RESPONSE });
    }
    if (path === "/api/auth/me" && method === "GET") {
      return route.fulfill({ json: CURRENT_USER });
    }
    if (path === "/api/auth/signout" && method === "POST") {
      return route.fulfill({ status: 200, body: "" });
    }
    return route.fulfill({ status: 404, json: { error: "unhandled in stub" } });
  });
}

/** Seed a signed-in session into localStorage before the app boots. */
async function seedSession(page: Page) {
  await page.addInitScript(
    ([token, user]) => {
      localStorage.setItem("mxi.auth.token", token as string);
      localStorage.setItem("mxi.auth.user", user as string);
    },
    [TOKEN, JSON.stringify(CURRENT_USER)],
  );
}

test.beforeEach(async ({ page }) => {
  await stubApi(page);
});

test("sign-up page shows the create-account form", async ({ page }) => {
  await page.goto("/signup", { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Create account" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Send magic link" }),
  ).toBeVisible();
});

test("sign-in page shows the magic-link request form", async ({ page }) => {
  await page.goto("/signin", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Email me a magic link" }),
  ).toBeVisible();
});

test("sign-in submits the email and confirms the link was sent", async ({
  page,
}) => {
  await page.goto("/signin", { waitUntil: "networkidle" });
  await page.getByLabel("Email").fill(EMAIL);
  await page.getByRole("button", { name: "Email me a magic link" }).click();
  await expect(page.getByText(/a magic link is on its way/)).toBeVisible();
});

test("verify route consumes the token and lands on the signed-in dashboard", async ({
  page,
}) => {
  await page.goto(`/verify?token=magic-123`, { waitUntil: "networkidle" });
  // On success the page stores the session and redirects to "/".
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByRole("heading", { name: "Account" })).toBeVisible();
  await expect(page.getByRole("main").getByText(EMAIL)).toBeVisible();
});

test("verify hands the token to an allowlisted return_to via the URL fragment", async ({
  page,
}) => {
  // Self-origin is always allowed (empty allowlist ⇒ same-origin only),
  // so we can exercise the cross-origin handoff decision without build
  // env config: park a same-origin return_to, then verify.
  await page.addInitScript(() => {
    sessionStorage.setItem("mxi_return_to", location.origin + "/signin");
  });
  await page.goto("/verify?token=magic-123", { waitUntil: "networkidle" });
  // The page redirects to return_to with the token in the URL fragment.
  await expect(page).toHaveURL(/\/signin#access_token=test-access-token/);
});

test("verify ignores a non-allowlisted return_to and never appends the token", async ({
  page,
}) => {
  await page.addInitScript(() => {
    sessionStorage.setItem("mxi_return_to", "https://evil.example.com/grab");
  });
  await page.goto("/verify?token=magic-123", { waitUntil: "networkidle" });
  // Falls through to home; the token is not handed off.
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByRole("heading", { name: "Account" })).toBeVisible();
});

test("verify route reports an error when the token is missing", async ({
  page,
}) => {
  await page.goto("/verify", { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Could not sign you in" }),
  ).toBeVisible();
});

test("home page renders the account dashboard for a signed-in session", async ({
  page,
}) => {
  await seedSession(page);
  await page.goto("/", { waitUntil: "networkidle" });
  await expect(page.getByRole("heading", { name: "Account" })).toBeVisible();
  await expect(page.getByRole("main").getByText(EMAIL)).toBeVisible();
  await expect(page.getByRole("button", { name: "Sign out" })).toBeVisible();
});

test("home page shows the signed-out state without a session", async ({
  page,
}) => {
  await page.goto("/", { waitUntil: "networkidle" });
  await expect(page.getByText("You are not signed in.")).toBeVisible();
  await expect(
    page.getByRole("main").getByRole("link", { name: "Sign in" }),
  ).toBeVisible();
});
