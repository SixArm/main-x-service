// Unit tests for the rune-based form store (src/lib/forms/form.svelte.ts).
// agents/testing.md lists "form-store behaviour" as an intended unit-test
// scope. The file name ends in `.svelte.test.ts` so the SvelteKit Vite
// plugin compiles the `$state` runes the store depends on.
import { describe, expect, it, vi } from "vitest";
import { createForm } from "../../src/lib/forms/form.svelte";

interface Draft {
    name: string;
    count: number;
}

describe("createForm", () => {
    // Pins: the initial value is exposed and structurally cloned (mutating
    // the caller's object after construction does not leak into state).
    it("seeds value from a deep clone of initial", () => {
        const initial: Draft = { name: "a", count: 1 };
        const form = createForm<Draft>({ initial, onSubmit: () => {} });
        initial.name = "mutated";
        expect(form.value).toEqual({ name: "a", count: 1 });
    });

    // Pins: update() shallow-merges a patch; setValue() replaces wholesale.
    it("update merges a patch and setValue replaces", () => {
        const form = createForm<Draft>({ initial: { name: "a", count: 1 }, onSubmit: () => {} });
        form.update({ count: 2 });
        expect(form.value).toEqual({ name: "a", count: 2 });
        form.setValue({ name: "b", count: 9 });
        expect(form.value).toEqual({ name: "b", count: 9 });
    });

    // Pins: setError/clearError manage the per-field error map.
    it("sets and clears field errors", () => {
        const form = createForm<Draft>({ initial: { name: "", count: 0 }, onSubmit: () => {} });
        form.setError("name", "required");
        expect(form.errors).toEqual({ name: "required" });
        form.clearError("name");
        expect(form.errors).toEqual({});
    });

    // Pins: submit() short-circuits when the validator returns errors and
    // never calls onSubmit.
    it("blocks submit when validation produces errors", async () => {
        const onSubmit = vi.fn();
        const form = createForm<Draft>({
            initial: { name: "", count: 0 },
            validate: (v): Record<string, string> => (v.name ? {} : { name: "required" }),
            onSubmit,
        });
        await form.submit();
        expect(onSubmit).not.toHaveBeenCalled();
        expect(form.errors).toEqual({ name: "required" });
    });

    // Pins: a clean validation runs onSubmit and toggles submitting.
    it("runs onSubmit when validation passes", async () => {
        const seen: Draft[] = [];
        const form = createForm<Draft>({
            initial: { name: "ok", count: 3 },
            validate: () => ({}),
            onSubmit: (v) => {
                seen.push({ ...v });
            },
        });
        await form.submit();
        expect(seen).toEqual([{ name: "ok", count: 3 }]);
        expect(form.submitting).toBe(false);
        expect(form.submitError).toBeNull();
    });

    // Pins: a thrown onSubmit is captured as submitError rather than
    // rejecting, and submitting is reset.
    it("captures a thrown onSubmit as submitError", async () => {
        const form = createForm<Draft>({
            initial: { name: "ok", count: 1 },
            onSubmit: () => {
                throw new Error("boom");
            },
        });
        await form.submit();
        expect(form.submitError).toBe("boom");
        expect(form.submitting).toBe(false);
    });

    // Pins: reset() restores the initial value and clears errors/submitError.
    it("reset restores initial state", () => {
        const form = createForm<Draft>({ initial: { name: "a", count: 1 }, onSubmit: () => {} });
        form.update({ name: "z" });
        form.setError("name", "bad");
        form.reset();
        expect(form.value).toEqual({ name: "a", count: 1 });
        expect(form.errors).toEqual({});
        expect(form.submitError).toBeNull();
    });
});
