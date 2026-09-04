// Unit tests for EventRepository: confirms each method hits the correct
// /api/ path, forwards query params, and normalizes responses. HTTP is
// stubbed via an injected fetch (no network).
import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { EventRepository } from "../../src/lib/api/events";
import type { Event, MergeRequest } from "../../src/lib/api/types";

// Cast a plain async function to the `fetch` type for injection.
function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

// Build a JSON Response with the given body/status for the stubbed fetch.
function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

// Minimal valid event used as the canned response body across tests.
const sampleEvent: Event = {
    id: "event-1",
    name: "Annual Conference",
    start_date: "2026-06-01T09:00:00Z",
    event_type: "conference",
    event_status: "scheduled",
};

describe("EventRepository", () => {
    // Pins: create() targets the /api/events path and returns the
    // unwrapped created event.
    // Pins: exportGdpr() GETs the dedicated /export endpoint (T-20) and
    // hands back the service-defined payload untouched — the page saves
    // it as a file, it never interprets it.
    it("GETs /api/events/{id}/export on exportGdpr()", async () => {
        let capturedUrl = "";
        let capturedMethod = "";
        const payload = { subject: "x1", exported_at: "2026-09-03T00:00:00Z", records: [] };
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                capturedUrl = String(input);
                capturedMethod = init?.method ?? "GET";
                return jsonResponse({ success: true, data: payload, error: null });
            }),
        });
        const repo = new EventRepository(client);
        const result = await repo.exportGdpr("x1");
        expect(capturedUrl).toContain("/api/events/x1/export");
        expect(capturedMethod).toBe("GET");
        expect(result).toEqual(payload);
    });

    it("POSTs to /api/events on create (version-free path)", async () => {
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
        expect(capturedUrl).toContain("/api/events");
        expect(result.id).toBe("event-1");
    });

    // Pins: search() forwards the date-window filters as query params.
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

    // Pins: search() forwards the `fuzzy` toggle as a query param when set,
    // and omits it (nullish) when left off (FR-2: fuzzy toggle wired in).
    it("forwards fuzzy=true when the toggle is on and omits it when off", async () => {
        let onUrl = "";
        const onClient = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                onUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        await new EventRepository(onClient).search({ q: "*", fuzzy: true });
        expect(onUrl).toContain("fuzzy=true");

        let offUrl = "";
        const offClient = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                offUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        await new EventRepository(offClient).search({ q: "*" });
        expect(offUrl).not.toContain("fuzzy");
    });

    // Pins: merge() POSTs to /api/events/merge with the snake_case body
    // shape the service expects (FR-9 / merge workflow).
    it("POSTs the merge body shape to /api/events/merge", async () => {
        let capturedUrl = "";
        let capturedBody: unknown = null;
        let capturedMethod = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                capturedUrl = String(input);
                capturedMethod = init?.method ?? "";
                capturedBody = init?.body ? JSON.parse(String(init.body)) : null;
                return jsonResponse({
                    success: true,
                    data: { merge_record: { id: "m-1", merged_at: "2026-06-01T00:00:00Z" }, main_event: sampleEvent },
                    error: null,
                });
            }),
        });
        const request: MergeRequest = {
            main_event_id: "main-1",
            duplicate_event_id: "dup-1",
            merge_reason: "Confirmed duplicate",
        };
        const result = await new EventRepository(client).merge(request);
        expect(capturedMethod).toBe("POST");
        expect(capturedUrl).toContain("/api/events/merge");
        expect(capturedBody).toEqual({
            main_event_id: "main-1",
            duplicate_event_id: "dup-1",
            merge_reason: "Confirmed duplicate",
        });
        expect(result.main_event.id).toBe("event-1");
    });

    // Pins: merge preview issues a per-ID GET (FR-9: preview before POST).
    it("fetches a single event by id for merge preview", async () => {
        let capturedUrl = "";
        let capturedMethod = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input, init) => {
                capturedUrl = String(input);
                capturedMethod = init?.method ?? "";
                return jsonResponse({ success: true, data: sampleEvent, error: null });
            }),
        });
        await new EventRepository(client).get("main-1");
        expect(capturedMethod).toBe("GET");
        expect(capturedUrl).toContain("/api/events/main-1");
    });

    // Pins: a bare-array search payload is normalized to {items, total}
    // with total derived from the array length.
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

    // Pins: health() targets the /api/health endpoint.
    it("uses /api/health for health-check", async () => {
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
        expect(capturedUrl).toContain("/api/health");
    });
});
