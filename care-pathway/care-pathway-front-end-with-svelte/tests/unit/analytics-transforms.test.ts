// Pure transforms over the T-14 analytics payloads (spec `13-tasks.md`
// T-14l): the process-map layered layout, the variants sunburst/Sankey
// (both with the synthetic Stopped terminal), the attrition tree
// layout, and dotted-chart points. No DOM, no fetch — see
// `$lib/analytics-transforms` for the functions under test.

import { describe, expect, it } from "vitest";
import {
  STOPPED,
  dottedChartPoints,
  layoutAttrition,
  layoutProcessMap,
  sankeyFromVariants,
  sunburstArcs,
  sunburstFromVariants,
  withheldLabel,
  type AttritionStep,
  type EventLogRow,
} from "$lib/analytics-transforms";
import type { VariantSummary } from "$lib/api/tba";

describe("layoutProcessMap", () => {
  it("ranks nodes by BFS distance from start and scales node size by instance_count", () => {
    const layout = layoutProcessMap({
      nodes: [
        {
          activity: "start",
          instance_count: 10,
          occurrence_count: 10,
          median_duration_days: null,
        },
        {
          activity: "referral",
          instance_count: 10,
          occurrence_count: 10,
          median_duration_days: 1,
        },
        {
          activity: "triage",
          instance_count: 4,
          occurrence_count: 4,
          median_duration_days: 2,
        },
        {
          activity: "end",
          instance_count: 10,
          occurrence_count: 10,
          median_duration_days: null,
        },
      ],
      edges: [
        {
          from: "start",
          to: "referral",
          instance_count: 10,
          occurrence_count: 10,
          median_gap_days: 0,
          p90_gap_days: 0,
        },
        {
          from: "referral",
          to: "triage",
          instance_count: 4,
          occurrence_count: 4,
          median_gap_days: 3,
          p90_gap_days: 5,
        },
        {
          from: "referral",
          to: "end",
          instance_count: 6,
          occurrence_count: 6,
          median_gap_days: 1,
          p90_gap_days: 2,
        },
        {
          from: "triage",
          to: "end",
          instance_count: 4,
          occurrence_count: 4,
          median_gap_days: 1,
          p90_gap_days: 1,
        },
      ],
    });

    const byActivity = new Map(layout.nodes.map((n) => [n.activity, n]));
    expect(byActivity.get("start")?.rank).toBe(0);
    expect(byActivity.get("referral")?.rank).toBe(1);
    expect(byActivity.get("triage")?.rank).toBe(2);
    // "end" is reachable at rank 2 (via referral) and rank 3 (via
    // triage) -- BFS keeps the first (shortest) one found.
    expect(byActivity.get("end")?.rank).toBe(2);

    // instance_count 10 (the max) gets the largest radius; 4 gets a
    // smaller one, scaled by sqrt so *area* is proportional.
    const big = byActivity.get("referral");
    const small = byActivity.get("triage");
    expect(big).toBeDefined();
    expect(small).toBeDefined();
    expect(big!.radius).toBeGreaterThan(small!.radius);
  });

  it("flags a self-loop and a back-edge without letting either re-lower a rank", () => {
    const layout = layoutProcessMap({
      nodes: [
        {
          activity: "start",
          instance_count: 5,
          occurrence_count: 5,
          median_duration_days: null,
        },
        {
          activity: "triage",
          instance_count: 5,
          occurrence_count: 6,
          median_duration_days: 1,
        },
        {
          activity: "treatment",
          instance_count: 5,
          occurrence_count: 5,
          median_duration_days: 4,
        },
      ],
      edges: [
        {
          from: "start",
          to: "triage",
          instance_count: 5,
          occurrence_count: 5,
          median_gap_days: 0,
          p90_gap_days: 0,
        },
        // A self-loop: triage repeats.
        {
          from: "triage",
          to: "triage",
          instance_count: 1,
          occurrence_count: 1,
          median_gap_days: 2,
          p90_gap_days: 2,
        },
        {
          from: "triage",
          to: "treatment",
          instance_count: 5,
          occurrence_count: 5,
          median_gap_days: 1,
          p90_gap_days: 1,
        },
        // A back-edge: treatment bounces back to triage.
        {
          from: "treatment",
          to: "triage",
          instance_count: 1,
          occurrence_count: 1,
          median_gap_days: 3,
          p90_gap_days: 3,
        },
      ],
    });

    const selfLoop = layout.edges.find(
      (e) => e.from === "triage" && e.to === "triage",
    );
    expect(selfLoop?.isSelfLoop).toBe(true);
    expect(selfLoop?.isBackEdge).toBe(false);

    const backEdge = layout.edges.find(
      (e) => e.from === "treatment" && e.to === "triage",
    );
    expect(backEdge?.isBackEdge).toBe(true);

    // The self-loop and the back-edge must not have dragged triage's
    // rank down to treatment's or start's.
    const byActivity = new Map(layout.nodes.map((n) => [n.activity, n]));
    expect(byActivity.get("triage")?.rank).toBe(1);
    expect(byActivity.get("treatment")?.rank).toBe(2);
  });

  it("places a node unreachable from start past the highest reached rank, never drops it", () => {
    const layout = layoutProcessMap({
      nodes: [
        {
          activity: "start",
          instance_count: 3,
          occurrence_count: 3,
          median_duration_days: null,
        },
        {
          activity: "referral",
          instance_count: 3,
          occurrence_count: 3,
          median_duration_days: 1,
        },
        {
          activity: "orphan",
          instance_count: 1,
          occurrence_count: 1,
          median_duration_days: null,
        },
      ],
      edges: [
        {
          from: "start",
          to: "referral",
          instance_count: 3,
          occurrence_count: 3,
          median_gap_days: 0,
          p90_gap_days: 0,
        },
      ],
    });
    expect(layout.nodes).toHaveLength(3);
    const orphan = layout.nodes.find((n) => n.activity === "orphan");
    expect(orphan?.rank).toBeGreaterThan(
      layout.nodes.find((n) => n.activity === "referral")?.rank ?? -1,
    );
  });

  it("carries a node's/edge's suppressed flag and note through to the layout", () => {
    const layout = layoutProcessMap({
      nodes: [
        {
          activity: "start",
          instance_count: 6,
          occurrence_count: 6,
          median_duration_days: null,
        },
        {
          activity: "rare",
          instance_count: 2,
          occurrence_count: 2,
          median_duration_days: null,
          suppressed: true,
          suppression_note: "withheld: fewer than the minimum cell count",
        },
      ],
      edges: [
        {
          from: "start",
          to: "rare",
          instance_count: 2,
          occurrence_count: 2,
          median_gap_days: 1,
          p90_gap_days: 1,
          suppressed: true,
        },
      ],
    });
    const rare = layout.nodes.find((n) => n.activity === "rare");
    expect(rare?.suppressed).toBe(true);
    expect(rare?.suppressionNote).toBe(
      "withheld: fewer than the minimum cell count",
    );
    expect(layout.edges[0]?.suppressed).toBe(true);
  });
});

describe("sunburstFromVariants / sankeyFromVariants", () => {
  const variants: VariantSummary[] = [
    {
      variant: "referral-triage-treatment",
      frequency: 6,
      share: 0.6,
      cumulative_share: 0.6,
    },
    {
      variant: "referral-triage-discharge",
      frequency: 3,
      share: 0.3,
      cumulative_share: 0.9,
    },
    { variant: "referral", frequency: 1, share: 0.1, cumulative_share: 1.0 },
  ];

  it("builds a sunburst tree that aggregates shared prefixes and always ends at Stopped", () => {
    const root = sunburstFromVariants({ variants });
    expect(root.value).toBe(10); // every variant's frequency, at the root

    const referral = root.children.find((c) => c.name === "referral");
    expect(referral?.value).toBe(10); // all three variants share this prefix

    const triage = referral?.children.find((c) => c.name === "triage");
    expect(triage?.value).toBe(9); // the two variants that reach triage

    // "referral" alone (the short variant) has its own Stopped leaf,
    // distinct from triage's descendants' Stopped leaves.
    const referralStopped = referral?.children.find((c) => c.name === STOPPED);
    expect(referralStopped?.value).toBe(1);

    const treatment = triage?.children.find((c) => c.name === "treatment");
    const treatmentStopped = treatment?.children.find(
      (c) => c.name === STOPPED,
    );
    expect(treatmentStopped?.value).toBe(6);
  });

  it("flattens the sunburst tree into arcs whose siblings' spans sum to the parent's own span", () => {
    const root = sunburstFromVariants({ variants });
    const arcs = sunburstArcs(root);

    // Depth-1 arcs (direct children of the invisible root) span the
    // full circle between them.
    const depth1 = arcs.filter((a) => a.depth === 1);
    expect(depth1).toHaveLength(1); // only "referral" at depth 1
    expect(depth1[0]?.startAngle).toBeCloseTo(0);
    expect(depth1[0]?.endAngle).toBeCloseTo(Math.PI * 2);

    // "triage"'s two children (treatment, discharge) split its own
    // span in proportion to their values (6 and 3 of triage's 9).
    const triage = arcs.find((a) => a.name === "triage");
    const treatment = arcs.find((a) => a.name === "treatment");
    const discharge = arcs.find((a) => a.name === "discharge");
    expect(triage).toBeDefined();
    expect(treatment).toBeDefined();
    expect(discharge).toBeDefined();
    const triageSpan = triage!.endAngle - triage!.startAngle;
    const treatmentSpan = treatment!.endAngle - treatment!.startAngle;
    const dischargeSpan = discharge!.endAngle - discharge!.startAngle;
    expect(treatmentSpan + dischargeSpan).toBeCloseTo(triageSpan);
    expect(treatmentSpan / dischargeSpan).toBeCloseTo(6 / 3);
  });

  it("handles the empty-variant (zero-length path) case as an immediate Stopped leaf", () => {
    const root = sunburstFromVariants({
      variants: [{ variant: "", frequency: 2, share: 1, cumulative_share: 1 }],
    });
    expect(root.value).toBe(2);
    expect(root.children).toHaveLength(1);
    expect(root.children[0]?.name).toBe(STOPPED);
    expect(root.children[0]?.value).toBe(2);
  });

  it("builds a position-qualified Sankey graph, so the same stage at two positions is two nodes", () => {
    const graph = sankeyFromVariants({ variants });
    const triageAt1 = graph.nodes.find(
      (n) => n.position === 1 && n.label === "triage",
    );
    expect(triageAt1?.value).toBe(9);

    // Every path ends in a Stopped node at its own final position --
    // never merged across differently-long paths.
    const stoppedNodes = graph.nodes.filter((n) => n.label === STOPPED);
    expect(stoppedNodes.length).toBeGreaterThanOrEqual(2);

    // Link weights sum to each variant's own frequency along its path.
    const referralToTriage = graph.links.find(
      (l) => l.source === "0:referral" && l.target === "1:triage",
    );
    expect(referralToTriage?.value).toBe(9);
  });
});

describe("layoutAttrition", () => {
  const steps: AttritionStep[] = [
    {
      label: "enrolled_on_pathway",
      operation: "all instances",
      instances: 20,
      parent: null,
    },
    {
      label: "status_filter",
      operation: "open only",
      instances: 15,
      parent: 0,
    },
    {
      label: "rule_filter",
      operation: "contains stage:triage",
      instances: 15,
      parent: 1,
    },
    {
      label: "matched",
      operation: "matched the rule",
      instances: 6,
      parent: 2,
    },
    {
      label: "complement",
      operation: "did not match",
      instances: 9,
      parent: 2,
    },
  ];

  it("assigns depth by distance from the root and row by sibling order", () => {
    const laid = layoutAttrition(steps);
    expect(laid[0]?.depth).toBe(0);
    expect(laid[1]?.depth).toBe(1);
    expect(laid[2]?.depth).toBe(2);
    expect(laid[3]?.depth).toBe(3);
    expect(laid[4]?.depth).toBe(3);
    // matched/complement fork from the same parent -- distinct rows.
    expect(laid[3]?.row).not.toBe(laid[4]?.row);
  });

  it("never loses a step's own instances/operation/label", () => {
    const laid = layoutAttrition(steps);
    expect(laid[3]?.instances).toBe(6);
    expect(laid[3]?.label).toBe("matched");
    expect(laid[3]?.operation).toBe("matched the rule");
  });
});

function row(
  over: Partial<EventLogRow> &
    Pick<EventLogRow, "case_id" | "activity" | "timestamp">,
): EventLogRow {
  return {
    lifecycle: "start",
    category: null,
    waste: null,
    resource: null,
    location_ref: null,
    pathway_pid: "p1",
    care_setting: null,
    urgency: "routine",
    status: "active",
    outcome: null,
    ...over,
  };
}

describe("dottedChartPoints", () => {
  const rows: EventLogRow[] = [
    row({
      case_id: "b",
      activity: "stage:triage",
      timestamp: "2026-01-02T00:00:00Z",
      category: "value_adding",
    }),
    row({
      case_id: "a",
      activity: "stage:referral",
      timestamp: "2026-01-01T00:00:00Z",
      category: "value_adding",
    }),
    row({
      case_id: "a",
      activity: "step:consent",
      lifecycle: "complete",
      timestamp: "2026-01-01T12:00:00Z",
    }),
    row({
      case_id: "a",
      activity: "stage:triage",
      timestamp: "2026-01-03T00:00:00Z",
      category: "necessary_non_value_adding",
    }),
  ];

  it("rows cases in first-occurrence order, not case-id order", () => {
    const chart = dottedChartPoints(rows);
    expect(chart.caseOrder).toEqual(["b", "a"]);
    expect(chart.points.find((p) => p.caseId === "b")?.caseRow).toBe(0);
    expect(chart.points.find((p) => p.caseId === "a")?.caseRow).toBe(1);
  });

  it("extracts the stage from a stage: activity, and leaves a step: row uncoloured", () => {
    const chart = dottedChartPoints(rows);
    const stepPoint = chart.points.find(
      (p) => p.caseId === "a" && p.stage === null,
    );
    expect(stepPoint).toBeDefined();
    const stagePoint = chart.points.find(
      (p) =>
        p.caseId === "a" &&
        p.timestampMs === Date.parse("2026-01-03T00:00:00Z"),
    );
    expect(stagePoint?.stage).toBe("triage");
  });

  it("spans min/max across every row, not just one case", () => {
    const chart = dottedChartPoints(rows);
    expect(chart.minMs).toBe(Date.parse("2026-01-01T00:00:00Z"));
    expect(chart.maxMs).toBe(Date.parse("2026-01-03T00:00:00Z"));
  });

  it("returns a zeroed, empty chart for no rows rather than an Infinity", () => {
    const chart = dottedChartPoints([]);
    expect(chart.points).toEqual([]);
    expect(chart.minMs).toBe(0);
    expect(chart.maxMs).toBe(0);
  });
});

describe("withheldLabel", () => {
  it("prefers the service's own suppression_note when present", () => {
    expect(withheldLabel("withheld: fewer than the minimum cell count")).toBe(
      "withheld: fewer than the minimum cell count",
    );
  });

  it("falls back to a stated withheld label when only a boolean flag exists", () => {
    expect(withheldLabel(null)).toBe("withheld (n < 5)");
    expect(withheldLabel(undefined)).toBe("withheld (n < 5)");
  });
});
