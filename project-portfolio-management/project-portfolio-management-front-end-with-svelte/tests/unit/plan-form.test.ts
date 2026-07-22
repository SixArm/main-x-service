// Component tests for PlanForm. These pin its build() behaviour:
// required-name validation, comma-list splitting, blank -> null nulling of
// scalars, optional kind label, status dropdown, identifier rows, and the
// general parent_ref (kept for any plan).
import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import PlanForm from "$lib/components/PlanForm.svelte";
import { ALL_KINDS } from "$lib/api/types";
import type { Plan } from "$lib/api/types";

/** Render the form with a spy `onsubmit`, returning it plus captured records. */
function renderForm(initial: Plan) {
  const submitted: Plan[] = [];
  const onsubmit = vi.fn(async (record: Plan) => {
    submitted.push(record);
  });
  const utils = render(PlanForm, { initial, onsubmit });
  return { ...utils, onsubmit, submitted };
}

/** Submit the form element that owns the given container. */
async function submit(container: HTMLElement) {
  const form = container.querySelector("form");
  if (!form) throw new Error("no <form> rendered");
  await fireEvent.submit(form);
}

describe("PlanForm assembly", () => {
  it("blocks submit and shows an error when the name is blank", async () => {
    const { container, onsubmit } = renderForm({ name: "   " });
    await submit(container);
    expect(onsubmit).not.toHaveBeenCalled();
    expect(container.querySelector(".banner")).toBeTruthy();
  });

  it("trims the name before building the record", async () => {
    const { container, submitted } = renderForm({ name: "  Apollo migration  " });
    await submit(container);
    expect(submitted[0]?.name).toBe("Apollo migration");
  });

  it("carries a seeded kind label into the record", async () => {
    const { container, submitted } = renderForm({ kind: "Program", name: "Delivery" });
    await submit(container);
    expect(submitted[0]?.kind).toBe("Program");
  });

  it("offers every kind label, including the newer ones", async () => {
    const { container } = renderForm({ name: "Kind options" });
    const options = [...container.querySelectorAll("select option")].map(
      (o) => o.textContent?.trim(),
    );
    for (const kind of ALL_KINDS) expect(options).toContain(kind);
    for (const kind of ["Practice", "Process", "Purpose", "Pathway", "Proposal"])
      expect(ALL_KINDS).toContain(kind as (typeof ALL_KINDS)[number]);
  });

  it("carries a newer kind label into the record", async () => {
    const { container, submitted } = renderForm({ kind: "Pathway", name: "Route" });
    await submit(container);
    expect(submitted[0]?.kind).toBe("Pathway");
  });

  it("nulls the kind when none is selected (optional label)", async () => {
    const { container, submitted } = renderForm({ name: "No kind" });
    await submit(container);
    expect(submitted[0]?.kind).toBeNull();
  });

  it("splits comma-list fields into trimmed, non-empty tokens", async () => {
    const { container, submitted } = renderForm({
      name: "Apollo",
      keywords: ["infra", " latency ", ""],
      tags: ["q1", "fast-track"],
    });
    await submit(container);
    const rec = submitted[0]!;
    expect(rec.keywords).toEqual(["infra", "latency"]);
    expect(rec.tags).toEqual(["q1", "fast-track"]);
  });

  it("nulls blank scalar fields (code, owner_org_id, owner_org_name)", async () => {
    const { container, submitted } = renderForm({ name: "Apollo" });
    await submit(container);
    const rec = submitted[0]!;
    expect(rec.code).toBeNull();
    expect(rec.owner_org_id).toBeNull();
    expect(rec.owner_org_name).toBeNull();
  });

  it("maps an unselected status dropdown to null", async () => {
    const { container, submitted } = renderForm({ name: "Apollo" });
    await submit(container);
    expect(submitted[0]?.status).toBeNull();
  });

  it("preserves a selected status value", async () => {
    const { container, submitted } = renderForm({ name: "Apollo", status: "Active" });
    await submit(container);
    expect(submitted[0]?.status).toBe("Active");
  });

  it("carries a typed code into the record", async () => {
    const { container, getByPlaceholderText, submitted } = renderForm({ name: "Apollo" });
    await fireEvent.input(getByPlaceholderText("PROJ-2026"), {
      target: { value: "  PROJ-2026  " },
    });
    await submit(container);
    expect(submitted[0]?.code).toBe("PROJ-2026");
  });

  it("emits non-empty identifier rows and drops empty ones", async () => {
    const { container, submitted } = renderForm({
      name: "Apollo",
      identifiers: [
        { scheme: "JiraProjectKey", value: "APOLLO" },
        { scheme: "LocalId", value: "" },
      ],
    });
    await submit(container);
    expect(submitted[0]?.identifiers).toEqual([
      { scheme: "JiraProjectKey", value: "APOLLO" },
    ]);
  });

  it("drops seeded Custom-scheme identifier rows", async () => {
    const { container, submitted } = renderForm({
      name: "Apollo",
      identifiers: [
        { scheme: { Custom: "InternalRef" }, value: "X-1" },
        { scheme: "JiraProjectKey", value: "APOLLO" },
      ],
    });
    await submit(container);
    expect(submitted[0]?.identifiers).toEqual([
      { scheme: "JiraProjectKey", value: "APOLLO" },
    ]);
  });

  it("keeps parent_ref for any plan (blank -> null)", async () => {
    const withParent = renderForm({
      name: "Apollo",
      parent_ref: "11111111-1111-1111-1111-111111111111",
    });
    await submit(withParent.container);
    expect(withParent.submitted[0]?.parent_ref).toBe(
      "11111111-1111-1111-1111-111111111111",
    );

    const rootPlan = renderForm({ name: "Roadmap" });
    await submit(rootPlan.container);
    expect(rootPlan.submitted[0]?.parent_ref).toBeNull();
  });
});
