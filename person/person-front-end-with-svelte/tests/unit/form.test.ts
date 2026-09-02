// Unit test for createForm's `initial` handling — specifically the
// regression PRO-P4's live-integration run found: passing a Svelte 5
// `$state` reactive proxy as `initial` (exactly what `/persons/[id]/edit`
// does — it loads the record into a `$state` variable, then hands it to
// PersonForm) used to throw inside `structuredClone`, because a `$state`
// proxy is not structured-cloneable even though the data it wraps is
// plain JSON. Reproduced live via Playwright against a real service
// before being pinned here — see AGENTS.md's PRO-P4 note.
import { describe, expect, it } from "vitest";
import { createForm } from "../../src/lib/forms/form.svelte";
import { reactiveState } from "./support/reactive-state.svelte";

interface Sample {
  name: string;
  tags: string[];
}

describe("createForm", () => {
  it("accepts a plain object as `initial` (the create-page shape)", () => {
    const plain: Sample = { name: "Alice", tags: ["a", "b"] };
    expect(() =>
      createForm({ initial: plain, onSubmit: async () => {} }),
    ).not.toThrow();
  });

  it("accepts a $state-wrapped object as `initial` without throwing", () => {
    // Exactly what an edit page produces: `person = await repo.get(id)`
    // into a `let person = $state<Person | null>(null)` variable, then
    // `<PersonForm initial={person} .../>`.
    const reactive = reactiveState<Sample>({ name: "Bob", tags: ["x"] });
    expect(() =>
      createForm({ initial: reactive, onSubmit: async () => {} }),
    ).not.toThrow();
  });

  it("the form's value is an independent, plain-JSON copy of a $state initial", () => {
    const reactive = reactiveState<Sample>({ name: "Carol", tags: ["y", "z"] });
    const form = createForm({ initial: reactive, onSubmit: async () => {} });
    expect(form.value).toEqual({ name: "Carol", tags: ["y", "z"] });
    // Mutating the caller's reactive source afterward must not leak
    // into the form's own state — the whole point of cloning it.
    reactive.name = "Changed";
    expect(form.value.name).toBe("Carol");
  });

  it("reset() restores the original $state-derived value", () => {
    const reactive = reactiveState<Sample>({ name: "Dana", tags: [] });
    const form = createForm({ initial: reactive, onSubmit: async () => {} });
    form.update({ name: "Edited" });
    expect(form.value.name).toBe("Edited");
    form.reset();
    expect(form.value.name).toBe("Dana");
  });
});
