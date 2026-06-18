// Unit tests for the CourseForm validation + wire-normalisation rules
// (spec §6 FR-4): name required; http(s) URL fields; course_code ≤ 100;
// number_of_credits ≥ 0; and the normalizeForWire blank→undefined sweep
// that keeps empty optional strings off the wire (CHANGELOG v0.2.0).
import { describe, expect, it } from "vitest";
import { validateCourse, normalizeForWire } from "../../src/lib/components/courseFormValidate";
import type { Course } from "../../src/lib/api/types";

describe("validateCourse (FR-4)", () => {
    it("requires a non-blank name", () => {
        expect(validateCourse({ name: "" }).name).toBe("Required");
        expect(validateCourse({ name: "   " }).name).toBe("Required");
        expect(validateCourse({ name: "CS101" }).name).toBeUndefined();
    });

    it("rejects non-http(s) URL fields and accepts http(s)", () => {
        const bad = validateCourse({
            name: "X",
            url: "ftp://example.com",
            additional_type: "notaurl",
            license: "www.example.com",
        });
        expect(bad.url).toMatch(/http\(s\)/);
        expect(bad.additional_type).toMatch(/http\(s\)/);
        expect(bad.license).toMatch(/http\(s\)/);

        const ok = validateCourse({
            name: "X",
            url: "https://example.com",
            additional_type: "http://schema.org/Course",
            license: "https://creativecommons.org/licenses/by/4.0/",
        });
        expect(ok.url).toBeUndefined();
        expect(ok.additional_type).toBeUndefined();
        expect(ok.license).toBeUndefined();
    });

    it("treats blank URL fields as valid (no error)", () => {
        expect(validateCourse({ name: "X", url: "" }).url).toBeUndefined();
    });

    it("caps course_code at 100 chars", () => {
        expect(validateCourse({ name: "X", course_code: "a".repeat(100) }).course_code).toBeUndefined();
        expect(validateCourse({ name: "X", course_code: "a".repeat(101) }).course_code).toBe("Max 100 chars");
    });

    it("rejects negative credits", () => {
        expect(validateCourse({ name: "X", number_of_credits: -1 }).number_of_credits).toBe("Must be ≥ 0");
        expect(validateCourse({ name: "X", number_of_credits: 0 }).number_of_credits).toBeUndefined();
        expect(validateCourse({ name: "X", number_of_credits: 3 }).number_of_credits).toBeUndefined();
    });

    it("returns an empty map for a fully valid course", () => {
        const value: Course = {
            name: "Intro to CS",
            url: "https://uni.edu/cs101",
            course_code: "CS101",
            number_of_credits: 4,
        };
        expect(validateCourse(value)).toEqual({});
    });
});

describe("normalizeForWire", () => {
    it("maps blank optional strings to undefined", () => {
        const out = normalizeForWire({
            name: "X",
            description: "",
            url: "   ",
            license: "",
            course_code: "",
        });
        expect(out.description).toBeUndefined();
        expect(out.url).toBeUndefined();
        expect(out.license).toBeUndefined();
        expect(out.course_code).toBeUndefined();
        expect(out.name).toBe("X");
    });

    it("preserves non-blank values", () => {
        const out = normalizeForWire({ name: "X", url: "https://e.com", course_code: "CS1" });
        expect(out.url).toBe("https://e.com");
        expect(out.course_code).toBe("CS1");
    });

    it("normalises nested identifier url/name blanks", () => {
        const out = normalizeForWire({
            name: "X",
            identifiers: [{ property_id: "Doi", value: "10.1/x", url: "", name: "  " }],
        });
        expect(out.identifiers?.[0]?.url).toBeUndefined();
        expect(out.identifiers?.[0]?.name).toBeUndefined();
        expect(out.identifiers?.[0]?.value).toBe("10.1/x");
    });
});
