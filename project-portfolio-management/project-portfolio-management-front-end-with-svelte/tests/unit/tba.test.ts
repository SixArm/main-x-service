// TbaClient path mapping + the presentation helpers. The client is the
// single source of TBA endpoint paths, so a wrong path fails here
// first; the helpers carry the two judgements the UI must not get
// wrong — that a null ratio is "unknown" rather than zero, and that a
// low flow efficiency is normal rather than alarming.

import { describe, expect, it, vi } from "vitest";
import { ApiClient } from "$lib/api/client";
import {
  TbaClient,
  days,
  flowEfficiencyBand,
  interpretationLabel,
  msAsDays,
  percent,
} from "$lib/api/tba";

/** A TbaClient whose fetch records every (url, method) pair. */
function recordingClient(): {
  tba: TbaClient;
  calls: { url: string; method: string }[];
} {
  const calls: { url: string; method: string }[] = [];
  const fetchFn = vi.fn(
    async (input: RequestInfo | URL, init?: RequestInit) => {
      calls.push({ url: String(input), method: init?.method ?? "GET" });
      return new Response("{}", {
        status: 200,
        headers: { "content-type": "application/json" },
      });
    },
  ) as unknown as typeof fetch;
  const tba = new TbaClient(
    new ApiClient({ baseUrl: "http://svc", fetch: fetchFn }),
  );
  return { tba, calls };
}

describe("TbaClient paths", () => {
  it("maps every time-based-analysis endpoint", async () => {
    const { tba, calls } = recordingClient();
    await tba.planTimeAnalysis("p1");
    await tba.taskTimeAnalysis("p1", "t1");
    await tba.transitions("p1", "t1");
    await tba.constraints("p1");
    await tba.agingWip("p1");
    await tba.flow("p1");
    await tba.cumulativeFlow("p1");
    await tba.flowClasses();

    expect(calls.map((c) => c.url)).toEqual([
      "http://svc/api/plans/p1/time-analysis",
      "http://svc/api/plans/p1/tasks/t1/time-analysis",
      "http://svc/api/plans/p1/tasks/t1/transitions",
      "http://svc/api/plans/p1/constraints",
      "http://svc/api/plans/p1/aging-wip",
      "http://svc/api/plans/p1/flow",
      "http://svc/api/plans/p1/cumulative-flow",
      "http://svc/api/flow-classes",
    ]);
  });

  it("is read-only — every call is a GET", async () => {
    // The transition log is append-only and is written by the task
    // endpoints. A write verb appearing here would mean the UI had
    // grown a way to edit history.
    const { tba, calls } = recordingClient();
    await tba.planTimeAnalysis("p1");
    await tba.constraints("p1");
    await tba.flow("p1");
    await tba.cumulativeFlow("p1");
    expect(calls.every((c) => c.method === "GET")).toBe(true);
  });

  it("appends query parameters only when asked", async () => {
    const { tba, calls } = recordingClient();
    await tba.planTimeAnalysis("p1", { slePercentile: 0.9, targetDays: 14 });
    await tba.planTimeAnalysis("p1", { sprint: "s1" });
    await tba.constraints("p1", "s1");
    await tba.agingWip("p1", 0.5);
    await tba.flow("p1", 30);
    await tba.cumulativeFlow("p1", 90);

    expect(calls[0]?.url).toBe(
      "http://svc/api/plans/p1/time-analysis?sle_percentile=0.9&target_days=14",
    );
    expect(calls[1]?.url).toBe(
      "http://svc/api/plans/p1/time-analysis?sprint=s1",
    );
    expect(calls[2]?.url).toBe("http://svc/api/plans/p1/constraints?sprint=s1");
    expect(calls[3]?.url).toBe(
      "http://svc/api/plans/p1/aging-wip?sle_percentile=0.5",
    );
    expect(calls[4]?.url).toBe("http://svc/api/plans/p1/flow?window_days=30");
    expect(calls[5]?.url).toBe(
      "http://svc/api/plans/p1/cumulative-flow?days=90",
    );
  });
});

describe("presentation helpers", () => {
  it("renders a null ratio as unknown, never as zero", () => {
    // A null ratio means undefined — an item that never started has no
    // flow efficiency. Rendering that as "0%" would read as
    // catastrophically inefficient rather than as not-applicable.
    expect(percent(null)).toBe("—");
    expect(percent(undefined)).toBe("—");
    expect(percent(Number.NaN)).toBe("—");
    expect(percent(0)).toBe("0%");
    expect(percent(0.1234, 1)).toBe("12.3%");
    expect(percent(1)).toBe("100%");
  });

  it("renders durations, and a missing one as an em-dash", () => {
    expect(days(null)).toBe("—");
    expect(days(2.345)).toBe("2.3d");
    expect(msAsDays(86_400_000)).toBe("1.0d");
    expect(msAsDays(null)).toBe("—");
    expect(msAsDays(0)).toBe("0.0d");
  });

  it("bands flow efficiency against the field's own norm", () => {
    // 5–15% is typical for knowledge work, so a low number is normal
    // and a very high one usually means the board is stale rather than
    // that the queue has gone.
    expect(flowEfficiencyBand(null)).toBe("unknown");
    expect(flowEfficiencyBand(0.06)).toBe("typical");
    expect(flowEfficiencyBand(0.149)).toBe("typical");
    expect(flowEfficiencyBand(0.2)).toBe("strong");
    expect(flowEfficiencyBand(0.6)).toBe("strong");
    expect(flowEfficiencyBand(0.95)).toBe("suspicious");
  });

  it("labels every Little's-Law interpretation the service can return", () => {
    expect(interpretationLabel("wip_growing")).toBe(
      "Work in progress is growing",
    );
    expect(interpretationLabel("steady_state")).toBe("Steady state");
    expect(interpretationLabel("queue_draining")).toBe("Queue draining");
    expect(interpretationLabel("insufficient_data")).toBe("Not enough data");
  });
});
