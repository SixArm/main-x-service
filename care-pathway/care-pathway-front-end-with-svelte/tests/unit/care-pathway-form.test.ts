import { describe, it, expect, vi, afterEach } from "vitest";
import { render, cleanup, fireEvent } from "@testing-library/svelte";
import CarePathwayForm from "$lib/components/CarePathwayForm.svelte";
import type { CarePathway } from "$lib/api/types";

afterEach(cleanup);

/**
 * Render the form with an `onsubmit` spy and return the spy plus the
 * rendered container. Exercises the component end-to-end (the form's
 * client-side required-name guard and `build()` normalization) rather
 * than reaching into internals.
 */
function renderForm(initial: CarePathway) {
  const onsubmit = vi.fn(async (_p: CarePathway) => {});
  const { container } = render(CarePathwayForm, { initial, onsubmit });
  const form = container.querySelector("form");
  if (!form) throw new Error("form did not render");
  return { onsubmit, container, form };
}

// The form's role is the create/edit boundary: it must (a) block a blank
// name before any save, and (b) hand the parent a clean CarePathway —
// comma lists split, blank scalars nulled, empty rows dropped — per
// spec §8. These are pure-UI behaviours with no test before this file.
describe("CarePathwayForm", () => {
  it("blocks submit with a required-name banner when the name is blank", async () => {
    const { onsubmit, container, form } = renderForm({ name: "" });
    await fireEvent.submit(form);
    // onsubmit must NOT fire; the inline banner must appear.
    expect(onsubmit).not.toHaveBeenCalled();
    const banner = container.querySelector('[role="alert"]');
    expect(banner?.textContent).toContain("Name is required.");
  });

  it("treats a whitespace-only name as blank (trims before the guard)", async () => {
    const { onsubmit, form } = renderForm({ name: "   " });
    await fireEvent.submit(form);
    expect(onsubmit).not.toHaveBeenCalled();
  });

  it("normalizes the payload on submit: split lists, blank→null, dropped empty rows", async () => {
    // Seed every list/scalar/row field so build()'s normalization is exercised.
    const { onsubmit, form } = renderForm({
      name: "  Acute Stroke  ",
      pathway_code: "   ", // blank scalar → null
      provider_id: "prov-1",
      provider_name: "   ", // blank scalar → null
      care_setting: "Inpatient",
      alternate_names: ["AIS", " ASC "],
      interventions: ["thrombolysis", "  "],
      keywords: ["stroke", "neuro"],
      same_as: ["https://example.org/p1"],
      in_language: ["en", " cy "],
      condition_codes: [
        { system: "Icd10", code: "I63" },
        { system: "Snomed", code: "  " }, // empty code → dropped
      ],
      identifiers: [
        { scheme: "GuidelineId", value: "NG128" },
        { scheme: "Doi", value: "  " }, // empty value → dropped
      ],
    });

    await fireEvent.submit(form);

    expect(onsubmit).toHaveBeenCalledTimes(1);
    const payload = onsubmit.mock.calls[0]?.[0] as CarePathway;

    // Name trimmed.
    expect(payload.name).toBe("Acute Stroke");
    // Blank scalars → null; non-blank trimmed-through.
    expect(payload.pathway_code).toBeNull();
    expect(payload.provider_id).toBe("prov-1");
    expect(payload.provider_name).toBeNull();
    expect(payload.care_setting).toBe("Inpatient");
    // Comma lists: trimmed, empties filtered.
    expect(payload.alternate_names).toEqual(["AIS", "ASC"]);
    expect(payload.interventions).toEqual(["thrombolysis"]);
    expect(payload.keywords).toEqual(["stroke", "neuro"]);
    expect(payload.same_as).toEqual(["https://example.org/p1"]);
    // in_language round-trips through the new comma-list field.
    expect(payload.in_language).toEqual(["en", "cy"]);
    // Rows with an empty code/value are dropped; survivors trimmed.
    expect(payload.condition_codes).toEqual([{ system: "Icd10", code: "I63" }]);
    expect(payload.identifiers).toEqual([{ scheme: "GuidelineId", value: "NG128" }]);
  });

  it("collapses a Custom care_setting seed to the empty (—) option", async () => {
    // The care-setting <select> offers only unit variants, so a seeded
    // Custom setting cannot round-trip and must surface as null.
    const { onsubmit, form } = renderForm({
      name: "P",
      care_setting: { Custom: "Telehealth" },
    });
    await fireEvent.submit(form);
    const payload = onsubmit.mock.calls[0]?.[0] as CarePathway;
    expect(payload.care_setting).toBeNull();
  });

  it("drops a seeded Custom-scheme identifier (cannot round-trip via the select)", async () => {
    const { onsubmit, form } = renderForm({
      name: "P",
      identifiers: [
        { scheme: { Custom: "MyScheme" }, value: "x" },
        { scheme: "Doi", value: "10.1/abc" },
      ],
    });
    await fireEvent.submit(form);
    const payload = onsubmit.mock.calls[0]?.[0] as CarePathway;
    expect(payload.identifiers).toEqual([{ scheme: "Doi", value: "10.1/abc" }]);
  });
});
