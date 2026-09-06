// Unit tests for the /verify BFF load function, run directly (no browser,
// no server) — SvelteKit server `load` functions are plain async
// functions, so this is the cheapest way to pin the error-branch
// behaviour without standing up a whole Playwright + stub-auth-server
// harness. See `+page.server.ts`'s own comment for the bug this pins.
import { describe, expect, it, vi } from "vitest";
import { load } from "../../src/routes/verify/+page.server";

// `cookies.set` is never called on any of these error branches (it only
// runs after a successful upstream response), so a no-op stub is enough.
const cookies = { set: vi.fn() } as unknown as Parameters<
  typeof load
>[0]["cookies"];

describe("verify load", () => {
  it("returns missingToken when no token is present", async () => {
    const result = await load({
      url: new URL("http://localhost/verify"),
      fetch: vi.fn(),
      cookies,
    } as unknown as Parameters<typeof load>[0]);
    expect(result).toEqual({ error: "missingToken", title: "Sign-in link — Content Management System" });
  });

  // Pins the fix: a network-level failure (fetch rejects) must not
  // propagate out of `load` — it must resolve to a friendly error
  // state instead.
  it("returns serviceUnavailable when the upstream fetch rejects", async () => {
    const result = await load({
      url: new URL("http://localhost/verify?token=abc"),
      fetch: vi.fn().mockRejectedValue(new Error("fetch failed")),
      cookies,
    } as unknown as Parameters<typeof load>[0]);
    expect(result).toEqual({ error: "serviceUnavailable" });
  });

  it("returns invalidToken when the upstream responds not-ok", async () => {
    const result = await load({
      url: new URL("http://localhost/verify?token=abc"),
      fetch: vi.fn().mockResolvedValue(new Response(null, { status: 401 })),
      cookies,
    } as unknown as Parameters<typeof load>[0]);
    expect(result).toEqual({ error: "invalidToken", title: "Sign-in link — Content Management System" });
  });
});
