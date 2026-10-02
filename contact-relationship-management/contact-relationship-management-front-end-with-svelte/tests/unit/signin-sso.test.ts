// Pins `/signin/sso` gating + redirect and `oidcLoginUrl` (EV-2).
import { describe, expect, it, vi, beforeEach } from "vitest";
import { isHttpError, isRedirect } from "@sveltejs/kit";

const mockEnv: { PUBLIC_OIDC_SIGNIN_ENABLED?: string } = {};

vi.mock("$app/env/public", () => ({
    get PUBLIC_OIDC_SIGNIN_ENABLED() {
        return mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED;
    },
}));

describe("oidcLoginUrl", () => {
  it("builds the auth service OIDC login URL with an encoded return_url", async () => {
    const { oidcLoginUrl } = await import("../../src/lib/server/auth");
    const u = oidcLoginUrl("http://localhost:5253");
    expect(u).toContain("/api/auth/oidc/login?return_url=");
    expect(u).toContain(encodeURIComponent("http://localhost:5253"));
  });
});

describe("GET /signin/sso", () => {
  beforeEach(() => {
    delete mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED;
  });

  it("404s when PUBLIC_OIDC_SIGNIN_ENABLED is unset", async () => {
    const { GET } = await import("../../src/routes/signin/sso/+server");
    const url = new URL("https://crm.example.test/signin/sso");
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
    const url = new URL("https://crm.example.test/signin/sso");
    try {
      GET({ url } as Parameters<typeof GET>[0]);
      throw new Error("expected GET to throw");
    } catch (err) {
      expect(isRedirect(err)).toBe(true);
      if (isRedirect(err)) {
        expect(err.status).toBe(303);
        expect(err.location).toContain("/api/auth/oidc/login?");
        expect(err.location).toContain(encodeURIComponent("https://crm.example.test"));
      }
    }
  });
});
