// Pins `/signin/sso` gating + redirect (EV-2, authentication-sessions.md
// §7a) and `oidcLoginUrl`.
import { describe, expect, it, vi, beforeEach } from "vitest";
import { isHttpError, isRedirect } from "@sveltejs/kit";
import { oidcLoginUrl } from "../../src/lib/server/auth";

const mockEnv: { PUBLIC_OIDC_SIGNIN_ENABLED?: string } = {};

vi.mock("$env/dynamic/public", () => ({ env: mockEnv }));

describe("oidcLoginUrl", () => {
  it("points at the auth service's OIDC login with an encoded return_url", () => {
    const url = oidcLoginUrl("https://wpm.example.test");
    expect(url).toContain("/api/auth/oidc/login?");
    expect(url).toContain(
      `return_url=${encodeURIComponent("https://wpm.example.test")}`,
    );
  });
});

describe("GET /signin/sso", () => {
  beforeEach(() => {
    delete mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED;
  });

  it("404s when PUBLIC_OIDC_SIGNIN_ENABLED is unset", async () => {
    const { GET } = await import("../../src/routes/signin/sso/+server");
    const url = new URL("https://wpm.example.test/signin/sso");
    try {
      GET({ url } as Parameters<typeof GET>[0]);
      throw new Error("expected GET to throw");
    } catch (err) {
      expect(isHttpError(err)).toBe(true);
      if (isHttpError(err)) expect(err.status).toBe(404);
    }
  });

  it("303s to the OIDC login URL when enabled", async () => {
    mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED = "true";
    const { GET } = await import("../../src/routes/signin/sso/+server");
    const url = new URL("https://wpm.example.test/signin/sso");
    try {
      GET({ url } as Parameters<typeof GET>[0]);
      throw new Error("expected GET to throw");
    } catch (err) {
      expect(isRedirect(err)).toBe(true);
      if (isRedirect(err)) {
        expect(err.status).toBe(303);
        expect(err.location).toBe(oidcLoginUrl("https://wpm.example.test"));
      }
    }
  });
});
