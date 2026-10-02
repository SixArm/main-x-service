// Pins `oidcLoginUrl` and `/signin/sso`'s gating + redirect behaviour
// (EV-2): 404 unless PUBLIC_OIDC_SIGNIN_ENABLED is "true", else a 303 to
// the auth service's OIDC login carrying this app's origin as return_url.
import { describe, expect, it, vi, beforeEach } from "vitest";
import { isHttpError, isRedirect } from "@sveltejs/kit";

const mockEnv: { PUBLIC_OIDC_SIGNIN_ENABLED?: string } = {};

vi.mock("$app/env/public", () => ({
    get PUBLIC_OIDC_SIGNIN_ENABLED() {
        return mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED;
    },
}));

const ORIGIN = "https://care-pathway.example.test";

describe("oidcLoginUrl", () => {
  it("builds the auth service OIDC login URL with an encoded return_url", async () => {
    const { oidcLoginUrl } = await import("../../src/lib/server/auth");
    expect(oidcLoginUrl(ORIGIN)).toBe(
      `http://localhost:5150/api/auth/oidc/login?return_url=${encodeURIComponent(ORIGIN)}`,
    );
  });
});

describe("GET /signin/sso", () => {
  beforeEach(() => {
    delete mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED;
  });

  it("404s when PUBLIC_OIDC_SIGNIN_ENABLED is unset", async () => {
    const { GET } = await import("../../src/routes/signin/sso/+server");
    const url = new URL(`${ORIGIN}/signin/sso`);
    try {
      GET({ url } as Parameters<typeof GET>[0]);
      throw new Error("expected GET to throw");
    } catch (err) {
      expect(isHttpError(err)).toBe(true);
      if (isHttpError(err)) expect(err.status).toBe(404);
    }
  });

  it("303s to the OIDC login with return_url=this origin when enabled", async () => {
    mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED = "true";
    const { GET } = await import("../../src/routes/signin/sso/+server");
    const url = new URL(`${ORIGIN}/signin/sso`);
    try {
      GET({ url } as Parameters<typeof GET>[0]);
      throw new Error("expected GET to throw");
    } catch (err) {
      expect(isRedirect(err)).toBe(true);
      if (isRedirect(err)) {
        expect(err.status).toBe(303);
        expect(err.location).toContain("/api/auth/oidc/login?");
        expect(err.location).toContain(encodeURIComponent(ORIGIN));
      }
    }
  });
});
