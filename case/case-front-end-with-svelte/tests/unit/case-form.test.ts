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

  // Pins FE-5: a seeded Custom-scheme identifier row now survives (it used
  // to be dropped, since the scheme <select> only offered unit schemes).
  it("preserves a seeded Custom-scheme identifier row", async () => {
    const { container, submitted } = renderForm({
      title: "Case",
      identifiers: [
        { scheme: { Custom: "InternalRef" }, value: "X-1" },
        { scheme: "Docket", value: "HB-1" },
      ],
    });
    await submit(container);
    const rec = submitted[0]!;
    expect(rec.identifiers).toEqual([
      { scheme: { Custom: "InternalRef" }, value: "X-1" },
      { scheme: "Docket", value: "HB-1" },
    ]);
  });
});

// FE-5: `Custom(label)` editing for case type / status / identifier scheme.
// Each dropdown offers a "Custom" option that reveals a label text input,
// reassembled into the `{ Custom: "<label>" }` wire shape on submit.
describe("CaseForm Custom(label) editing (FE-5)", () => {
  // Pins: selecting "Custom" for case type reveals a label input, and the
  // round-trip payload shape is `{ Custom: "<label>" }`.
  it("selects Custom for case type and sends { Custom: label }", async () => {
    const { container, getByLabelText, submitted } = renderForm({
      title: "Case",
    });
    const caseTypeSelect = container.querySelectorAll("select")[0]!;
    await fireEvent.change(caseTypeSelect, { target: { value: "Custom" } });
    await fireEvent.input(getByLabelText("Custom label"), {
      target: { value: "  Guardianship  " },
    });
    await submit(container);
    expect(submitted[0]?.case_type).toEqual({ Custom: "Guardianship" });
  });

  // Pins: selecting "Custom" for status reveals a label input, and the
  // round-trip payload shape is `{ Custom: "<label>" }`.
  it("selects Custom for status and sends { Custom: label }", async () => {
    const { container, getByLabelText, submitted } = renderForm({
      title: "Case",
    });
    const statusSelect = container.querySelectorAll("select")[1]!;
    await fireEvent.change(statusSelect, { target: { value: "Custom" } });
    await fireEvent.input(getByLabelText("Custom label"), {
      target: { value: "  Under appeal  " },
    });
    await submit(container);
    expect(submitted[0]?.status).toEqual({ Custom: "Under appeal" });
  });

  // Pins: selecting "Custom" for an identifier row's scheme reveals a label
  // input alongside the value input, and the round-trip payload shape is
  // `{ scheme: { Custom: "<label>" }, value: "<value>" }`.
  it("selects Custom for an identifier scheme and sends { Custom: label }", async () => {
    const { container, getByLabelText, getByText, submitted } = renderForm({
      title: "Case",
    });
    await fireEvent.click(getByText("+ Add identifier"));
    const schemeSelect = container.querySelectorAll("fieldset select")[0]!;
    await fireEvent.change(schemeSelect, { target: { value: "Custom" } });
    await fireEvent.input(getByLabelText("Custom label"), {
      target: { value: "  InternalRef  " },
    });
    await fireEvent.input(container.querySelector("fieldset input[type=text]:last-of-type")!, {
      target: { value: "X-9" },
    });
    await submit(container);
    expect(submitted[0]?.identifiers).toEqual([
      { scheme: { Custom: "InternalRef" }, value: "X-9" },
    ]);
  });

  // Pins: selecting "Custom" but leaving the label blank blocks submit with
  // a clear error, rather than sending `{ Custom: "" }`.
  it("blocks submit when a selected Custom label is left blank", async () => {
    const { container, onsubmit, getByText } = renderForm({ title: "Case" });
    const caseTypeSelect = container.querySelectorAll("select")[0]!;
    await fireEvent.change(caseTypeSelect, { target: { value: "Custom" } });
    await submit(container);
    expect(onsubmit).not.toHaveBeenCalled();
    expect(getByText("A custom label is required.")).toBeTruthy();
  });
});
