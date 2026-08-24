// TbaRepository path mapping + the presentation helpers. The repository
// is the single source of TBA endpoint paths, so a wrong path fails
// here first; the helpers carry the two judgements the UI must not get
// wrong — that a null ratio is "unknown" rather than zero, and that a
// single-digit value-adding ratio is the norm rather than a fault.

import { describe, expect, it, vi } from "vitest";
import { ApiClient } from "$lib/api/client";
import {
  TbaRepository,
  confidenceNote,
  days,
  interpretationLabel,
  msAsDays,
  percent,
  valueAddingBand,
} from "$lib/api/tba";

/** A TbaRepository whose fetch records every (url, method) pair. */
function recording(): {
  tba: TbaRepository;
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
  const tba = new TbaRepository(
    new ApiClient({ baseUrl: "http://svc", fetch: fetchFn }),
  );
  return { tba, calls };
}

describe("TbaRepository paths", () => {
  it("maps every time-based-analysis endpoint", async () => {
    const { tba, calls } = recording();
    await tba.timeline("i1");
    await tba.instanceAnalysis("i1");
    await tba.segments("i1");
    await tba.cohort("p1");
    await tba.constraints("p1");
    await tba.standards();
    await tba.flow();

    expect(calls.map((c) => c.url)).toEqual([
      "http://svc/api/instances/i1/timeline",
      "http://svc/api/instances/i1/time-analysis",
      "http://svc/api/instances/i1/segments",
      "http://svc/api/care-pathways/p1/time-analysis",
      "http://svc/api/care-pathways/p1/constraints",
      "http://svc/api/instances/time-standards",
      "http://svc/api/instances/flow",
    ]);
    expect(calls.every((c) => c.method === "GET")).toBe(true);
  });

  it("appends query parameters only when asked", async () => {
    const { tba, calls } = recording();
    await tba.cohort("p1", { standard: "rtt_18_weeks" });
    await tba.cohort("p1", { targetDays: 30, status: "closed" });
    await tba.constraints("p1", "open");
    await tba.flow(30, "p1");

    expect(calls[0]?.url).toBe(
      "http://svc/api/care-pathways/p1/time-analysis?standard=rtt_18_weeks",
    );
    expect(calls[1]?.url).toBe(
      "http://svc/api/care-pathways/p1/time-analysis?target_days=30&status=closed",
    );
    expect(calls[2]?.url).toBe(
      "http://svc/api/care-pathways/p1/constraints?status=open",
    );
    expect(calls[3]?.url).toBe(
      "http://svc/api/instances/flow?window_days=30&pathway=p1",
    );
  });

  it("posts a recorded segment and a clock event", async () => {
    const { tba, calls } = recording();
    await tba.recordSegment("i1", {
      label: "MRI",
      stage: "diagnostics",
      category: "value_adding",
      started_at: "2026-01-01T00:00:00Z",
    });
    await tba.setClock("i1", "stop", "2026-02-01T00:00:00Z");
    await tba.setClock("i1", "start");

    expect(calls.map((c) => [c.method, c.url])).toEqual([
      ["POST", "http://svc/api/instances/i1/segments"],
      ["POST", "http://svc/api/instances/i1/clock"],
      ["POST", "http://svc/api/instances/i1/clock"],
    ]);
  });

  it("encodes a pid rather than splicing it into the path", async () => {
    const { tba, calls } = recording();
    await tba.timeline("a/b?c");
    expect(calls[0]?.url).toBe("http://svc/api/instances/a%2Fb%3Fc/timeline");
  });
});

describe("presentation helpers", () => {
  it("renders a null ratio as unknown, never as zero", () => {
    // A null ratio means undefined. An unmeasurable clock has no
    // value-adding ratio, and "0%" would read as a catastrophically
    // wasteful journey rather than as "we cannot tell".
    expect(percent(null)).toBe("—");
    expect(percent(undefined)).toBe("—");
    expect(percent(Number.NaN)).toBe("—");
    expect(percent(0)).toBe("0%");
    expect(percent(0.14, 1)).toBe("14.0%");
  });

  it("renders durations, and a missing one as an em-dash", () => {
    expect(days(null)).toBe("—");
    expect(days(2.345)).toBe("2.3d");
    expect(msAsDays(86_400_000)).toBe("1.0d");
    expect(msAsDays(null)).toBe("—");
  });

  it("bands a value-adding ratio against the tracked NHS norm", () => {
    // 8–14% is what tracked journeys measure, so single digits are the
    // norm the method predicts — not a fault to explain away.
    expect(valueAddingBand(0.11, "mapped")).toBe("typical");
    expect(valueAddingBand(0.05, "mapped")).toBe("typical");
    expect(valueAddingBand(0.3, "mapped")).toBe("better");
    expect(valueAddingBand(0.9, "mapped")).toBe("suspicious");
    expect(valueAddingBand(null, "mapped")).toBe("unknown");
  });

  it("refuses to band a journey nobody mapped", () => {
    // An unmapped journey reports a ratio near zero that looks exactly
    // like a catastrophically wasteful one. It must never be read as a
    // measurement, whatever the number happens to be.
    expect(valueAddingBand(0.02, "unmapped")).toBe("unknown");
    expect(valueAddingBand(0.9, "unmapped")).toBe("unknown");
    expect(confidenceNote("unmapped")).toContain("not a measurement");
    expect(confidenceNote("partial")).toContain("floor");
    expect(confidenceNote("mapped")).toContain("evidenced");
  });

  it("labels every Little's-Law interpretation the service can return", () => {
    expect(interpretationLabel("backlog_growing")).toBe(
      "The backlog is growing",
    );
    expect(interpretationLabel("steady_state")).toBe("Steady state");
    expect(interpretationLabel("queue_draining")).toBe("Queue draining");
    expect(interpretationLabel("insufficient_data")).toBe("Not enough data");
  });
});
