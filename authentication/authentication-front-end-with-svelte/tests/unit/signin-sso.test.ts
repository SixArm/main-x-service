// Pins `/signin/sso`'s gating + redirect behaviour (EV-2,
// agents/share/authentication-sessions.md §7a): a deployment that has
// not opted in via PUBLIC_OIDC_SIGNIN_ENABLED sees a 404 rather than a
// dead link into an unconfigured auth service; an opted-in deployment
// gets a 303 to the auth service's own OIDC login endpoint, carrying
// this app's origin as return_url.
import { describe, expect, it, vi, beforeEach } from "vitest";
import { isHttpError, isRedirect } from "@sveltejs/kit";

const mockEnv: { PUBLIC_OIDC_SIGNIN_ENABLED?: string } = {};

vi.mock("$env/dynamic/public", () => ({ env: mockEnv }));

describe("GET /signin/sso", () => {
    beforeEach(() => {
        delete mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED;
    });

    it("404s when PUBLIC_OIDC_SIGNIN_ENABLED is unset", async () => {
        const { GET } = await import("../../src/routes/signin/sso/+server");
        const url = new URL("https://auth-front-end.example.test/signin/sso");
        try {
            GET({ url } as Parameters<typeof GET>[0]);
            throw new Error("expected GET to throw");
        } catch (err) {
            expect(isHttpError(err)).toBe(true);
            if (isHttpError(err)) expect(err.status).toBe(404);
        }
    });

    it("303s to the auth service's OIDC login with return_url=this origin when enabled", async () => {
        mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED = "true";
        const { GET } = await import("../../src/routes/signin/sso/+server");
        const url = new URL("https://auth-front-end.example.test/signin/sso");
        try {
            GET({ url } as Parameters<typeof GET>[0]);
            throw new Error("expected GET to throw");
        } catch (err) {
            expect(isRedirect(err)).toBe(true);
            if (isRedirect(err)) {
                expect(err.status).toBe(303);
                expect(err.location).toContain("/api/auth/oidc/login?");
                expect(err.location).toContain(
                    encodeURIComponent("https://auth-front-end.example.test"),
                );
            }
        }
    });
});
