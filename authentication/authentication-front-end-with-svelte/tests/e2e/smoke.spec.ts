// Pins the BFF SPA behaviour against a real (if tiny) auth-service stub:
// the signup/signin forms render and submit, the verify route consumes a
// token server-side and lands on the signed-in dashboard, and the home
// page's signed-in/signed-out views render correctly.
//
// Every auth-service call this app makes happens server-side (the
// SvelteKit BFF's own `fetch`, src/lib/server/auth.ts) — never from the
// browser — so `page.route()` cannot stub it (it only intercepts
// browser-issued requests). `AUTH_API_URL` is instead pointed, via
// `playwright.config.ts`'s second `webServer` entry, at
// `tests/e2e/mock-auth-server.mjs`, a real Node HTTP server implementing
// the handful of endpoints the BFF calls. See spec §11/§13.
//
// The prior cross-origin `return_to` handoff and the `localStorage`
// bearer-token model it and the seeded-session test assumed were removed
// with the BFF migration (`authentication-sessions.md`; spec §13,
// 2026-08-04); those three cases are deleted rather than fixed, not
// merely skipped — there is no return_to handoff and no client-held
// session left to test.
import { test, expect } from "@playwright/test";

// Must match tests/e2e/mock-auth-server.mjs's fixture data.
const EMAIL = "alice@example.com";
const VALID_MAGIC_TOKEN = "magic-123";
const NETWORK_ERROR_MAGIC_TOKEN = "magic-network-error";
const RATE_LIMITED_EMAIL = "toomany@example.com";

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

// Pins AFE-4: a 429 from the auth service's rate limit renders a
// distinct "try again in a few minutes" message rather than the generic
// "Request failed" every other non-2xx outcome shows.
test("sign-in shows a distinct message when the auth service rate-limits the request", async ({
  page,
}) => {
  await page.goto("/signin", { waitUntil: "networkidle" });
  await page.getByLabel("Email").fill(RATE_LIMITED_EMAIL);
  await page.getByRole("button", { name: "Email me a magic link" }).click();
  await expect(
    page.getByText("Too many requests. Please wait a few minutes and try again."),
  ).toBeVisible();
  await expect(page.getByText("Request failed")).not.toBeVisible();
});

test("sign-up shows a distinct message when the auth service rate-limits the request", async ({
  page,
}) => {
  await page.goto("/signup", { waitUntil: "networkidle" });
  await page.getByLabel("Email").fill(RATE_LIMITED_EMAIL);
  await page.getByRole("button", { name: "Send magic link" }).click();
  await expect(
    page.getByText("Too many requests. Please wait a few minutes and try again."),
  ).toBeVisible();
  await expect(page.getByText("Sign up failed")).not.toBeVisible();
});

test("verify route consumes the token and lands on the signed-in dashboard", async ({
  page,
}) => {
  // The mock auth server accepts exactly this token, sets the session +
  // CSRF cookies on its response, and the BFF's `+page.server.ts` load
  // re-hosts them on this origin before redirecting home — all server
  // side; the browser never sees a token.
  await page.goto(`/verify?token=${VALID_MAGIC_TOKEN}`, {
    waitUntil: "networkidle",
  });
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByRole("heading", { name: "Account" })).toBeVisible();
  await expect(page.getByRole("main").getByText(EMAIL)).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Sign out" }),
  ).toBeVisible();
});

test("verify route reports an error when the token is missing", async ({
  page,
}) => {
  await page.goto("/verify", { waitUntil: "networkidle" });
  await expect(
    page.getByRole("heading", { name: "Could not sign you in" }),
  ).toBeVisible();
});

// Pins the fix ported from place-front-end's T-26: before it, an
// unreachable authentication service made `load` throw uncaught, and
// SvelteKit rendered its generic 500 page instead of this route's own
// UI — confirmed directly by reproducing the scenario before fixing it.
test("verify route shows a friendly error when the auth service is unreachable, not a raw 500", async ({
  page,
}) => {
  const response = await page.goto(
    `/verify?token=${NETWORK_ERROR_MAGIC_TOKEN}`,
    { waitUntil: "networkidle" },
  );
  expect(response?.status()).toBe(200);
  await expect(
    page.getByRole("heading", { name: "Could not sign you in" }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "We could not reach the sign-in service. Please try again in a moment.",
    ),
  ).toBeVisible();
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
