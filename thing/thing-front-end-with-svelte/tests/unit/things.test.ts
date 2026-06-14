// Unit tests for ThingRepository: verifies each method targets the right
// endpoint/verb and that search responses are normalised, using a mock fetch.
import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { ThingRepository } from "../../src/lib/api/things";
import type { Thing } from "../../src/lib/api/types";

// Coerce a plain impl into the `fetch` type for injection (see client test).
function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

// Build a JSON Response with the expected content-type header.
function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

// Representative Thing fixture reused across the cases below.
const sampleThing: Thing = {
    id: "thing-1",
    name: "Pride and Prejudice",
    additional_type: "https://schema.org/Book",
    identifiers: [{ property_id: "Isbn", value: "9780141439518" }],
};

describe("ThingRepository", () => {
    // Pins: create() hits /api/things and returns the persisted record.
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

    // Pins: checkDuplicates() uses /api/things/duplicates — guards against
    // regressing to the older "check-duplicates" path.
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

    // Pins: a bare-array search response is normalised to {items, total}.
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
