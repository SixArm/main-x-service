// Unit tests for the low-level ApiClient: envelope unwrapping, error
// classification, query-string building, and 204 handling. Uses an injected
// fake fetch so no network is involved.
import { afterEach, describe, expect, it } from "vitest";
import { ApiClient, ApiError } from "../../src/lib/api/client";

// Cast a plain async function to the `fetch` type so it can be injected.
function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

// Build a JSON Response with the given body/status for the fake fetch.
function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

describe("ApiClient", () => {
    // Pins: a success envelope is unwrapped to its bare `data`.
    it("unwraps ApiResponse.data on success", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: { id: "abc" }, error: null }),
            ),
        });
        const data = await client.get<{ id: string }>("/api/persons/abc");
        expect(data).toEqual({ id: "abc" });
    });

    // Pins: a non-2xx status throws ApiError carrying the parsed code/message.
    it("throws ApiError with parsed envelope on non-2xx", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () =>
                jsonResponse(
                    { success: false, data: null, error: { code: "NOT_FOUND", message: "missing" } },
                    404,
                ),
            ),
        });
        await expect(client.get("/api/persons/missing")).rejects.toMatchObject({
            name: "ApiError",
            status: 404,
            code: "NOT_FOUND",
            message: "missing",
        });
    });

    // Pins: 409 sets isConflict and preserves the `details` payload that the
    // create flow reads to show duplicate candidates.
    it("exposes ApiError.isConflict for 409 duplicate detection", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () =>
                jsonResponse(
                    { success: false, data: null, error: { code: "DUPLICATE", message: "dup", details: [] } },
                    409,
                ),
            ),
        });
        try {
            await client.post("/api/persons", { body: {} });
            throw new Error("should have thrown");
        } catch (err) {
            expect(err).toBeInstanceOf(ApiError);
            expect((err as ApiError).isConflict).toBe(true);
            expect((err as ApiError).details).toEqual([]);
        }
    });

    // Pins: query params are appended, and undefined/null values are omitted
    // entirely (so optional params don't leak empty keys into the URL).
    it("appends query string parameters and skips nullish values", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://localhost:8080/",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        await client.get("/api/persons/search", {
            query: { q: "Smith", limit: 10, fuzzy: true, mask_sensitive: undefined },
        });
        expect(capturedUrl).toContain("q=Smith");
        expect(capturedUrl).toContain("limit=10");
        expect(capturedUrl).toContain("fuzzy=true");
        expect(capturedUrl).not.toContain("mask_sensitive");
    });

    // Regression (2026-08-03): a base URL that itself has a path segment —
    // the BFF proxy, `<origin>/api/proxy` — must keep that segment. An
    // earlier version resolved an absolute-path `path` (one starting with
    // `/`) as a host-relative reference, which per the URL spec replaces
    // the base URL's entire path rather than appending to it, silently
    // discarding `/api/proxy` from every request in every BFF-proxied
    // front-end in the family.
    it("keeps the base URL's own path segment (a BFF proxy prefix)", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://localhost:5173/api/proxy",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: {}, error: null });
            }),
        });
        await client.get("/api/persons/abc");
        expect(capturedUrl).toBe("http://localhost:5173/api/proxy/api/persons/abc");
    });

    // Pins: a 204 (e.g. DELETE) resolves to undefined without parsing a body.
    it("returns undefined for 204 No Content", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () => new Response(null, { status: 204 })),
        });
        const result = await client.delete("/api/persons/abc");
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
                return jsonResponse({ success: true, data: {}, error: null });
            }),
        });
        await client.post("/api/persons", { body: {} });
        expect(capturedHeaders?.get("x-csrf-token")).toBe("tok-abc");
    });

    it("does not attach X-CSRF-Token on GET even when the cookie is present", async () => {
        document.cookie = "__Host-mxi_csrf=tok-abc; path=/; Secure";
        let capturedHeaders: Headers | undefined;
        const client = new ApiClient({
            baseUrl: "http://localhost:5173/api/proxy",
            fetch: mockFetch(async (_input, init) => {
                capturedHeaders = new Headers(init?.headers);
                return jsonResponse({ success: true, data: {}, error: null });
            }),
        });
        await client.get("/api/persons/abc");
        expect(capturedHeaders?.has("x-csrf-token")).toBe(false);
    });

    it("omits X-CSRF-Token on POST when no cookie is set", async () => {
        let capturedHeaders: Headers | undefined;
        const client = new ApiClient({
            baseUrl: "http://localhost:5173/api/proxy",
            fetch: mockFetch(async (_input, init) => {
                capturedHeaders = new Headers(init?.headers);
                return jsonResponse({ success: true, data: {}, error: null });
            }),
        });
        await client.post("/api/persons", { body: {} });
        expect(capturedHeaders?.has("x-csrf-token")).toBe(false);
    });
});
