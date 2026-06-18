// Unit tests for the Thing form validator (FR-4): the pure validateThing()
// helper that ThingForm.svelte wires into createForm. Pins that `name` is
// required and that url/additional_type/main_entity_of_page/subject_of must
// be absolute http(s) URLs when present.
import { describe, expect, it } from "vitest";
import { validateThing } from "../../src/lib/components/thing-validation";
import type { Thing } from "../../src/lib/api/types";

describe("validateThing (FR-4)", () => {
    // FR-4: name is the only hard-required field.
    it("flags an empty name as Required", () => {
        const errors = validateThing({ name: "" } satisfies Thing);
        expect(errors.name).toBe("Required");
    });

    // FR-4: a whitespace-only name is still treated as missing.
    it("flags a whitespace-only name as Required", () => {
        const errors = validateThing({ name: "   " } satisfies Thing);
        expect(errors.name).toBe("Required");
    });

    // FR-4: a non-http(s) scheme on a URL-shaped field is rejected.
    it("rejects a non-http(s) additional_type", () => {
        const errors = validateThing({
            name: "Pride and Prejudice",
            additional_type: "ftp://example.com",
        } satisfies Thing);
        expect(errors.additional_type).toMatch(/Additional type must start with http\(s\):\/\//);
    });

    // FR-4: a bare host (no scheme) in `url` is rejected.
    it("rejects a url without a scheme", () => {
        const errors = validateThing({ name: "X", url: "example.com" } satisfies Thing);
        expect(errors.url).toMatch(/URL must start with http\(s\):\/\//);
    });

    // FR-4: each of the four URL-shaped fields is validated.
    it("validates main_entity_of_page and subject_of too", () => {
        const errors = validateThing({
            name: "X",
            main_entity_of_page: "not-a-url",
            subject_of: "also-not-a-url",
        } satisfies Thing);
        expect(errors.main_entity_of_page).toMatch(/Main entity of page must start/);
        expect(errors.subject_of).toMatch(/Subject of must start/);
    });

    // FR-4: empty/omitted URL fields are not flagged (only required when present).
    it("does not flag absent URL fields", () => {
        const errors = validateThing({ name: "X", url: "" } satisfies Thing);
        expect(Object.keys(errors)).toHaveLength(0);
    });

    // FR-4: a fully valid Thing with http(s) URLs produces no errors.
    it("accepts a valid Thing with http(s) URLs", () => {
        const errors = validateThing({
            name: "Pride and Prejudice",
            additional_type: "https://schema.org/Book",
            url: "http://example.com/book",
        } satisfies Thing);
        expect(Object.keys(errors)).toHaveLength(0);
    });
});
