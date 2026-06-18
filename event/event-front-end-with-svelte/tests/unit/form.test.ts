// Unit tests for the createForm rune-based controller
// (src/lib/forms/form.svelte.ts) and the Event-specific time-window
// validation rules it drives in EventForm.svelte (spec §6 FR-4).
//
// Spec §11 lists "form-store behaviour" as an in-scope unit-test target.
// `createForm` is a plain function of its args; the runes ($state) it uses
// are compiled by the Svelte Vite plugin (the source is `*.svelte.ts`), so
// importing it under Vitest yields real reactive state — no DOM mount.
import { describe, expect, it, vi } from "vitest";
import { createForm, type FieldErrors } from "../../src/lib/forms/form.svelte";
import type { Event } from "../../src/lib/api/types";

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

// The Event time-window validator that EventForm.svelte passes to
// createForm (spec §6 FR-4: name + start required, end >= start,
// door <= start). Kept in lock-step with EventForm.svelte's inline
// `validate(value)` so this suite pins the behaviour without a DOM mount.
function validateEvent(value: Event): FieldErrors {
    const errors: FieldErrors = {};
    if (!value.name.trim()) errors.name = "Required";
    if (!value.start_date) errors.start_date = "Required";
    if (value.start_date && value.end_date && Date.parse(value.end_date) < Date.parse(value.start_date)) {
        errors.end_date = "End must be ≥ start";
    }
    if (value.door_time && value.start_date && Date.parse(value.door_time) > Date.parse(value.start_date)) {
        errors.door_time = "Door time must be ≤ start";
    }
    return errors;
}

describe("Event time-window validation (FR-4)", () => {
    const base: Event = { name: "Annual Conference", start_date: "2026-06-01T09:00:00Z" };

    it("requires name and start_date", () => {
        expect(validateEvent({ name: "  ", start_date: "" }).name).toBe("Required");
        expect(validateEvent({ name: "  ", start_date: "" }).start_date).toBe("Required");
    });

    it("accepts a minimal valid event (name + start only)", () => {
        expect(validateEvent(base)).toEqual({});
    });

    it("rejects end_date before start_date and accepts end >= start", () => {
        expect(validateEvent({ ...base, end_date: "2026-06-01T08:00:00Z" }).end_date).toBe("End must be ≥ start");
        expect(validateEvent({ ...base, end_date: "2026-06-01T09:00:00Z" }).end_date).toBeUndefined();
        expect(validateEvent({ ...base, end_date: "2026-06-01T17:00:00Z" }).end_date).toBeUndefined();
    });

    it("rejects door_time after start_date and accepts door <= start", () => {
        expect(validateEvent({ ...base, door_time: "2026-06-01T09:30:00Z" }).door_time).toBe("Door time must be ≤ start");
        expect(validateEvent({ ...base, door_time: "2026-06-01T08:30:00Z" }).door_time).toBeUndefined();
        expect(validateEvent({ ...base, door_time: "2026-06-01T09:00:00Z" }).door_time).toBeUndefined();
    });

    it("drives createForm: invalid time window blocks submit", async () => {
        const onSubmit = vi.fn();
        const form = createForm<Event>({
            initial: { ...base, end_date: "2026-06-01T08:00:00Z" },
            validate: validateEvent,
            onSubmit,
        });
        await form.submit();
        expect(onSubmit).not.toHaveBeenCalled();
        expect(form.errors.end_date).toBe("End must be ≥ start");
    });
});
