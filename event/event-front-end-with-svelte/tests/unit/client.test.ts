// Unit tests for ApiClient: envelope unwrapping, error mapping, query
// building, and 204 handling. All HTTP is stubbed via an injected fetch.
import { afterEach, describe, expect, it } from "vitest";
import { ApiClient, ApiError } from "../../src/lib/api/client";

// Cast a plain async function to the `fetch` type so it can be injected
// as the client's fetch implementation (no real network involved).
function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

// Build a JSON Response with the given body/status for the stubbed fetch.
function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

describe("ApiClient", () => {
    // Pins: a 2xx success envelope is unwrapped to its `data` payload.
    it("unwraps ApiResponse.data on success", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: { id: "abc" }, error: null }),
            ),
        });
        const data = await client.get<{ id: string }>("/api/events/abc");
        expect(data).toEqual({ id: "abc" });
    });

    // Pins: a non-2xx response throws an ApiError carrying the envelope's
    // status, code, and message.
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
        await expect(client.get("/api/events/missing")).rejects.toMatchObject({
            name: "ApiError",
            status: 404,
            code: "NOT_FOUND",
            message: "missing",
        });
    });

    // Pins: a 409 maps to ApiError.isConflict === true and preserves
    // `details` (used to surface duplicate candidates on create).
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
            await client.post("/api/events", { body: {} });
            throw new Error("should have thrown");
        } catch (err) {
            expect(err).toBeInstanceOf(ApiError);
            expect((err as ApiError).isConflict).toBe(true);
            expect((err as ApiError).details).toEqual([]);
        }
    });

    // Pins: defined query params are serialized; undefined/null ones are
    // omitted from the URL.
    it("appends query string parameters and skips nullish values", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://localhost:8080/",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        await client.get("/api/events/search", {
            query: { q: "Smith", limit: 10, fuzzy: true, mask_sensitive: undefined },
        });
        expect(capturedUrl).toContain("q=Smith");
        expect(capturedUrl).toContain("limit=10");
        expect(capturedUrl).toContain("fuzzy=true");
        expect(capturedUrl).not.toContain("mask_sensitive");
    });

    // Pins: a 204 response resolves to undefined (no body parsing).
    it("returns undefined for 204 No Content", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () => new Response(null, { status: 204 })),
        });
        const result = await client.delete("/api/events/abc");
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
        await client.post("/api/events", { body: {} });
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
        await client.get("/api/events/abc");
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
        await client.post("/api/events", { body: {} });
        expect(capturedHeaders?.has("x-csrf-token")).toBe(false);
    });
});
