// Unit tests for the low-level ApiClient: envelope unwrapping, error
// classification, query-string building, and 204 handling. Uses an injected
// fake fetch so no network is involved.
import { describe, expect, it } from "vitest";
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
