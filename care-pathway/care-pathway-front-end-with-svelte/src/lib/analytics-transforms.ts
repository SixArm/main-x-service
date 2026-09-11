// Pure, DB-free (and network-free) transforms over the T-14 analytics
// payloads: the directly-follows process map into a layered SVG
// layout, journey variants into a sunburst tree and a Sankey graph
// (both terminated by a synthetic "Stopped" node — the diagram
// convention, not something the service sends), a CONSORT attrition
// trail into a tree layout, and a parsed event log into dotted-chart
// points. Every function here is a pure `data in -> data out`
// transform with no DOM, no fetch, no Svelte — see
// `tests/unit/analytics-transforms.test.ts` for the acceptance-cited
// unit coverage (spec `13-tasks.md` T-14l).
//
// No new graph/charting dependency: `/time` draws all of this itself,
// as plain SVG, from the layouts these functions compute.

import type {
  AttritionStep,
  EventLogRow,
  ProcessMapEdge,
  ProcessMapNode,
  VariantSummary,
} from "./api/tba";

export type { AttritionStep, EventLogRow };

// ---- process map (directly-follows graph) layout -----------------

/** One laid-out process-map node. */
export interface ProcessMapLayoutNode {
  activity: string;
  /** BFS distance from the `start` pseudo-node — the layer/column. */
  rank: number;
  /** Position within its rank — the row. */
  row: number;
  x: number;
  y: number;
  /** Scaled so *area* (not radius) is proportional to `instanceCount`. */
  radius: number;
  instanceCount: number;
  occurrenceCount: number;
  medianDurationDays: number | null;
  suppressed: boolean;
  suppressionNote: string | null;
}

/** One laid-out process-map edge, referencing its two endpoints. */
export interface ProcessMapLayoutEdge {
  from: string;
  to: string;
  fromNode: ProcessMapLayoutNode;
  toNode: ProcessMapLayoutNode;
  /** A return to an earlier-or-same rank — drawn as a curved back-edge. */
  isBackEdge: boolean;
  isSelfLoop: boolean;
  instanceCount: number;
  occurrenceCount: number;
  medianGapDays: number;
  p90GapDays: number;
  suppressed: boolean;
}

/** A complete layered layout, ready to render as SVG. */
export interface ProcessMapLayout {
  nodes: ProcessMapLayoutNode[];
  edges: ProcessMapLayoutEdge[];
  width: number;
  height: number;
}

const COLUMN_WIDTH = 160;
const ROW_HEIGHT = 90;
const MARGIN = 60;
const MIN_RADIUS = 18;
const MAX_RADIUS = 44;

/** A process-map response's suppression-decorated node/edge shape. */
interface Suppressible {
  suppressed?: boolean;
  suppression_note?: string | null;
}

/**
 * Lay out a directly-follows process map (spec T-14b) as a set of
 * ranked columns — an in-house layered layout, not a force-directed
 * one, so the same map always draws the same way.
 *
 * Ranking is BFS distance from the `start` pseudo-node (every real
 * sequence begins there, per `analytics::START_NODE`), which — unlike
 * a topological longest-path — stays well-defined even though the
 * graph may contain cycles: this crate's own process map deliberately
 * keeps self-loops ("a return to a stage/step is a finding"), and a
 * real cohort can bounce between two stages more than once. An edge
 * whose target rank is not strictly greater than its source rank
 * (including a pure self-loop) is flagged `isBackEdge`/`isSelfLoop`
 * for the caller to draw as a curve rather than a straight line, and
 * is excluded from the BFS frontier so it cannot re-lower a rank
 * that a forward path already fixed.
 *
 * A node with no path from `start` at all (should not happen against
 * a real response — every activity comes from a real sequence — but
 * not assumed) is placed one rank past the highest reached rank,
 * rather than dropped.
 */
export function layoutProcessMap(map: {
  nodes: (ProcessMapNode & Suppressible)[];
  edges: (ProcessMapEdge & Suppressible)[];
}): ProcessMapLayout {
  const byActivity = new Map(map.nodes.map((node) => [node.activity, node]));
  const adjacency = new Map<string, string[]>();
  for (const edge of map.edges) {
    if (edge.from === edge.to) continue; // self-loops never advance rank
    const list = adjacency.get(edge.from) ?? [];
    list.push(edge.to);
    adjacency.set(edge.from, list);
  }

  const rank = new Map<string, number>();
  const start = byActivity.has("start") ? "start" : map.nodes[0]?.activity;
  if (start !== undefined) {
    rank.set(start, 0);
    const queue: string[] = [start];
    while (queue.length > 0) {
      const current = queue.shift();
      if (current === undefined) break;
      const currentRank = rank.get(current) ?? 0;
      for (const next of adjacency.get(current) ?? []) {
        if (!rank.has(next)) {
          rank.set(next, currentRank + 1);
          queue.push(next);
        }
      }
    }
  }
  const maxReachedRank = Math.max(0, ...rank.values());
  for (const node of map.nodes) {
    if (!rank.has(node.activity)) rank.set(node.activity, maxReachedRank + 1);
  }

  const maxInstances = Math.max(1, ...map.nodes.map((n) => n.instance_count));
  const byRank = new Map<number, string[]>();
  for (const node of map.nodes) {
    const r = rank.get(node.activity) ?? 0;
    const list = byRank.get(r) ?? [];
    list.push(node.activity);
    byRank.set(r, list);
  }

  const layoutNodes = new Map<string, ProcessMapLayoutNode>();
  for (const [r, activities] of byRank) {
    activities.sort();
    activities.forEach((activity, row) => {
      const node = byActivity.get(activity);
      if (!node) return;
      const areaScale = Math.sqrt(node.instance_count / maxInstances);
      layoutNodes.set(activity, {
        activity,
        rank: r,
        row,
        x: MARGIN + r * COLUMN_WIDTH,
        y: MARGIN + row * ROW_HEIGHT,
        radius: MIN_RADIUS + (MAX_RADIUS - MIN_RADIUS) * areaScale,
        instanceCount: node.instance_count,
        occurrenceCount: node.occurrence_count,
        medianDurationDays: node.median_duration_days ?? null,
        suppressed: node.suppressed ?? false,
        suppressionNote: node.suppression_note ?? null,
      });
    });
  }

  const nodes = [...layoutNodes.values()];
  const edges: ProcessMapLayoutEdge[] = [];
  for (const edge of map.edges) {
    const fromNode = layoutNodes.get(edge.from);
    const toNode = layoutNodes.get(edge.to);
    if (!fromNode || !toNode) continue;
    edges.push({
      from: edge.from,
      to: edge.to,
      fromNode,
      toNode,
      isSelfLoop: edge.from === edge.to,
      isBackEdge: edge.from !== edge.to && toNode.rank <= fromNode.rank,
      instanceCount: edge.instance_count,
      occurrenceCount: edge.occurrence_count,
      medianGapDays: edge.median_gap_days,
      p90GapDays: edge.p90_gap_days,
      suppressed: edge.suppressed ?? false,
    });
  }

  const maxRank = Math.max(0, ...nodes.map((n) => n.rank));
  const maxRow = Math.max(0, ...nodes.map((n) => n.row));
  return {
    nodes,
    edges,
    width: MARGIN * 2 + (maxRank + 1) * COLUMN_WIDTH,
    height: MARGIN * 2 + (maxRow + 1) * ROW_HEIGHT,
  };
}

// ---- variants: sunburst -------------------------------------------

/** The synthetic terminal every path ends at — a diagram convention
 * (TreatmentPatterns' own), not a value the service ever sends. */
export const STOPPED = "Stopped";

/** One node of the sunburst tree, aggregated by shared path prefix. */
export interface SunburstNode {
  /** The stage token this node represents, or `""` for the root. */
  name: string;
  /** Aggregated frequency of every variant passing through this node. */
  value: number;
  depth: number;
  children: SunburstNode[];
}

/**
 * Build the sunburst tree from a variants report's Pareto (spec
 * T-14c/T-14l): each variant string splits on `-` into its stage
 * sequence, always ending in the synthetic {@link STOPPED} terminal so
 * every path — long or short — has a visible, labelled leaf rather
 * than trailing off. Shared prefixes across variants share the same
 * ancestor node, aggregating their frequencies, which is what makes
 * the diagram a sunburst rather than a flat list of paths.
 */
export function sunburstFromVariants(report: {
  variants: VariantSummary[];
}): SunburstNode {
  const root: SunburstNode = { name: "", value: 0, depth: 0, children: [] };
  for (const entry of report.variants) {
    const tokens = entry.variant.length > 0 ? entry.variant.split("-") : [];
    let node = root;
    node.value += entry.frequency;
    for (const token of [...tokens, STOPPED]) {
      let child = node.children.find((c) => c.name === token);
      if (!child) {
        child = { name: token, value: 0, depth: node.depth + 1, children: [] };
        node.children.push(child);
      }
      child.value += entry.frequency;
      node = child;
    }
  }
  return root;
}

/** One arc of a rendered sunburst: a depth (ring) and an angular span
 * proportional to `value`, in radians, `0` at the top going clockwise. */
export interface SunburstArc {
  name: string;
  value: number;
  depth: number;
  startAngle: number;
  endAngle: number;
}

/**
 * Flatten a {@link SunburstNode} tree into the arcs a renderer draws:
 * each node's angular span is its share of its *parent's* value,
 * scaled into the parent's own span — the standard sunburst/icicle
 * partition, computed once here so the rendering layer is just "draw
 * this ring segment", not itself responsible for the arithmetic.
 */
export function sunburstArcs(root: SunburstNode): SunburstArc[] {
  const arcs: SunburstArc[] = [];
  function visit(node: SunburstNode, start: number, end: number): void {
    if (node.depth > 0) {
      arcs.push({
        name: node.name,
        value: node.value,
        depth: node.depth,
        startAngle: start,
        endAngle: end,
      });
    }
    const total = node.value > 0 ? node.value : 1;
    let angle = start;
    for (const child of node.children) {
      const span = (end - start) * (child.value / total);
      visit(child, angle, angle + span);
      angle += span;
    }
  }
  visit(root, 0, Math.PI * 2);
  return arcs;
}

// ---- variants: Sankey ----------------------------------------------

/** One Sankey node: a stage token at a specific position in the
 * sequence (so the same stage name at position 1 and position 3 are
 * two distinct nodes — merging them would draw a misleading loop). */
export interface SankeyNode {
  id: string;
  position: number;
  label: string;
  value: number;
}

/** One weighted flow between two {@link SankeyNode} ids. */
export interface SankeyLink {
  source: string;
  target: string;
  value: number;
}

/** A complete Sankey graph. */
export interface SankeyGraph {
  nodes: SankeyNode[];
  links: SankeyLink[];
}

function sankeyNodeId(position: number, label: string): string {
  return `${position}:${label}`;
}

/**
 * Build the position-qualified Sankey graph from a variants report
 * (spec T-14c/T-14l), again always flowing into {@link STOPPED}. Every
 * path's weight is its variant's raw `frequency` (not the renormalised
 * `share`) so the link widths sum to the same visible-instance total
 * the sunburst does.
 */
export function sankeyFromVariants(report: {
  variants: VariantSummary[];
}): SankeyGraph {
  const nodes = new Map<string, SankeyNode>();
  const links = new Map<string, SankeyLink>();

  const touch = (position: number, label: string, weight: number) => {
    const id = sankeyNodeId(position, label);
    const existing = nodes.get(id);
    if (existing) {
      existing.value += weight;
    } else {
      nodes.set(id, { id, position, label, value: weight });
    }
    return id;
  };
  const link = (source: string, target: string, weight: number) => {
    const key = `${source}->${target}`;
    const existing = links.get(key);
    if (existing) {
      existing.value += weight;
    } else {
      links.set(key, { source, target, value: weight });
    }
  };

  for (const entry of report.variants) {
    const tokens = entry.variant.length > 0 ? entry.variant.split("-") : [];
    const path = [...tokens, STOPPED];
    let previousId: string | null = null;
    path.forEach((label, index) => {
      const id = touch(index, label, entry.frequency);
      if (previousId !== null) link(previousId, id, entry.frequency);
      previousId = id;
    });
  }

  return { nodes: [...nodes.values()], links: [...links.values()] };
}

// ---- attrition tree layout -----------------------------------------

/** One laid-out attrition-flowchart box. */
export interface AttritionLayoutNode extends AttritionStep {
  index: number;
  parentIndex: number | null;
  depth: number;
  row: number;
}

/**
 * Lay out a CONSORT attrition trail (spec T-14g) as a tree: `depth` is
 * distance from the root step (the one entry with `parent: null`),
 * `row` is order among siblings at that depth — enough for a simple
 * box-and-arrow flowchart with no crossing lines, since this crate's
 * own attrition trail is a narrow tree (at most one branch, for a
 * rule split's `matched`/`complement` fork).
 */
export function layoutAttrition(steps: AttritionStep[]): AttritionLayoutNode[] {
  const childCounts = new Map<number, number>();
  return steps.map((step, index) => {
    const depth = depthOf(steps, index);
    const row = step.parent === null ? 0 : (childCounts.get(step.parent) ?? 0);
    if (step.parent !== null) {
      childCounts.set(step.parent, row + 1);
    }
    return { ...step, index, parentIndex: step.parent, depth, row };
  });
}

function depthOf(steps: AttritionStep[], index: number): number {
  let depth = 0;
  let current = steps[index];
  const seen = new Set<number>();
  while (current && current.parent !== null && !seen.has(current.parent)) {
    seen.add(current.parent);
    depth += 1;
    current = steps[current.parent];
  }
  return depth;
}

// ---- dotted chart ----------------------------------------------------

/** One plotted dot: one case's one activity, at its own row and x. */
export interface DottedChartPoint {
  caseId: string;
  /** 0-based row, one per distinct case, first-occurrence order. */
  caseRow: number;
  timestampMs: number;
  /** The stage name, when `activity` is a `stage:<name>` row; `null`
   * for a `step:`/`event:` row, which the caller renders in a neutral
   * colour rather than guessing a stage for it. */
  stage: string | null;
  category: string | null;
}

/** A dotted chart's plotted points plus the axes they were built from. */
export interface DottedChart {
  points: DottedChartPoint[];
  /** Case ids, in row order (first-occurrence in the log). */
  caseOrder: string[];
  minMs: number;
  maxMs: number;
}

const STAGE_PREFIX = "stage:";

/** The four {@link EventLogRow} fields a dotted chart actually needs. */
type DottedChartInput = Pick<
  EventLogRow,
  "case_id" | "activity" | "timestamp" | "category"
>;

/**
 * Turn a parsed `event_log` export (spec T-14a) into dotted-chart
 * points: one row per case, one dot per event, positioned by time and
 * coloured by stage — the classic process-mining "dotted chart"
 * (spec T-14l). Cases are rowed in first-occurrence order, not
 * case-id order, so the chart reads top-to-bottom roughly in
 * enrolment order for a cohort whose ids are otherwise random UUIDs.
 */
export function dottedChartPoints(rows: DottedChartInput[]): DottedChart {
  const caseOrder: string[] = [];
  const caseRow = new Map<string, number>();
  const points: DottedChartPoint[] = [];
  let minMs = Number.POSITIVE_INFINITY;
  let maxMs = Number.NEGATIVE_INFINITY;

  for (const row of rows) {
    let rowIndex = caseRow.get(row.case_id);
    if (rowIndex === undefined) {
      rowIndex = caseOrder.length;
      caseRow.set(row.case_id, rowIndex);
      caseOrder.push(row.case_id);
    }
    const timestampMs = Date.parse(row.timestamp);
    if (!Number.isFinite(timestampMs)) continue;
    minMs = Math.min(minMs, timestampMs);
    maxMs = Math.max(maxMs, timestampMs);
    points.push({
      caseId: row.case_id,
      caseRow: rowIndex,
      timestampMs,
      stage: row.activity.startsWith(STAGE_PREFIX)
        ? row.activity.slice(STAGE_PREFIX.length)
        : null,
      category: row.category,
    });
  }

  return {
    points,
    caseOrder,
    minMs: Number.isFinite(minMs) ? minMs : 0,
    maxMs: Number.isFinite(maxMs) ? maxMs : 0,
  };
}

// ---- suppression rendering ------------------------------------------

/**
 * The uniform "withheld" label for a suppressed cell (process-map
 * node/edge, a split side, …) — never a blank, per this task's own
 * acceptance text and the family's disclosure-control convention
 * (`agents/share/time-based-analysis.md` §12.2: hide detail, not the
 * count). Every suppressed-capable payload already carries its own
 * `suppression_note`; this is the fallback for the ones that only
 * carry a boolean `suppressed` flag (a process-map node/edge).
 */
export function withheldLabel(note?: string | null): string {
  return note ?? "withheld (n < 5)";
}
