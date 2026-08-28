// Integration-style tests for the BFF reverse proxy's CSRF gate
// (routes/api/proxy/[...path]/+server.ts). Exercises the exported route
// handlers directly with a hand-built `RequestEvent`-shaped object,
// stubbing just the pieces the CSRF check touches (`cookies.get`) plus a
// `fetch` that fails the test if the CSRF gate lets a bad request through
// to the "upstream".
import { describe, expect, it, vi } from "vitest";
import { DELETE, GET, POST } from "../../src/routes/api/proxy/[...path]/+server";

const CSRF_COOKIE = "__Host-mxi_csrf";

// Minimal stand-in for SvelteKit's `RequestEvent`, covering only the
// fields the proxy handler reads.
function makeEvent(opts: {
  method: string;
  headers?: Record<string, string>;
  cookieValue?: string | undefined;
  origin?: string;
}) {
  const headers = new Headers(opts.headers ?? {});
  const request = new Request(`${opts.origin ?? "http://localhost:5173"}/api/proxy/api/courses`, {
    method: opts.method,
    headers,
  });
  const fetchSpy = vi.fn(async () => new Response(null, { status: 200 }));
  return {
    event: {
      request,
      params: { path: "api/courses" },
      url: new URL(request.url),
      locals: { sessionId: null },
      fetch: fetchSpy,
      cookies: {
        get: (name: string) => (name === CSRF_COOKIE ? opts.cookieValue : undefined),
      },
      // Fields unused by the handler but present on a real RequestEvent;
      // omitted here since the handler never touches them.
    } as unknown as Parameters<typeof POST>[0],
    fetchSpy,
  };
}

describe("proxy CSRF gate", () => {
  it("never blocks GET regardless of CSRF header/cookie state", async () => {
    const { event, fetchSpy } = makeEvent({ method: "GET" });
    const response = await GET(event);
    expect(response.status).not.toBe(403);
    expect(fetchSpy).toHaveBeenCalled();
  });

  it("rejects POST with no CSRF cookie or header", async () => {
    const { event, fetchSpy } = makeEvent({ method: "POST" });
    const response = await POST(event);
    expect(response.status).toBe(403);
    expect(await response.json()).toMatchObject({ error: "csrf" });
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("rejects POST with a mismatched header token", async () => {
    const { event, fetchSpy } = makeEvent({
      method: "POST",
      cookieValue: "token-a",
      headers: { "x-csrf-token": "token-b" },
    });
    const response = await POST(event);
    expect(response.status).toBe(403);
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("accepts POST with a matching header token and no Origin/Referer", async () => {
    const { event, fetchSpy } = makeEvent({
      method: "POST",
      cookieValue: "token-a",
      headers: { "x-csrf-token": "token-a" },
    });
    const response = await POST(event);
    expect(response.status).not.toBe(403);
    expect(fetchSpy).toHaveBeenCalled();
  });

  it("accepts POST with a matching header token and a same-origin Origin header", async () => {
    const { event, fetchSpy } = makeEvent({
      method: "POST",
      cookieValue: "token-a",
      headers: {
        "x-csrf-token": "token-a",
        origin: "http://localhost:5173",
      },
    });
    const response = await POST(event);
    expect(response.status).not.toBe(403);
    expect(fetchSpy).toHaveBeenCalled();
  });

  it("rejects POST with a matching token but a cross-site Origin header", async () => {
    const { event, fetchSpy } = makeEvent({
      method: "POST",
      cookieValue: "token-a",
      headers: {
        "x-csrf-token": "token-a",
        origin: "https://evil.example",
      },
    });
    const response = await POST(event);
    expect(response.status).toBe(403);
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("rejects DELETE with a matching token but a cross-site Referer header", async () => {
    const { event, fetchSpy } = makeEvent({
      method: "DELETE",
      cookieValue: "token-a",
      headers: {
        "x-csrf-token": "token-a",
        referer: "https://evil.example/steal",
      },
    });
    const response = await DELETE(event);
    expect(response.status).toBe(403);
    expect(fetchSpy).not.toHaveBeenCalled();
  });
});
