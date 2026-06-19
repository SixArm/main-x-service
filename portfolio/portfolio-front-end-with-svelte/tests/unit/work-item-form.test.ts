// Component tests for WorkItemForm. These pin its build() behaviour:
// required-name validation, comma-list splitting, blank -> null nulling of
// scalars, status dropdown, identifier rows, and child-kind portfolio_ref.
import { describe, it, expect, vi } from "vitest";
import { render, fireEvent } from "@testing-library/svelte";
import WorkItemForm from "$lib/components/WorkItemForm.svelte";
import type { WorkItem, WorkItemKind } from "$lib/api/types";

/** Render the form with a spy `onsubmit`, returning it plus captured records. */
function renderForm(
  initial: WorkItem,
  kind: WorkItemKind = "Project",
) {
  const submitted: WorkItem[] = [];
  const onsubmit = vi.fn(async (record: WorkItem) => {
    submitted.push(record);
  });
  const utils = render(WorkItemForm, { kind, initial, onsubmit });
  return { ...utils, onsubmit, submitted };
}

/** Submit the form element that owns the given container. */
async function submit(container: HTMLElement) {
  const form = container.querySelector("form");
  if (!form) throw new Error("no <form> rendered");
  await fireEvent.submit(form);
}

describe("WorkItemForm assembly", () => {
  it("blocks submit and shows an error when the name is blank", async () => {
    const { container, onsubmit } = renderForm({ kind: "Project", name: "   " });
    await submit(container);
    expect(onsubmit).not.toHaveBeenCalled();
    expect(container.querySelector(".banner")).toBeTruthy();
  });

  it("trims the name before building the record", async () => {
    const { container, submitted } = renderForm({
      kind: "Project",
      name: "  Apollo migration  ",
    });
    await submit(container);
    expect(submitted[0]?.name).toBe("Apollo migration");
  });

  it("carries the collection's kind into the record", async () => {
    const { container, submitted } = renderForm(
      { kind: "Program", name: "Delivery" },
      "Program",
    );
    await submit(container);
    expect(submitted[0]?.kind).toBe("Program");
  });

  it("splits comma-list fields into trimmed, non-empty tokens", async () => {
    const { container, submitted } = renderForm({
      kind: "Project",
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
    const { container, submitted } = renderForm({ kind: "Project", name: "Apollo" });
    await submit(container);
    const rec = submitted[0]!;
    expect(rec.code).toBeNull();
    expect(rec.owner_org_id).toBeNull();
    expect(rec.owner_org_name).toBeNull();
  });

  it("maps an unselected status dropdown to null", async () => {
    const { container, submitted } = renderForm({ kind: "Project", name: "Apollo" });
    await submit(container);
    expect(submitted[0]?.status).toBeNull();
  });

  it("preserves a selected status value", async () => {
    const { container, submitted } = renderForm({
      kind: "Project",
      name: "Apollo",
      status: "Active",
    });
    await submit(container);
    expect(submitted[0]?.status).toBe("Active");
  });

  it("carries a typed code into the record", async () => {
    const { container, getByPlaceholderText, submitted } = renderForm({
      kind: "Project",
      name: "Apollo",
    });
    await fireEvent.input(getByPlaceholderText("PROJ-2026"), {
      target: { value: "  PROJ-2026  " },
    });
    await submit(container);
    expect(submitted[0]?.code).toBe("PROJ-2026");
  });

  it("emits non-empty identifier rows and drops empty ones", async () => {
    const { container, submitted } = renderForm({
      kind: "Project",
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
      kind: "Project",
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

  it("keeps portfolio_ref for child kinds and nulls it for portfolios", async () => {
    const child = renderForm(
      { kind: "Project", name: "Apollo", portfolio_ref: "11111111-1111-1111-1111-111111111111" },
      "Project",
    );
    await submit(child.container);
    expect(child.submitted[0]?.portfolio_ref).toBe(
      "11111111-1111-1111-1111-111111111111",
    );

    const umbrella = renderForm(
      { kind: "Portfolio", name: "Roadmap", portfolio_ref: "should-be-dropped" },
      "Portfolio",
    );
    await submit(umbrella.container);
    expect(umbrella.submitted[0]?.portfolio_ref).toBeNull();
  });
});
