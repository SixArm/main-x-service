// Unit tests for the createForm rune-based controller
// (src/lib/forms/form.svelte.ts). Exercises the pure controller logic
// spec §11 cites as a unit-test target: validate-blocks-submit,
// submit-error capture, reset, and per-field error set/clear. Runs
// without a DOM mount — `createForm` is a plain function of its args.
//
// The runes ($state) used inside are compiled by the Svelte Vite
// plugin (the file is `*.svelte.ts`), so importing it under Vitest
// gives real reactive state.
import { describe, expect, it, vi } from "vitest";
import { createForm, type FieldErrors } from "../../src/lib/forms/form.svelte";

interface Model {
    name: string;
    count: number;
}

describe("createForm", () => {
    // Pins: the initial object is deep-cloned, so mutating the caller's
    // object after construction does not leak into the form value.
    it("deep-clones the initial value", () => {
        const initial: Model = { name: "a", count: 1 };
        const form = createForm<Model>({ initial, onSubmit: () => {} });
        initial.name = "mutated";
        expect(form.value.name).toBe("a");
    });

    // Pins FR-4 path: when validate returns errors, onSubmit never runs
    // and the errors are exposed on the controller.
    it("blocks submit and surfaces field errors when validation fails", async () => {
        const onSubmit = vi.fn();
        const form = createForm<Model>({
            initial: { name: "", count: 0 },
            validate: (v): FieldErrors => (v.name.trim() ? {} : { name: "Required" }),
            onSubmit,
        });
        await form.submit();
        expect(onSubmit).not.toHaveBeenCalled();
        expect(form.errors.name).toBe("Required");
    });

    // Pins: a passing validation runs onSubmit with the current value.
    it("runs onSubmit with the value when validation passes", async () => {
        const onSubmit = vi.fn();
        const form = createForm<Model>({
            initial: { name: "ok", count: 2 },
            validate: () => ({}),
            onSubmit,
        });
        await form.submit();
        expect(onSubmit).toHaveBeenCalledWith({ name: "ok", count: 2 });
        expect(form.errors).toEqual({});
    });

    // Pins: a rejection inside onSubmit is captured as submitError (never
    // escapes), and the submitting flag is cleared afterwards.
    it("captures an onSubmit failure as submitError and clears submitting", async () => {
        const form = createForm<Model>({
            initial: { name: "ok", count: 0 },
            onSubmit: () => {
                throw new Error("boom");
            },
        });
        await form.submit();
        expect(form.submitError).toBe("boom");
        expect(form.submitting).toBe(false);
    });

    // Pins: per-field error set/clear helpers reassign (not mutate) the
    // error map so rune tracking sees the change.
    it("sets and clears per-field errors", () => {
        const form = createForm<Model>({ initial: { name: "x", count: 0 }, onSubmit: () => {} });
        form.setError("count", "Must be ≥ 0");
        expect(form.errors.count).toBe("Must be ≥ 0");
        form.clearError("count");
        expect(form.errors.count).toBeUndefined();
    });

    // Pins: update() shallow-merges a patch; setValue replaces wholesale.
    it("update() patches and setValue() replaces the value", () => {
        const form = createForm<Model>({ initial: { name: "a", count: 1 }, onSubmit: () => {} });
        form.update({ count: 9 });
        expect(form.value).toEqual({ name: "a", count: 9 });
        form.setValue({ name: "b", count: 2 });
        expect(form.value).toEqual({ name: "b", count: 2 });
    });

    // Pins: reset() restores the pristine snapshot and clears errors +
    // submitError, and survives a prior value mutation.
    it("reset() restores the initial value and clears errors", async () => {
        const form = createForm<Model>({
            initial: { name: "init", count: 0 },
            validate: (v): FieldErrors => (v.name ? {} : { name: "Required" }),
            onSubmit: () => {
                throw new Error("boom");
            },
        });
        form.update({ name: "edited" });
        await form.submit(); // sets submitError
        form.reset();
        expect(form.value).toEqual({ name: "init", count: 0 });
        expect(form.errors).toEqual({});
        expect(form.submitError).toBeNull();
    });
});
