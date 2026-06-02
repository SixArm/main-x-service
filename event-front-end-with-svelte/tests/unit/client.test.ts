import { describe, expect, it } from "vitest";
import { ApiClient, ApiError } from "../../src/lib/api/client";

function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

describe("ApiClient", () => {
    it("unwraps ApiResponse.data on success", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: { id: "abc" }, error: null }),
            ),
        });
        const data = await client.get<{ id: string }>("/api/v1/events/abc");
        expect(data).toEqual({ id: "abc" });
    });

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
        await expect(client.get("/api/v1/events/missing")).rejects.toMatchObject({
            name: "ApiError",
            status: 404,
            code: "NOT_FOUND",
            message: "missing",
        });
    });

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
            await client.post("/api/v1/events", { body: {} });
            throw new Error("should have thrown");
        } catch (err) {
            expect(err).toBeInstanceOf(ApiError);
            expect((err as ApiError).isConflict).toBe(true);
            expect((err as ApiError).details).toEqual([]);
        }
    });

    it("appends query string parameters and skips nullish values", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://localhost:8080/",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        await client.get("/api/v1/events/search", {
            query: { q: "Smith", limit: 10, fuzzy: true, mask_sensitive: undefined },
        });
        expect(capturedUrl).toContain("q=Smith");
        expect(capturedUrl).toContain("limit=10");
        expect(capturedUrl).toContain("fuzzy=true");
        expect(capturedUrl).not.toContain("mask_sensitive");
    });

    it("returns undefined for 204 No Content", async () => {
        const client = new ApiClient({
            baseUrl: "http://localhost:8080",
            fetch: mockFetch(async () => new Response(null, { status: 204 })),
        });
        const result = await client.delete("/api/v1/events/abc");
        expect(result).toBeUndefined();
    });
});
