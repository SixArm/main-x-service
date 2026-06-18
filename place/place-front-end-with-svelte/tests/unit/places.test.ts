// Unit tests for PlaceRepository: verify each method hits the expected
// endpoint path and that response normalization works. Uses a stub fetch
// that captures the request URL.
import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { PlaceRepository } from "../../src/lib/api/places";
import type { MatchResult, MergeResponse, Place } from "../../src/lib/api/types";

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

/** A representative place used as the stubbed response payload. */
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
    // Pins: create() targets the collection path `/api/places` and returns
    // the unwrapped created record.
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

    // Pins: the duplicate-check endpoint is the hyphenated
    // `/api/places/check-duplicates` (distinct from `/match`).
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

    // Pins: a bare `Place[]` search payload is normalized to
    // `{ items, total }`, with total derived from the array length.
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

    // Pins: an enveloped `{ items, total }` search payload trusts the
    // service-supplied total rather than the page length.
    it("trusts the service total on enveloped search responses", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse(
                    { success: true, data: { items: [samplePlace], total: 42 }, error: null },
                ),
            ),
        });
        const repo = new PlaceRepository(client);
        const result = await repo.search({ q: "Central Park" });
        expect(result.items).toHaveLength(1);
        expect(result.total).toBe(42);
    });

    // Pins: each remaining repository method targets its documented path
    // and HTTP verb. A single captured request per call guards against the
    // /match vs /check-duplicates and per-id-path copy artifacts.
    type Captured = { url: string; method: string };

    /** Build a repo whose fetch captures the URL + method, returning `body`. */
    function captureRepo(body: unknown): { repo: PlaceRepository; last: Captured } {
        const last: Captured = { url: "", method: "" };
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                last.url = String(input);
                last.method = init?.method ?? "GET";
                return jsonResponse({ success: true, data: body, error: null });
            }),
        });
        return { repo: new PlaceRepository(client), last };
    }

    it("get() GETs /api/places/:id", async () => {
        const { repo, last } = captureRepo(samplePlace);
        await repo.get("place-1");
        expect(last.method).toBe("GET");
        expect(last.url).toContain("/api/places/place-1");
    });

    it("update() PUTs /api/places/:id", async () => {
        const { repo, last } = captureRepo(samplePlace);
        await repo.update("place-1", samplePlace);
        expect(last.method).toBe("PUT");
        expect(last.url).toContain("/api/places/place-1");
    });

    it("softDelete() DELETEs /api/places/:id", async () => {
        const { repo, last } = captureRepo(null);
        await repo.softDelete("place-1");
        expect(last.method).toBe("DELETE");
        expect(last.url).toContain("/api/places/place-1");
    });

    it("match() POSTs /api/places/match (distinct from check-duplicates)", async () => {
        const { repo, last } = captureRepo([] as MatchResult[]);
        await repo.match({ name: "Central Park" });
        expect(last.method).toBe("POST");
        expect(last.url).toContain("/api/places/match");
        expect(last.url).not.toContain("check-duplicates");
    });

    it("merge() POSTs /api/places/merge", async () => {
        const { repo, last } = captureRepo({} as MergeResponse);
        await repo.merge({ main_place_id: "a", duplicate_place_id: "b" });
        expect(last.method).toBe("POST");
        expect(last.url).toContain("/api/places/merge");
    });

    it("deduplicate() POSTs /api/places/deduplicate", async () => {
        const { repo, last } = captureRepo({
            places_scanned: 0,
            duplicates_found: 0,
            auto_merged: 0,
            queued_for_review: 0,
            review_items: [],
        });
        await repo.deduplicate();
        expect(last.method).toBe("POST");
        expect(last.url).toContain("/api/places/deduplicate");
    });

    it("masked() GETs /api/places/:id/masked", async () => {
        const { repo, last } = captureRepo(samplePlace);
        await repo.masked("place-1");
        expect(last.method).toBe("GET");
        expect(last.url).toContain("/api/places/place-1/masked");
    });

    it("exportGdpr() GETs /api/places/:id/export", async () => {
        const { repo, last } = captureRepo({});
        await repo.exportGdpr("place-1");
        expect(last.method).toBe("GET");
        expect(last.url).toContain("/api/places/place-1/export");
    });

    it("audit() GETs /api/places/:id/audit with a limit", async () => {
        const { repo, last } = captureRepo([]);
        await repo.audit("place-1", 25);
        expect(last.method).toBe("GET");
        expect(last.url).toContain("/api/places/place-1/audit");
        expect(last.url).toContain("limit=25");
    });

    it("recentAudit() GETs /api/audit/recent", async () => {
        const { repo, last } = captureRepo([]);
        await repo.recentAudit();
        expect(last.method).toBe("GET");
        expect(last.url).toContain("/api/audit/recent");
    });

    it("health() GETs /api/health", async () => {
        const { repo, last } = captureRepo({ status: "ok" });
        await repo.health();
        expect(last.method).toBe("GET");
        expect(last.url).toContain("/api/health");
    });
});
