import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { PlaceRepository } from "../../src/lib/api/places";
import type { Place } from "../../src/lib/api/types";

function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

const samplePlace: Place = {
    id: "place-1",
    name: "Central Park",
    address: {
        address_locality: "New York",
        address_region: "NY",
        address_country: "US",
        postal_code: "10022",
    },
    geo: { latitude: 40.7829, longitude: -73.9654 },
};

describe("PlaceRepository", () => {
    it("POSTs to /api/places on create", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: samplePlace, error: null });
            }),
        });
        const repo = new PlaceRepository(client);
        const result = await repo.create({ name: "Central Park" });
        expect(capturedUrl).toContain("/api/places");
        expect(result.id).toBe("place-1");
    });

    it("uses /api/places/check-duplicates for duplicate check", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        const repo = new PlaceRepository(client);
        await repo.checkDuplicates({ name: "Central Park" });
        expect(capturedUrl).toContain("/api/places/check-duplicates");
    });

    it("normalises search responses to {items, total}", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: [samplePlace], error: null }),
            ),
        });
        const repo = new PlaceRepository(client);
        const result = await repo.search({ q: "Central Park" });
        expect(result.items).toHaveLength(1);
        expect(result.total).toBe(1);
    });
});
