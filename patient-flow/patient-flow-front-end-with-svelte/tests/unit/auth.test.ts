// Unit tests for the BFF's server-side calls to the authentication
// service (PF-T18, PF-T23): magic-link request/verify and the
// session→PASETO exchange, against a mocked `fetch`. No SvelteKit
// runtime or real authentication service involved.
import { describe, expect, it, vi } from "vitest";
import {
  exchangeToken,
  requestMagicLink,
  signout,
  verifyMagicLink,
} from "../../src/lib/server/auth";
import { SESSION_COOKIE } from "../../src/lib/server/session";

describe("verifyMagicLink", () => {
  it("GETs the magic-link consume endpoint with the encoded token", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));
    await verifyMagicLink(fetchSpy, "a token/with?chars");
    expect(fetchSpy).toHaveBeenCalledWith(
      expect.stringContaining(
        `/api/auth/magic-link/${encodeURIComponent("a token/with?chars")}`,
      ),
    );
  });

  it("returns the raw upstream response so the caller can read Set-Cookie", async () => {
    const upstream = new Response(null, {
      status: 200,
      headers: { "set-cookie": `${SESSION_COOKIE}=abc` },
    });
    const fetchSpy = vi.fn().mockResolvedValue(upstream);
    const result = await verifyMagicLink(fetchSpy, "tok");
    expect(result).toBe(upstream);
  });
});

describe("requestMagicLink", () => {
  it("POSTs email, locale, and return_url as JSON", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));
    await requestMagicLink(fetchSpy, "a@example.test", "http://localhost/verify", "en");
    expect(fetchSpy).toHaveBeenCalledWith(
      expect.stringContaining("/api/auth/magic-link"),
      expect.objectContaining({
        method: "POST",
        headers: { "content-type": "application/json" },
      }),
    );
    const body = JSON.parse(fetchSpy.mock.calls[0]?.[1]?.body as string);
    expect(body).toEqual({
      email: "a@example.test",
      locale: "en",
      return_url: "http://localhost/verify",
    });
  });

  it("resolves true on a 2xx upstream response", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(new Response(null, { status: 200 }));
    await expect(
      requestMagicLink(fetchSpy, "a@example.test", "http://localhost/verify"),
    ).resolves.toBe(true);
  });

  it("resolves false on a non-ok upstream response", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(new Response(null, { status: 429 }));
    await expect(
      requestMagicLink(fetchSpy, "a@example.test", "http://localhost/verify"),
    ).resolves.toBe(false);
  });
});

describe("exchangeToken", () => {
  it("sends the session id as a Cookie header and returns the minted token", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ token: "v4.public.xyz" }), { status: 200 }),
    );
    const token = await exchangeToken(fetchSpy, "sid-123");
    expect(token).toBe("v4.public.xyz");
    expect(fetchSpy).toHaveBeenCalledWith(
      expect.stringContaining("/api/auth/token"),
      expect.objectContaining({
        method: "POST",
        headers: { cookie: `${SESSION_COOKIE}=sid-123` },
      }),
    );
  });

  it("returns null when the upstream responds not-ok", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(new Response(null, { status: 401 }));
    await expect(exchangeToken(fetchSpy, "sid-123")).resolves.toBeNull();
  });

  it("returns null when the upstream body carries no token", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({}), { status: 200 }),
    );
    await expect(exchangeToken(fetchSpy, "sid-123")).resolves.toBeNull();
  });
});

describe("signout", () => {
  it("does nothing further when the session cannot be exchanged for a token", async () => {
    const fetchSpy = vi.fn().mockResolvedValue(new Response(null, { status: 401 }));
    await signout(fetchSpy, "sid-123");
    // Only the exchangeToken call happened; no signout POST was attempted.
    expect(fetchSpy).toHaveBeenCalledTimes(1);
  });

  it("POSTs to /api/auth/signout with a bearer token once exchanged", async () => {
    const fetchSpy = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ token: "v4.public.xyz" }), { status: 200 }),
      )
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    await signout(fetchSpy, "sid-123");
    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(fetchSpy).toHaveBeenNthCalledWith(
      2,
      expect.stringContaining("/api/auth/signout"),
      expect.objectContaining({
        method: "POST",
        headers: { authorization: "Bearer v4.public.xyz" },
      }),
    );
  });
});
