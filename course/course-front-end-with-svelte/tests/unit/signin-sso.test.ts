// Pins `/signin/sso`'s gating + redirect behaviour and `oidcLoginUrl`
// (EV-2, agents/share/authentication-sessions.md §7a).
import { describe, expect, it, vi, beforeEach } from "vitest";
import { isHttpError, isRedirect } from "@sveltejs/kit";
import { oidcLoginUrl } from "../../src/lib/server/auth";

const mockEnv: { PUBLIC_OIDC_SIGNIN_ENABLED?: string } = {};

vi.mock("$app/env/public", () => ({
    get PUBLIC_OIDC_SIGNIN_ENABLED() {
        return mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED;
    },
}));

const origin = "https://course-front-end.example.test";

describe("oidcLoginUrl", () => {
  it("points at the auth service's /api/auth/oidc/login", () => {
    expect(oidcLoginUrl(origin)).toContain("/api/auth/oidc/login?");
  });

  it("carries the encoded origin as return_url", () => {
    const url = oidcLoginUrl(origin);
    expect(url).toContain(`return_url=${encodeURIComponent(origin)}`);
    expect(new URL(url).searchParams.get("return_url")).toBe(origin);
  });
});

describe("GET /signin/sso", () => {
  beforeEach(() => {
    delete mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED;
  });

  it("404s when PUBLIC_OIDC_SIGNIN_ENABLED is unset", async () => {
    const { GET } = await import("../../src/routes/signin/sso/+server");
    const url = new URL(`${origin}/signin/sso`);
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
    const url = new URL(`${origin}/signin/sso`);
    try {
      GET({ url } as Parameters<typeof GET>[0]);
      throw new Error("expected GET to throw");
    } catch (err) {
      expect(isRedirect(err)).toBe(true);
      if (isRedirect(err)) {
        expect(err.status).toBe(303);
        expect(err.location).toBe(oidcLoginUrl(origin));
      }
    }
  });
});
