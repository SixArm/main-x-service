// Unit tests for the BFF session helpers. No SvelteKit runtime involved.
import { describe, expect, it } from "vitest";
import { isRedirect } from "@sveltejs/kit";
import { requireSignedIn } from "../../src/lib/server/session";

describe("requireSignedIn", () => {
  // Pins: PRO-H10's page-visit guard — a signed-out visitor is redirected
  // to /signin, never allowed to fall through to the page's own load.
  it("redirects to /signin when no session is present", () => {
    try {
      requireSignedIn({ sessionId: null });
      throw new Error("expected requireSignedIn to throw a redirect");
    } catch (error) {
      expect(isRedirect(error)).toBe(true);
      if (isRedirect(error)) {
        expect(error.status).toBe(303);
        expect(error.location).toBe("/signin");
      }
    }
  });

  // Pins: a signed-in visitor (any non-null session id) passes through
  // silently — no throw, no return value to check.
  it("does not throw when a session is present", () => {
    expect(() => requireSignedIn({ sessionId: "some-session-id" })).not.toThrow();
  });
});
