// AFE-3: end-to-end coverage of the `/admin/attributes` route, which
// (unlike every other route in `src/routes/`) had none. Same BFF
// constraint as tests/e2e/smoke.spec.ts: the admin calls happen
// server-side (src/lib/server/admin.ts), so `page.route()` cannot stub
// them — the mock auth server (tests/e2e/mock-auth-server.mjs) is
// extended with a session-aware `/api/auth/admin/users/{pid}/attributes`
// handler and a second, `access=admin` login identity instead.
//
// Covers: viewing an existing user's attributes as an admin, saving a
// valid change, the 403 path (signed in, but not an admin), and the 401
// path (no session at all).
import { test, expect } from "@playwright/test";

// Must match tests/e2e/mock-auth-server.mjs's fixture data.
const ADMIN_MAGIC_TOKEN = "magic-admin-456";
const VALID_MAGIC_TOKEN = "magic-123"; // signed in, but not an admin
const TARGET_PID = "33333333-3333-4333-8333-333333333333";
const TARGET_EMAIL = "target@example.com";

test("admin views an existing user's attributes", async ({ page }) => {
  await page.goto(`/verify?token=${ADMIN_MAGIC_TOKEN}`, {
    waitUntil: "networkidle",
  });
  await page.goto(`/admin/attributes?pid=${TARGET_PID}`, {
    waitUntil: "networkidle",
  });
  await expect(page.getByRole("heading", { name: TARGET_EMAIL })).toBeVisible();
  await expect(page.getByRole("textbox", { name: /Attributes/ })).toHaveValue(
    /"access"[\s\S]*"write"/,
  );
});

test("admin saves a valid attribute change", async ({ page }) => {
  await page.goto(`/verify?token=${ADMIN_MAGIC_TOKEN}`, {
    waitUntil: "networkidle",
  });
  await page.goto(`/admin/attributes?pid=${TARGET_PID}`, {
    waitUntil: "networkidle",
  });
  const editor = page.getByRole("textbox", { name: /Attributes/ });
  await editor.fill(JSON.stringify({ access: ["write"], dept: ["ops"] }));
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Attributes saved.")).toBeVisible();
  // The saved value round-trips into the editor, proving the PUT was
  // applied rather than merely acknowledged.
  await expect(editor).toHaveValue(/"dept"[\s\S]*"ops"/);
});

test("a signed-in, non-admin caller sees the 403 the service returns", async ({
  page,
}) => {
  await page.goto(`/verify?token=${VALID_MAGIC_TOKEN}`, {
    waitUntil: "networkidle",
  });
  await page.goto(`/admin/attributes?pid=${TARGET_PID}`, {
    waitUntil: "networkidle",
  });
  await expect(page.getByRole("alert")).toContainText("403");
  await expect(page.getByRole("alert")).toContainText(
    "caller does not carry access=admin",
  );
  // The denied caller never sees the target's data.
  await expect(page.getByText(TARGET_EMAIL)).not.toBeVisible();
});

test("an unauthenticated visitor is asked to sign in, not shown the target", async ({
  page,
}) => {
  await page.goto(`/admin/attributes?pid=${TARGET_PID}`, {
    waitUntil: "networkidle",
  });
  await expect(page.getByRole("alert")).toContainText(
    "Sign in as an admin to manage attributes.",
  );
  await expect(page.getByText(TARGET_EMAIL)).not.toBeVisible();
});
