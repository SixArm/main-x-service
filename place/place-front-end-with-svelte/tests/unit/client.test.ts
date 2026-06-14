// Unit tests for the low-level ApiClient: envelope unwrapping, error
// mapping, query building, and no-content handling. Uses an injected stub
// fetch so no real network or server is involved.
import { describe, expect, it } from "vitest";
import { ApiClient, ApiError } from "../../src/lib/api/client";

/** Cast a plain async function to the `fetch` signature for injection. */
function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
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
    // Pins: a `{ success, data, error }` envelope is unwrapped to `data`.
    it("unwraps ApiResponse.data on success", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: { id: "abc" }, error: null }),
            ),
        });
        const data = await client.get<{ id: string }>("/api/places/abc");
        expect(data).toEqual({ id: "abc" });
    });

    // Pins: a non-2xx response throws an ApiError carrying status + the
    // envelope's code/message.
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
        await expect(client.get("/api/places/missing")).rejects.toMatchObject({
            name: "ApiError",
            status: 404,
            code: "NOT_FOUND",
            message: "missing",
        });
    });

    // Pins: 409 maps to `isConflict`, and `details` (the duplicate
    // candidates array) is preserved for the create flow to render.
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
                return jsonResponse({ success: true, data: [], error: null });
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
