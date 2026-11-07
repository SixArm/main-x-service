// Unit tests for the low-level ApiClient: bare JSON successes, RFC 9457
// problem errors, query-string handling, and the 204 path — all via an
// injected mock fetch (no network).
import { afterEach, describe, expect, it } from "vitest";
import { ApiClient, ApiError } from "../../src/lib/api/client";

// Cast an arbitrary handler to the `fetch` type so it can be injected.
function mockFetch(
  impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
) {
  return impl as unknown as typeof fetch;
}

// Build a JSON Response with the right content-type for the given body/status.
function jsonResponse(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("ApiClient", () => {
  // Pins: a success is the bare JSON body.
  it("returns the bare body on success", async () => {
    const client = new ApiClient({
      baseUrl: "http://localhost:8080",
      fetch: mockFetch(async () => jsonResponse({ id: "abc" })),
    });
    const data = await client.get<{ id: string }>("/api/courses/abc");
    expect(data).toEqual({ id: "abc" });
  });

  // Pins: a non-2xx maps to an ApiError carrying the problem's status/code/detail.
  it("throws ApiError with the parsed problem on non-2xx", async () => {
    const client = new ApiClient({
      baseUrl: "http://localhost:8080",
      fetch: mockFetch(async () =>
        jsonResponse(
          { type: "about:blank", detail: "missing", code: "not_found" },
          404,
        ),
      ),
    });
    await expect(client.get("/api/courses/missing")).rejects.toMatchObject({
      name: "ApiError",
      status: 404,
      code: "not_found",
      message: "missing",
    });
  });

  // Pins: a problem with no detail (a 500) is reported by its title.
  it("falls back to the problem's title when it has no detail", async () => {
    const client = new ApiClient({
      baseUrl: "http://localhost:8080",
      fetch: mockFetch(async () =>
        jsonResponse(
          {
            type: "about:blank",
            title: "Internal Server Error",
            status: 500,
            code: "internal_server_error",
          },
          500,
        ),
      ),
    });
    await expect(client.get("/api/courses/abc")).rejects.toMatchObject({
      status: 500,
      code: "internal_server_error",
      message: "Internal Server Error",
    });
  });

  // Pins: a 409 sets isConflict and exposes the problem's errors (the duplicate candidates) as details.
  it("exposes ApiError.isConflict for 409 duplicate detection", async () => {
    const client = new ApiClient({
      baseUrl: "http://localhost:8080",
      fetch: mockFetch(async () =>
        jsonResponse(
          { type: "about:blank", detail: "dup", code: "duplicate", errors: [] },
          409,
        ),
      ),
    });
    try {
      await client.post("/api/courses", { body: {} });
      throw new Error("should have thrown");
    } catch (err) {
      expect(err).toBeInstanceOf(ApiError);
      expect((err as ApiError).isConflict).toBe(true);
      expect((err as ApiError).details).toEqual([]);
    }
  });

  // Pins: defined query params are serialised; undefined/null ones are omitted entirely.
  it("appends query string parameters and skips nullish values", async () => {
    let capturedUrl = "";
    const client = new ApiClient({
      baseUrl: "http://localhost:8080/",
      fetch: mockFetch(async (input) => {
        capturedUrl = String(input);
        return jsonResponse([]);
      }),
    });
    await client.get("/api/courses/search", {
      query: { q: "Smith", limit: 10, fuzzy: true, mask_sensitive: undefined },
    });
    expect(capturedUrl).toContain("q=Smith");
    expect(capturedUrl).toContain("limit=10");
    expect(capturedUrl).toContain("fuzzy=true");
    expect(capturedUrl).not.toContain("mask_sensitive");
  });

  // Pins: a 204 (e.g. DELETE) resolves to undefined without attempting to parse a body.
  it("returns undefined for 204 No Content", async () => {
    const client = new ApiClient({
      baseUrl: "http://localhost:8080",
      fetch: mockFetch(async () => new Response(null, { status: 204 })),
    });
    const result = await client.delete("/api/courses/abc");
    expect(result).toBeUndefined();
  });
});

// CSRF double-submit: mutating requests echo the browser-readable
// `__Host-mxi_csrf` cookie in an `X-CSRF-Token` header (jsdom gives every
// test a real `document.cookie`, so this exercises the actual browser
// code path rather than a stub). The `__Host-` prefix requires a secure
// (https) origin to actually stick, which is why `vite.config.ts` points
// jsdom's testURL at `https://` — see the comment there.
describe("ApiClient CSRF header", () => {
  afterEach(() => {
    // Clear the cookie between tests (jsdom's `document.cookie` persists
    // across tests in the same file otherwise).
    document.cookie =
      "__Host-mxi_csrf=; expires=Thu, 01 Jan 1970 00:00:00 GMT; path=/; Secure";
  });

  it("attaches X-CSRF-Token on POST when the cookie is present", async () => {
    document.cookie = "__Host-mxi_csrf=tok-abc; path=/; Secure";
    let capturedHeaders: Headers | undefined;
    const client = new ApiClient({
      baseUrl: "http://localhost:5173/api/proxy",
      fetch: mockFetch(async (_input, init) => {
        capturedHeaders = new Headers(init?.headers);
        return jsonResponse({});
      }),
    });
    await client.post("/api/courses", { body: {} });
    expect(capturedHeaders?.get("x-csrf-token")).toBe("tok-abc");
  });

  it("does not attach X-CSRF-Token on GET even when the cookie is present", async () => {
    document.cookie = "__Host-mxi_csrf=tok-abc; path=/; Secure";
    let capturedHeaders: Headers | undefined;
    const client = new ApiClient({
      baseUrl: "http://localhost:5173/api/proxy",
      fetch: mockFetch(async (_input, init) => {
        capturedHeaders = new Headers(init?.headers);
        return jsonResponse({});
      }),
    });
    await client.get("/api/courses/abc");
    expect(capturedHeaders?.has("x-csrf-token")).toBe(false);
  });

  it("omits X-CSRF-Token on POST when no cookie is set", async () => {
    let capturedHeaders: Headers | undefined;
    const client = new ApiClient({
      baseUrl: "http://localhost:5173/api/proxy",
      fetch: mockFetch(async (_input, init) => {
        capturedHeaders = new Headers(init?.headers);
        return jsonResponse({});
      }),
    });
    await client.post("/api/courses", { body: {} });
    expect(capturedHeaders?.has("x-csrf-token")).toBe(false);
  });
});
