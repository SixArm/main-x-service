// Unit tests for the merge guard: the pure validateMerge() helper that
// routes/merge/+page.svelte calls before POSTing a merge. It returns an
// i18n KEY (not an English sentence) so the page can translate it, which
// is what these assertions pin.
import { describe, expect, it } from "vitest";
import { validateMerge } from "../../src/lib/components/merge-validation";

describe("validateMerge", () => {
    // A missing main pid blocks the merge.
    it("requires a main pid", () => {
        expect(validateMerge("", "dup-1")).toBe("merge.bothIdsRequired");
    });

    // A missing duplicate pid blocks the merge.
    it("requires a duplicate pid", () => {
        expect(validateMerge("main-1", "")).toBe("merge.bothIdsRequired");
    });

    // The two pids must differ — the service answers a self-merge with
    // 422, and this catches it before the round trip.
    it("rejects identical main and duplicate pids", () => {
        expect(validateMerge("same", "same")).toBe("merge.mustDiffer");
    });

    // Two distinct, present pids pass the guard.
    it("accepts two distinct pids", () => {
        expect(validateMerge("main-1", "dup-1")).toBeNull();
    });
});
