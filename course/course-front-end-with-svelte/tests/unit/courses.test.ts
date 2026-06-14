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
