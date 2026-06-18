// Unit tests for the merge guard (FR-9): the pure validateMerge() helper
// that routes/things/merge/+page.svelte calls before POSTing a merge.
import { describe, expect, it } from "vitest";
import { validateMerge } from "../../src/lib/components/merge-validation";

describe("validateMerge (FR-9)", () => {
    // FR-9: a missing main id blocks the merge.
    it("requires a main id", () => {
        expect(validateMerge("", "dup-1")).toBe("Both IDs required");
    });

    // FR-9: a missing duplicate id blocks the merge.
    it("requires a duplicate id", () => {
        expect(validateMerge("main-1", "")).toBe("Both IDs required");
    });

    // FR-9: the two ids must differ (no self-merge).
    it("rejects identical main and duplicate ids", () => {
        expect(validateMerge("same", "same")).toBe("Main and duplicate must differ");
    });

    // FR-9: two distinct, present ids pass the guard.
    it("accepts two distinct ids", () => {
        expect(validateMerge("main-1", "dup-1")).toBeNull();
    });
});
