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

// Build a JSON Response with the expected content-type header. `extraHeaders`
// lets a test (T-28) also stub the family-wide pagination headers.
function jsonResponse(
    body: unknown,
    status = 200,
    extraHeaders: Record<string, string> = {},
): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json", ...extraHeaders },
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

    // Pins: checkDuplicates() uses /api/things/check-duplicates — the path the
    // Thing Service actually serves (service spec §6 / agents/restful.md;
    // front-end spec §9). Guards against regressing to the wrong /duplicates path.
    it("uses /api/things/check-duplicates for duplicate check", async () => {
        let capturedUrl = "";
        let capturedMethod = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                capturedUrl = String(input);
                capturedMethod = init?.method ?? "";
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.checkDuplicates({ name: "Pride and Prejudice" });
        expect(capturedUrl).toContain("/api/things/check-duplicates");
        expect(capturedMethod).toBe("POST");
    });

    // Pins: a bare-array search response is normalised to {items, total}.
    it("normalises bare-array search responses to {items, total}", async () => {
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

    // Pins: an enveloped { items, total } search response is passed through,
    // including an explicit total that differs from the page item count.
    it("normalises enveloped {items,total} search responses", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: { items: [sampleThing], total: 42 }, error: null }),
            ),
        });
        const repo = new ThingRepository(client);
        const result = await repo.search({ q: "Pride" });
        expect(result.items).toHaveLength(1);
        expect(result.total).toBe(42);
    });

    // Pins: an enveloped response with items but no total falls back to count.
    it("falls back to item count when an enveloped response omits total", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: { items: [sampleThing] }, error: null }),
            ),
        });
        const repo = new ThingRepository(client);
        const result = await repo.search({ q: "Pride" });
        expect(result.total).toBe(1);
    });

    // Pins T-28: the family-wide X-Total-Count/X-Limit/X-Offset response
    // headers (agents/share/restful.md) take priority over anything the
    // body itself carries — the headers are the authoritative count
    // ignoring limit/offset, where a bare-array body has no room for one
    // at all.
    it("prefers X-Total-Count/X-Limit/X-Offset headers over the body", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse(
                    { success: true, data: [sampleThing], error: null },
                    200,
                    { "X-Total-Count": "431", "X-Limit": "25", "X-Offset": "50" },
                ),
            ),
        });
        const repo = new ThingRepository(client);
        const result = await repo.search({ q: "Pride", limit: 25, offset: 50 });
        expect(result.items).toHaveLength(1);
        expect(result.total).toBe(431);
        expect(result.limit).toBe(25);
        expect(result.offset).toBe(50);
    });

    // Pins: a service with no pagination headers still works, falling
    // back to the requested limit/offset (or item count / 0) exactly as
    // it did before the headers existed.
    it("falls back to the requested limit/offset when headers are absent", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: [sampleThing], error: null }),
            ),
        });
        const repo = new ThingRepository(client);
        const result = await repo.search({ q: "Pride", limit: 50 });
        expect(result.limit).toBe(50);
        expect(result.offset).toBe(0);
    });

    // Pins: get(id) issues GET /api/things/{id}.
    it("GETs /api/things/{id} on get", async () => {
        let capturedUrl = "";
        let capturedMethod = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                capturedUrl = String(input);
                capturedMethod = init?.method ?? "";
                return jsonResponse({ success: true, data: sampleThing, error: null });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.get("thing-1");
        expect(capturedUrl).toContain("/api/things/thing-1");
        expect(capturedMethod).toBe("GET");
    });

    // Pins: update(id) issues PUT /api/things/{id}.
    it("PUTs /api/things/{id} on update", async () => {
        let capturedUrl = "";
        let capturedMethod = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                capturedUrl = String(input);
                capturedMethod = init?.method ?? "";
                return jsonResponse({ success: true, data: sampleThing, error: null });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.update("thing-1", sampleThing);
        expect(capturedUrl).toContain("/api/things/thing-1");
        expect(capturedMethod).toBe("PUT");
    });

    // Pins: softDelete(id) issues DELETE /api/things/{id}.
    it("DELETEs /api/things/{id} on softDelete", async () => {
        let capturedUrl = "";
        let capturedMethod = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                capturedUrl = String(input);
                capturedMethod = init?.method ?? "";
                return new Response(null, { status: 204 });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.softDelete("thing-1");
        expect(capturedUrl).toContain("/api/things/thing-1");
        expect(capturedMethod).toBe("DELETE");
    });

    // Pins: match() POSTs /api/things/match.
    it("POSTs /api/things/match on match", async () => {
        let capturedUrl = "";
        let capturedMethod = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                capturedUrl = String(input);
                capturedMethod = init?.method ?? "";
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.match({ name: "Pride and Prejudice" });
        expect(capturedUrl).toContain("/api/things/match");
        expect(capturedMethod).toBe("POST");
    });

    // Pins: merge() POSTs /api/things/merge with the request body.
    it("POSTs /api/things/merge on merge", async () => {
        let capturedUrl = "";
        let capturedMethod = "";
        let capturedBody = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                capturedUrl = String(input);
                capturedMethod = init?.method ?? "";
                capturedBody = String(init?.body ?? "");
                return jsonResponse({
                    success: true,
                    data: {
                        merge_record: { id: "m-1", merged_at: "2026-06-15T00:00:00Z" },
                        main_thing: sampleThing,
                    },
                    error: null,
                });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.merge({
            main_thing_id: "thing-1",
            duplicate_thing_id: "thing-2",
            merge_reason: "dup",
        });
        expect(capturedUrl).toContain("/api/things/merge");
        expect(capturedMethod).toBe("POST");
        expect(capturedBody).toContain("thing-2");
    });

    // Pins: deduplicate() POSTs /api/things/deduplicate.
    it("POSTs /api/things/deduplicate on deduplicate", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: { pairs: [] }, error: null });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.deduplicate();
        expect(capturedUrl).toContain("/api/things/deduplicate");
    });

    // Pins: masked(id) GETs the privacy view at /api/things/{id}/masked.
    it("GETs /api/things/{id}/masked on masked", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: sampleThing, error: null });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.masked("thing-1");
        expect(capturedUrl).toContain("/api/things/thing-1/masked");
    });

    // Pins: exportGdpr(id) GETs /api/things/{id}/export.
    it("GETs /api/things/{id}/export on exportGdpr", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: {}, error: null });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.exportGdpr("thing-1");
        expect(capturedUrl).toContain("/api/things/thing-1/export");
    });

    // Pins: audit(id, limit) GETs /api/things/{id}/audit with the limit query.
    it("GETs /api/things/{id}/audit with a limit on audit", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.audit("thing-1", 10);
        expect(capturedUrl).toContain("/api/things/thing-1/audit");
        expect(capturedUrl).toContain("limit=10");
    });

    // Pins: recentAudit() GETs the system-wide feed at /api/audit/recent.
    it("GETs /api/audit/recent on recentAudit", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        const repo = new ThingRepository(client);
        await repo.recentAudit();
        expect(capturedUrl).toContain("/api/audit/recent");
    });

    // Pins: health() GETs /api/health.
    it("GETs /api/health on health", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: { status: "ok" }, error: null });
            }),
        });
        const repo = new ThingRepository(client);
        const status = await repo.health();
        expect(capturedUrl).toContain("/api/health");
        expect(status.status).toBe("ok");
    });
});
