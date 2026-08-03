// Unit tests for the merge guard: the pure validateMerge() helper that
// routes/merge/+page.svelte calls before POSTing a merge. It returns an
// i18n key (or null), so these pin the key rather than English prose.
import { describe, expect, it } from "vitest";
import { validateMerge } from "../../src/lib/components/merge-validation";

describe("validateMerge", () => {
  // A missing main id blocks the merge.
  it("requires a main id", () => {
    expect(validateMerge("", "dup-1")).toBe("merge.bothIdsRequired");
  });

  // A missing duplicate id blocks the merge.
  it("requires a duplicate id", () => {
    expect(validateMerge("main-1", "")).toBe("merge.bothIdsRequired");
  });

  // The two ids must differ — the service answers 422 on a self-merge, so
  // catching it here saves the round trip.
  it("rejects identical main and duplicate ids", () => {
    expect(validateMerge("same", "same")).toBe("merge.mustDiffer");
  });

  // Two distinct, present ids pass the guard.
  it("accepts two distinct ids", () => {
    expect(validateMerge("main-1", "dup-1")).toBeNull();
  });
});
