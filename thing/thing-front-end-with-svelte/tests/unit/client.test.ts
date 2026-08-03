// Unit tests for ApiClient: verifies envelope unwrapping, error mapping,
// query-string building, and 204 handling using an injected mock fetch.
import { describe, expect, it } from "vitest";
import { ApiClient, ApiError } from "../../src/lib/api/client";

// Coerce a plain impl into the `fetch` type so it can be injected as the
// client's fetch without pulling in DOM lib typings.
function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

// Helper to build a JSON Response with the expected content-type header.
function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

describe("ApiClient", () => {
    // Pins: a 200 with { success, data } returns just the inner data.
    it("unwraps ApiResponse.data on success", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: { id: "abc" }, error: null }),
            ),
        });
        const data = await client.get<{ id: string }>("/api/things/abc");
        expect(data).toEqual({ id: "abc" });
    });

    // Pins: a non-2xx maps the envelope error into a typed ApiError
    // (status + code + message).
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
        await expect(client.get("/api/things/missing")).rejects.toMatchObject({
            name: "ApiError",
            status: 404,
            code: "NOT_FOUND",
            message: "missing",
        });
    });

    // Pins: 409 sets isConflict and preserves details (the duplicate list)
    // — the contract the create flow relies on.
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
            await client.post("/api/things", { body: {} });
            throw new Error("should have thrown");
        } catch (err) {
            expect(err).toBeInstanceOf(ApiError);
            expect((err as ApiError).isConflict).toBe(true);
            expect((err as ApiError).details).toEqual([]);
        }
    });

    // Pins: defined query params are serialised; undefined/null are omitted.
    it("appends query string parameters and skips nullish values", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://localhost:8080/",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        await client.get("/api/things/search", {
            query: { q: "Smith", limit: 10, fuzzy: true, mask_sensitive: undefined },
        });
        expect(capturedUrl).toContain("q=Smith");
        expect(capturedUrl).toContain("limit=10");
        expect(capturedUrl).toContain("fuzzy=true");
        expect(capturedUrl).not.toContain("mask_sensitive");
    });

    // Pins: a 204 (e.g. DELETE) resolves to undefined with no body parsing.
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
        await client.get("/api/things/abc");
        expect(capturedUrl).toBe("http://localhost:5173/api/proxy/api/things/abc");
    });

    it("returns undefined for 204 No Content", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () => new Response(null, { status: 204 })),
        });
        const result = await client.delete("/api/things/abc");
        expect(result).toBeUndefined();
    });
});
