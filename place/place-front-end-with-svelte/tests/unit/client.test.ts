// Unit tests for the low-level ApiClient: bare JSON successes, RFC 9457
// problem errors, query building, and no-content handling. Uses an injected stub
// fetch so no real network or server is involved.
import { describe, expect, it } from "vitest";
import { ApiClient, ApiError } from "../../src/lib/api/client";

/** Cast a plain async function to the `fetch` signature for injection. */
function mockFetch(
  impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>,
) {
  return impl as unknown as typeof fetch;
}

/** Build a JSON `Response` with the given body/status for stub fetches. */
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
    const data = await client.get<{ id: string }>("/api/places/abc");
    expect(data).toEqual({ id: "abc" });
  });

  // Pins: a non-2xx response throws an ApiError carrying status + the
  // problem's code/detail.
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
    await expect(client.get("/api/places/missing")).rejects.toMatchObject({
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
    await expect(client.get("/api/places/abc")).rejects.toMatchObject({
      status: 500,
      code: "internal_server_error",
      message: "Internal Server Error",
    });
  });

  // Pins: 409 maps to `isConflict`, and the problem's `errors` (the
  // duplicate candidates array) is exposed as `details` for the create
  // flow to render.
  it("exposes ApiError.isConflict for 409 duplicate detection", async () => {
    const client = new ApiClient({
      baseUrl: "http://localhost:8080",
      fetch: mockFetch(async () =>
        jsonResponse(
          {
            type: "about:blank",
            detail: "dup",
            code: "duplicate_detected",
            errors: [],
          },
          409,
        ),
      ),
    });
    try {
      await client.post("/api/places", { body: {} });
      throw new Error("should have thrown");
    } catch (err) {
      expect(err).toBeInstanceOf(ApiError);
      expect((err as ApiError).isConflict).toBe(true);
      expect((err as ApiError).details).toEqual([]);
    }
  });

  // Pins: query params are serialized, `undefined`/`null` are omitted,
  // and a trailing slash on the base URL doesn't break path resolution.
  it("appends query string parameters and skips nullish values", async () => {
    let capturedUrl = "";
    const client = new ApiClient({
      baseUrl: "http://localhost:8080/",
      fetch: mockFetch(async (input) => {
        capturedUrl = String(input);
        return jsonResponse([]);
      }),
    });
    await client.get("/api/places/search", {
      query: { q: "Smith", limit: 10, fuzzy: true, mask_sensitive: undefined },
    });
    expect(capturedUrl).toContain("q=Smith");
    expect(capturedUrl).toContain("limit=10");
    expect(capturedUrl).toContain("fuzzy=true");
    expect(capturedUrl).not.toContain("mask_sensitive");
  });

  // Pins: 204 responses resolve to `undefined` (e.g. soft-delete) rather
  // than attempting to parse an empty body.
  it("returns undefined for 204 No Content", async () => {
    const client = new ApiClient({
      baseUrl: "http://localhost:8080",
      fetch: mockFetch(async () => new Response(null, { status: 204 })),
    });
    const result = await client.delete("/api/places/abc");
    expect(result).toBeUndefined();
  });
});
