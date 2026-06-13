import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { ThingRepository } from "../../src/lib/api/things";
import type { Thing } from "../../src/lib/api/types";

function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

const sampleThing: Thing = {
    id: "thing-1",
    name: "Pride and Prejudice",
    additional_type: "https://schema.org/Book",
    identifiers: [{ property_id: "Isbn", value: "9780141439518" }],
};

describe("ThingRepository", () => {
    it("POSTs to /api/things on create", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: sampleThing, error: null });
            }),
        });
        const repo = new ThingRepository(client);
        const result = await repo.create({ name: "Pride and Prejudice" });
        expect(capturedUrl).toContain("/api/things");
        expect(result.id).toBe("thing-1");
    });

    it("uses /api/things/duplicates for duplicate check", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.checkDuplicates({ name: "Pride and Prejudice" });
        expect(capturedUrl).toContain("/api/things/duplicates");
        expect(capturedUrl).not.toContain("check-duplicates");
    });

    it("normalises search responses to {items, total}", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: [sampleThing], error: null }),
            ),
        });
        const repo = new ThingRepository(client);
        const result = await repo.search({ q: "Pride" });
        expect(result.items).toHaveLength(1);
        expect(result.total).toBe(1);
    });
});
