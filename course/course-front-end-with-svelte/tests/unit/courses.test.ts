// Unit tests for CourseRepository: that each method hits the right URL
// and that search() normalises the service's response variants — all
// over an injected mock fetch (no network).
import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { CourseRepository } from "../../src/lib/api/courses";
import type { Course } from "../../src/lib/api/types";

// Cast an arbitrary handler to the `fetch` type so it can be injected.
function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

// Build a JSON Response with the right content-type for the given body/status.
function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

// Representative course fixture reused across the cases below.
const sampleCourse: Course = {
    id: "course-1",
    name: "Introduction to Computer Science",
    course_code: "CS101",
    educational_level: "Undergraduate",
    identifiers: [{ property_id: "Doi", value: "10.0000/cs101" }],
};

describe("CourseRepository", () => {
    // Pins: create() targets /api/courses and returns the persisted course's id.
    // Pins: exportGdpr() GETs the dedicated /export endpoint (T-20) and
    // hands back the service-defined payload untouched — the page saves
    // it as a file, it never interprets it.
    it("GETs /api/courses/{id}/export on exportGdpr()", async () => {
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
        const repo = new CourseRepository(client);
        const result = await repo.exportGdpr("x1");
        expect(capturedUrl).toContain("/api/courses/x1/export");
        expect(capturedMethod).toBe("GET");
        expect(result).toEqual(payload);
    });

    it("POSTs to /api/courses on create", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: sampleCourse, error: null });
            }),
        });
        const repo = new CourseRepository(client);
        const result = await repo.create({ name: "Introduction to Computer Science" });
        expect(capturedUrl).toContain("/api/courses");
        expect(result.id).toBe("course-1");
    });

    // Pins: checkDuplicates() targets the dedicated /check-duplicates endpoint.
    it("uses /api/courses/check-duplicates for duplicate check", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        const repo = new CourseRepository(client);
        await repo.checkDuplicates({ name: "Introduction to Computer Science" });
        expect(capturedUrl).toContain("/api/courses/check-duplicates");
    });

    // Pins: a bare-array `data` payload is wrapped to {items, total} with total = length.
    it("normalises bare-array search responses to {items, total}", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse({ success: true, data: [sampleCourse], error: null }),
            ),
        });
        const repo = new CourseRepository(client);
        const result = await repo.search({ q: "CS" });
        expect(result.items).toHaveLength(1);
        expect(result.total).toBe(1);
    });

    // Pins FR-1/FR-2: search() forwards q / fuzzy / limit / offset as
    // query params, and phonetic (still accepted by the client type for
    // service parity) is forwarded when set.
    it("forwards search query params (fuzzy/limit/offset/phonetic)", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        const repo = new CourseRepository(client);
        await repo.search({ q: "CS", fuzzy: true, limit: 25, offset: 50, phonetic: true });
        const url = new URL(capturedUrl);
        expect(url.pathname).toBe("/api/courses/search");
        expect(url.searchParams.get("q")).toBe("CS");
        expect(url.searchParams.get("fuzzy")).toBe("true");
        expect(url.searchParams.get("limit")).toBe("25");
        expect(url.searchParams.get("offset")).toBe("50");
        expect(url.searchParams.get("phonetic")).toBe("true");
    });

    // Pins: unset optional search params are omitted from the query string.
    it("omits unset search params", async () => {
        let capturedUrl = "";
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async (input) => {
                capturedUrl = String(input);
                return jsonResponse({ success: true, data: [], error: null });
            }),
        });
        const repo = new CourseRepository(client);
        await repo.search({ q: "" });
        const url = new URL(capturedUrl);
        expect(url.searchParams.has("fuzzy")).toBe(false);
        expect(url.searchParams.has("limit")).toBe(false);
        expect(url.searchParams.has("offset")).toBe(false);
        expect(url.searchParams.has("phonetic")).toBe(false);
    });

    // Pins: the service's {courses, total} shape maps to {items, total}, preserving the server total.
    it("normalises {courses, total} search responses (service shape)", async () => {
        const client = new ApiClient({
            baseUrl: "http://test",
            fetch: mockFetch(async () =>
                jsonResponse({
                    success: true,
                    data: { courses: [sampleCourse], total: 42 },
                    error: null,
                }),
            ),
        });
        const repo = new CourseRepository(client);
        const result = await repo.search({ q: "CS" });
        expect(result.items).toHaveLength(1);
        expect(result.total).toBe(42);
    });
});
