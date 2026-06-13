import { describe, expect, it } from "vitest";
import { ApiClient } from "../../src/lib/api/client";
import { CourseRepository } from "../../src/lib/api/courses";
import type { Course } from "../../src/lib/api/types";

function mockFetch(impl: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>) {
    return impl as unknown as typeof fetch;
}

function jsonResponse(body: unknown, status = 200): Response {
    return new Response(JSON.stringify(body), {
        status,
        headers: { "content-type": "application/json" },
    });
}

const sampleCourse: Course = {
    id: "course-1",
    name: "Introduction to Computer Science",
    course_code: "CS101",
    educational_level: "Undergraduate",
    identifiers: [{ property_id: "Doi", value: "10.0000/cs101" }],
};

describe("CourseRepository", () => {
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
