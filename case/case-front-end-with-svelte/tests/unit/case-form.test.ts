// Component tests for CaseForm. These pin the spec §8 "Case-assembly"
// behaviour: required-title validation, comma-list splitting, blank -> null
// nulling of scalars, enum dropdown selection, and identifier rows. The form
// builds the wire `Case` imperatively in `build()` on submit, so we render it,
// drive the inputs via @testing-library, submit, and assert the record handed
// to the `onsubmit` callback.
import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import CaseForm from "$lib/components/CaseForm.svelte";
import type { Case } from "$lib/api/types";

/** Render the form with a spy `onsubmit`, returning it plus the captured records. */
function renderForm(initial: Case = { title: "" }) {
  const submitted: Case[] = [];
  const onsubmit = vi.fn(async (record: Case) => {
    submitted.push(record);
  });
  const utils = render(CaseForm, { initial, onsubmit });
  return { ...utils, onsubmit, submitted };
}

/** Submit the form element that owns the given container. */
async function submit(container: HTMLElement) {
  const form = container.querySelector("form");
  if (!form) throw new Error("no <form> rendered");
  await fireEvent.submit(form);
}

describe("CaseForm assembly", () => {
  // Pins: a blank title is rejected client-side and never reaches onsubmit.
  it("blocks submit and shows an error when the title is blank", async () => {
    const { container, getByText, onsubmit } = renderForm({ title: "   " });
    await submit(container);
    expect(onsubmit).not.toHaveBeenCalled();
    expect(getByText("Title is required.")).toBeTruthy();
  });

  // Pins: the title is trimmed into the built record.
  it("trims the title before building the record", async () => {
    const { container, submitted } = renderForm({ title: "  Housing appeal  " });
    await submit(container);
    expect(submitted[0]?.title).toBe("Housing appeal");
  });

  // Pins: comma-separated list inputs split into trimmed, non-empty tokens.
  it("splits comma-list fields into trimmed, non-empty tokens", async () => {
    const { container, submitted } = renderForm({
      title: "Case",
      subjects: ["claimant-7", " party-2 ", ""],
      keywords: ["housing", "appeal"],
    });
    await submit(container);
    const rec = submitted[0]!;
    expect(rec.subjects).toEqual(["claimant-7", "party-2"]);
    expect(rec.keywords).toEqual(["housing", "appeal"]);
  });

  // Pins: blank scalar inputs collapse to null on the wire.
  it("nulls blank scalar fields (case_number, agency, opened_date)", async () => {
    const { container, submitted } = renderForm({ title: "Case" });
    await submit(container);
    const rec = submitted[0]!;
    expect(rec.case_number).toBeNull();
    expect(rec.agency_id).toBeNull();
    expect(rec.agency_name).toBeNull();
    expect(rec.opened_date).toBeNull();
  });

  // Pins: an unselected enum dropdown ("—") maps back to null.
  it("maps unselected enum dropdowns to null", async () => {
    const { container, submitted } = renderForm({ title: "Case" });
    await submit(container);
    const rec = submitted[0]!;
    expect(rec.case_type).toBeNull();
    expect(rec.status).toBeNull();
    expect(rec.priority).toBeNull();
  });

  // Pins: a seeded bare-string enum survives the round-trip into the record.
  it("preserves a selected enum value", async () => {
    const { container, submitted } = renderForm({
      title: "Case",
      case_type: "Housing",
      status: "Open",
      priority: "Normal",
    });
    await submit(container);
    const rec = submitted[0]!;
    expect(rec.case_type).toBe("Housing");
    expect(rec.status).toBe("Open");
    expect(rec.priority).toBe("Normal");
  });

  // Pins: a non-blank scalar input is carried through trimmed.
  it("carries a typed case number into the record", async () => {
    const { container, getByPlaceholderText, submitted } = renderForm({
      title: "Case",
    });
    await fireEvent.input(getByPlaceholderText("2026-HB-0042"), {
      target: { value: "  2026-HB-0042  " },
    });
    await submit(container);
    expect(submitted[0]?.case_number).toBe("2026-HB-0042");
  });

  // Pins: seeded identifier rows are emitted; empty-value rows are dropped.
  it("emits non-empty identifier rows and drops empty ones", async () => {
    const { container, submitted } = renderForm({
      title: "Case",
      identifiers: [
        { scheme: "Docket", value: "HB-2026-0042" },
        { scheme: "LocalId", value: "" },
      ],
    });
    await submit(container);
    const rec = submitted[0]!;
    expect(rec.identifiers).toEqual([
      { scheme: "Docket", value: "HB-2026-0042" },
    ]);
  });

  // Pins: Custom-scheme identifier rows are dropped on seed (the scheme
  // <select> only offers unit schemes), per the form's documented behaviour.
  it("drops seeded Custom-scheme identifier rows", async () => {
    const { container, submitted } = renderForm({
      title: "Case",
      identifiers: [
        { scheme: { Custom: "InternalRef" }, value: "X-1" },
        { scheme: "Docket", value: "HB-1" },
      ],
    });
    await submit(container);
    const rec = submitted[0]!;
    expect(rec.identifiers).toEqual([{ scheme: "Docket", value: "HB-1" }]);
  });
});
