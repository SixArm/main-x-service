// Integration-style tests for the BFF reverse proxy
// (routes/api/proxy/[...path]/+server.ts), run directly against the
// exported route handler — no browser, no `page.route` interception,
// no running Rust service. This is PF-T23's other half: the vitest
// suites cover `session.ts`/`auth.ts` directly, and this exercises the
// actual proxy handler's header stripping, `accepts-version` stamping,
// and session→token exchange, with only `exchangeToken` mocked (the
// hop that would otherwise need a real authentication service).
import { describe, expect, it, vi } from "vitest";

vi.mock("$lib/server/auth", () => ({
  exchangeToken: vi.fn(),
}));
vi.mock("$lib/server/config", () => ({
  PATIENT_FLOW_API_URL: "http://upstream.test",
}));

import { exchangeToken } from "$lib/server/auth";
import { GET, POST } from "../../src/routes/api/proxy/[...path]/+server";

function makeEvent(opts: {
  method: string;
  sessionId?: string | null;
  body?: string;
}) {
  const headers = new Headers({
    cookie: "__Host-mxi_session=leaked-if-forwarded",
    host: "localhost:5173",
    connection: "keep-alive",
    "content-length": "3",
  });
  const request = new Request("http://localhost:5173/api/proxy/api/at-a-glance", {
    method: opts.method,
    headers,
    body: opts.body,
  });
  const upstreamResponse = new Response("upstream body", {
    status: 200,
    headers: { "content-type": "application/json", etag: "\"v1\"", "x-unlisted": "drop-me" },
  });
  const fetchSpy = vi.fn().mockResolvedValue(upstreamResponse);
  return {
    event: {
      request,
      params: { path: "api/at-a-glance" },
      url: new URL(request.url),
      locals: { sessionId: opts.sessionId ?? null },
      fetch: fetchSpy,
    } as unknown as Parameters<typeof GET>[0],
    fetchSpy,
  };
}

describe("proxy handler", () => {
  it("forwards to the configured upstream URL with the query string", async () => {
    const { event, fetchSpy } = makeEvent({ method: "GET" });
    event.url.search = "?x=1";
    await GET(event);
    expect(fetchSpy).toHaveBeenCalledWith(
      "http://upstream.test/api/at-a-glance?x=1",
      expect.anything(),
    );
  });

  it("strips hop-by-hop and origin-specific request headers", async () => {
    const { event, fetchSpy } = makeEvent({ method: "GET" });
    await GET(event);
    const forwarded = fetchSpy.mock.calls[0]?.[1]?.headers as Headers;
    expect(forwarded.has("cookie")).toBe(false);
    expect(forwarded.has("host")).toBe(false);
    expect(forwarded.has("connection")).toBe(false);
    expect(forwarded.has("content-length")).toBe(false);
  });

  it("stamps accepts-version on every forwarded request", async () => {
    const { event, fetchSpy } = makeEvent({ method: "GET" });
    await GET(event);
    const forwarded = fetchSpy.mock.calls[0]?.[1]?.headers as Headers;
    expect(forwarded.get("accepts-version")).toBe("1.0");
  });

  it("never calls exchangeToken and never sets Authorization when there is no session", async () => {
    const { event, fetchSpy } = makeEvent({ method: "GET", sessionId: null });
    await GET(event);
    expect(exchangeToken).not.toHaveBeenCalled();
    const forwarded = fetchSpy.mock.calls[0]?.[1]?.headers as Headers;
    expect(forwarded.has("authorization")).toBe(false);
  });

  it("exchanges the session for a token and forwards it as Bearer", async () => {
    vi.mocked(exchangeToken).mockResolvedValueOnce("v4.public.minted");
    const { event, fetchSpy } = makeEvent({ method: "GET", sessionId: "sid-1" });
    await GET(event);
    expect(exchangeToken).toHaveBeenCalledWith(event.fetch, "sid-1");
    const forwarded = fetchSpy.mock.calls[0]?.[1]?.headers as Headers;
    expect(forwarded.get("authorization")).toBe("Bearer v4.public.minted");
  });

  it("forwards without Authorization when the session fails to exchange", async () => {
    vi.mocked(exchangeToken).mockResolvedValueOnce(null);
    const { event, fetchSpy } = makeEvent({ method: "GET", sessionId: "sid-1" });
    await GET(event);
    const forwarded = fetchSpy.mock.calls[0]?.[1]?.headers as Headers;
    expect(forwarded.has("authorization")).toBe(false);
  });

  it("forwards the request body on a POST", async () => {
    const { event, fetchSpy } = makeEvent({ method: "POST", body: "abc" });
    await POST(event);
    const init = fetchSpy.mock.calls[0]?.[1] as RequestInit;
    expect(init.method).toBe("POST");
    expect(new TextDecoder().decode(init.body as ArrayBuffer)).toBe("abc");
  });

  it("passes through only the allow-listed response headers, and the upstream status/body", async () => {
    const { event } = makeEvent({ method: "GET" });
    const response = await GET(event);
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toBe("application/json");
    expect(response.headers.get("etag")).toBe('"v1"');
    expect(response.headers.get("x-unlisted")).toBeNull();
    expect(await response.text()).toBe("upstream body");
  });
});
