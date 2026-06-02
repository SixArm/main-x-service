import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { EventRepository } from "../../src/lib/api/events";
import type { Event } from "../../src/lib/api/types";

function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

const sampleEvent: Event = {
    id: "event-1",
    name: "Annual Conference",
    start_date: "2026-06-01T09:00:00Z",
    event_type: "conference",
    event_status: "scheduled",
};

describe("EventRepository", () => {
    it("POSTs to /api/v1/events on create (note the /v1 prefix)", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: sampleEvent, error: null });
            }),
        });
        const repo = new EventRepository(client);
        const result = await repo.create({ name: "Annual Conference", start_date: "2026-06-01T09:00:00Z" });
        expect(capturedUrl).toContain("/api/v1/events");
        expect(result.id).toBe("event-1");
    });

    it("passes date_from / date_to as search query params", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        const repo = new EventRepository(client);
        await repo.search({ q: "*", date_from: "2026-06-01", date_to: "2026-06-30" });
        expect(capturedUrl).toContain("date_from=2026-06-01");
        expect(capturedUrl).toContain("date_to=2026-06-30");
    });

    it("normalises search responses to {items, total}", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: [sampleEvent], error: null }),
            ),
        });
        const repo = new EventRepository(client);
        const result = await repo.search({ q: "Annual" });
        expect(result.items).toHaveLength(1);
        expect(result.total).toBe(1);
    });

    it("uses /api/v1/health for health-check", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: { status: "ok" }, error: null });
            }),
        });
        const repo = new EventRepository(client);
        await repo.health();
        expect(capturedUrl).toContain("/api/v1/health");
    });
});
