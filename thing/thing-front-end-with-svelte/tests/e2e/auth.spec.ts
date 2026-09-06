// E2E coverage for /signin and /verify (T-23) — the magic-link BFF flow
// had zero Playwright coverage. The outbound calls these pages make
// happen server-side (src/lib/server/auth.ts), not in the browser, so
// they can't be stubbed with `page.route`; instead a real stub HTTP
// server (tests/e2e/auth-stub-server.ts) stands in for the authentication
// service, wired in via `AUTH_API_URL` in playwright.config.ts.
import { expect, test } from "@playwright/test";
import {
    EXPIRED_TOKEN,
    NETWORK_ERROR_TOKEN,
    VALID_TOKEN,
} from "./auth-stub-server";

test.describe("Sign-in / verify (T-23)", () => {
    test("signin renders the email form and shows the confirmation state after submitting", async ({
        page,
    }) => {
        await page.goto("/signin");
        await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
        const emailField = page.getByLabel("Email");
        await expect(emailField).toBeVisible();
        await emailField.fill("operator@example.com");
        await page.getByRole("button", { name: "Send magic link" }).click();
        await expect(
            page.getByText("Check your email for a sign-in link."),
        ).toBeVisible();
    });

    test("verify with no token shows a friendly missing-token message", async ({
        page,
    }) => {
        await page.goto("/verify");
        await expect(
            page.getByText("This sign-in link is missing its token."),
        ).toBeVisible();
    });

    test("verify with an expired/unknown token shows a friendly invalid-token message", async ({
        page,
    }) => {
        await page.goto(`/verify?token=${EXPIRED_TOKEN}`);
        await expect(
            page.getByText("This sign-in link is invalid or has expired."),
        ).toBeVisible();
    });

    // Pins the fix ported from place-front-end's T-26: before it, an
    // unreachable authentication service made `load` throw uncaught, and
    // SvelteKit rendered its generic 500 page instead of this route's own
    // UI — confirmed directly by reproducing the scenario before fixing it.
    test("verify with the authentication service unreachable shows a friendly error, not a raw 500", async ({
        page,
    }) => {
        const response = await page.goto(`/verify?token=${NETWORK_ERROR_TOKEN}`);
        expect(response?.status()).toBe(200);
        await expect(
            page.getByText(
                "We could not reach the sign-in service. Please try again in a moment.",
            ),
        ).toBeVisible();
    });

    test("verify with a valid token establishes the session and redirects home", async ({
        page,
    }) => {
        await page.goto(`/verify?token=${VALID_TOKEN}`);
        await expect(page).toHaveURL("/");
        const cookies = await page.context().cookies();
        expect(
            cookies.some(
                (c) =>
                    c.name === "__Host-mxi_session" && c.value === "stub-session-abc",
            ),
        ).toBe(true);
    });
});
