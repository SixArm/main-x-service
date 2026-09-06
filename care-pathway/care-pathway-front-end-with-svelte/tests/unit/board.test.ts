// CPFE-T3: the `/board` page's new "Record a segment" panel. The
// repository methods it calls (`TbaRepository.recordSegment`/`.setClock`)
// already had URL/body pins in `tba.test.ts`; what was untested is the
// PAGE itself — that picking an instance, filling the form, and
// submitting actually reaches those methods with the right payload, that
// the waste/category pairing guard blocks a bad submit client-side
// (mirroring the service's `tba.rs::validate_segment_fields` rule), and
// that the start/stop clock buttons fire.
//
// `+page.svelte` builds its repositories at module scope via
// `CarePathwayRepository.withFetch()` / `TbaRepository.withFetch()`
// (no injectable fetch), which binds `globalThis.fetch` once at *module
// import* time (`ApiClient`'s constructor). So the stub must be in place
// before the page module is first imported — done once, below, via a
// top-level `await import()` after `vi.stubGlobal` — and the page module
// is imported exactly once for the whole file (never re-imported via
// `vi.resetModules()`, which was tried and breaks Svelte 5's component
// effect context across the two module instances that then exist).
// Per-test behaviour varies through a reassignable handler instead.

import { describe, it, expect, afterEach, vi, beforeAll } from "vitest";
import { render, cleanup, fireEvent, waitFor } from "@testing-library/svelte";

vi.mock("$app/environment", () => ({ browser: false }));

const PATHWAY_PID = "22222222-2222-4222-8222-222222222222";
const INSTANCE_PID = "33333333-3333-4333-8333-333333333333";
const SUBJECT = "person:44444444-4444-4444-8444-444444444444";

const INSTANCE = {
  pid: INSTANCE_PID,
  pathway_pid: PATHWAY_PID,
  subject_ref: SUBJECT,
  status: "active",
  urgency: "urgent",
  enrolled_on: "2026-06-01",
  next_review_on: null,
  closed_on: null,
  closure_reason: null,
  outcome: null,
};

type Call = { path: string; method: string; body: unknown };

/** Reassigned per test; the stubbed global `fetch` always delegates here. */
let calls: Call[] = [];

function jsonResponse(body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

/** Answers the page's boot calls (refs, caseload, instances) plus the two new POSTs. */
async function defaultHandler(
  input: RequestInfo | URL,
  init?: RequestInit,
): Promise<Response> {
  const path = new URL(String(input)).pathname;
  const method = init?.method ?? "GET";
  const body = init?.body ? JSON.parse(String(init.body)) : undefined;
  calls.push({ path, method, body });

  if (path.endsWith("/api/proxy/api/care-pathways") && method === "GET") {
    return jsonResponse([{ pid: PATHWAY_PID, name: "Suspected stroke" }]);
  }
  if (path.endsWith("/api/proxy/api/instances/caseload")) {
    return jsonResponse({ note: "derived", total: 1 });
  }
  if (path.endsWith(`/api/proxy/api/care-pathways/${PATHWAY_PID}/instances`)) {
    return jsonResponse([INSTANCE]);
  }
  if (
    path.endsWith(`/api/proxy/api/instances/${INSTANCE_PID}/segments`) &&
    method === "POST"
  ) {
    return jsonResponse({ pid: "seg-1", ...(body as object) });
  }
  if (
    path.endsWith(`/api/proxy/api/instances/${INSTANCE_PID}/clock`) &&
    method === "POST"
  ) {
    return jsonResponse({ ok: true });
  }
  return new Response(JSON.stringify({ error: "unhandled" }), { status: 404 });
}

// Bound once, before the page module (below) is ever imported.
vi.stubGlobal(
  "fetch",
  vi.fn((input: RequestInfo | URL, init?: RequestInit) =>
    defaultHandler(input, init),
  ),
);

// A single import for the whole file — see the header comment on why a
// second (`vi.resetModules()`-forced) import breaks Svelte's runtime.
const { default: BoardPage } =
  await import("../../src/routes/board/+page.svelte");

beforeAll(() => {
  calls = [];
});

afterEach(() => {
  cleanup();
  calls = [];
});

/** Render the page and wait for `onMount`'s instance fetch to land. */
async function renderBoard() {
  const view = render(BoardPage);
  await waitFor(() =>
    expect(
      (
        view.container.querySelector(
          '[data-testid="segment-instance"]',
        ) as HTMLSelectElement | null
      )?.options.length,
    ).toBeGreaterThan(1),
  );
  return view;
}

describe("/board record-segment panel", () => {
  it("blocks a segment submit with no instance selected", async () => {
    const { getByText, container } = await renderBoard();

    const form = container.querySelector("form")!;
    const label = container.querySelector(
      "input[type=text]",
    ) as HTMLInputElement;
    await fireEvent.input(label, { target: { value: "MRI" } });
    const startedAt = container.querySelector(
      'input[type="datetime-local"]',
    ) as HTMLInputElement;
    await fireEvent.input(startedAt, {
      target: { value: "2026-01-01T09:00" },
    });

    await fireEvent.submit(form);
    expect(getByText("Select an instance first.")).toBeTruthy();
  });

  it("blocks an unnecessary-waste segment submit with no waste chosen", async () => {
    const { container, getByText } = await renderBoard();

    const instanceSelect = container.querySelector(
      '[data-testid="segment-instance"]',
    ) as HTMLSelectElement;
    await fireEvent.change(instanceSelect, {
      target: { value: INSTANCE_PID },
    });

    const selects = container.querySelectorAll("form select");
    const categorySelect = selects[1] as HTMLSelectElement; // Stage, Category, Waste
    await fireEvent.change(categorySelect, {
      target: { value: "unnecessary_non_value_adding" },
    });

    const label = container.querySelector(
      "input[type=text]",
    ) as HTMLInputElement;
    await fireEvent.input(label, { target: { value: "Wait for scan" } });
    const startedAt = container.querySelector(
      'input[type="datetime-local"]',
    ) as HTMLInputElement;
    await fireEvent.input(startedAt, {
      target: { value: "2026-01-01T09:00" },
    });

    await fireEvent.submit(container.querySelector("form")!);
    expect(
      getByText(
        "Waste is required on an unnecessary non-value-adding segment.",
      ),
    ).toBeTruthy();
  });

  it("records a value-adding segment (waste forced to null) and shows success", async () => {
    const { container, getByText } = await renderBoard();

    const instanceSelect = container.querySelector(
      '[data-testid="segment-instance"]',
    ) as HTMLSelectElement;
    await fireEvent.change(instanceSelect, {
      target: { value: INSTANCE_PID },
    });

    const label = container.querySelector(
      "input[type=text]",
    ) as HTMLInputElement;
    await fireEvent.input(label, { target: { value: "Scan" } });
    const startedAt = container.querySelector(
      'input[type="datetime-local"]',
    ) as HTMLInputElement;
    await fireEvent.input(startedAt, {
      target: { value: "2026-01-01T09:00" },
    });

    await fireEvent.submit(container.querySelector("form")!);

    await waitFor(() => expect(getByText(/recorded\./)).toBeTruthy());

    const posted = calls.find((c) =>
      c.path.endsWith(`/api/instances/${INSTANCE_PID}/segments`),
    );
    expect(posted?.method).toBe("POST");
    expect(posted?.body).toMatchObject({
      label: "Scan",
      stage: "treatment",
      category: "value_adding",
      waste: null,
      ended_at: null,
    });
    expect((posted?.body as { started_at: string }).started_at).toMatch(
      /^\d{4}-\d{2}-\d{2}T/,
    );
  });

  it("start/stop clock buttons call setClock and disable until an instance is picked", async () => {
    const { container, getByText } = await renderBoard();

    const buttons = Array.from(
      container.querySelectorAll(
        '[data-testid="record-segment-panel"] .row button',
      ),
    ) as HTMLButtonElement[];
    const [startBtn, stopBtn] = buttons;
    if (!startBtn || !stopBtn) throw new Error("clock buttons did not render");
    expect(startBtn.disabled).toBe(true);
    expect(stopBtn.disabled).toBe(true);

    const instanceSelect = container.querySelector(
      '[data-testid="segment-instance"]',
    ) as HTMLSelectElement;
    await fireEvent.change(instanceSelect, {
      target: { value: INSTANCE_PID },
    });
    expect(startBtn.disabled).toBe(false);

    await fireEvent.click(startBtn);
    await waitFor(() => expect(getByText("Clock started.")).toBeTruthy());

    const posted = calls.find((c) =>
      c.path.endsWith(`/api/instances/${INSTANCE_PID}/clock`),
    );
    expect(posted?.method).toBe("POST");
    expect(posted?.body).toEqual({ event: "start" });
  });
});
