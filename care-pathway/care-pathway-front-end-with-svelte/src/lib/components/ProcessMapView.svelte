<!--
  The directly-follows process map (spec T-14b/T-14l) — never a
  discovered model: every node is an observed activity, every edge an
  observed transition, drawn from `layoutProcessMap`'s own in-house
  layered layout rather than a force-directed one, so the same map
  always draws the same way. Node size is proportional to
  `instance_count` (area, not radius — a raw-radius scale exaggerates
  small differences); the edge label is its median gap in days.
  Self-loops and back-edges (a real cohort can bounce between two
  activities more than once) are drawn as curves rather than straight
  lines, since a layered layout has no "backwards" direction to draw a
  straight line in.

  A suppressed node/edge (T-14k) is drawn hollow, with its counts
  withheld — never silently omitted, per the family's own convention.
-->
<script lang="ts">
  import type { ProcessMapResponse } from "$lib/api/tba";
  import { layoutProcessMap, withheldLabel } from "$lib/analytics-transforms";

  interface Props {
    map: ProcessMapResponse;
  }
  let { map }: Props = $props();

  const layout = $derived(layoutProcessMap(map));

  /** A quadratic curve control point offset above the two endpoints,
   * so a self-loop or back-edge reads as a loop rather than a line. */
  function loopPath(x: number, y: number, radius: number): string {
    const top = y - radius - 40;
    return `M ${x - radius} ${y} C ${x - radius - 30} ${top}, ${x + radius + 30} ${top}, ${x + radius} ${y}`;
  }
  function backEdgePath(
    x1: number,
    y1: number,
    x2: number,
    y2: number,
  ): string {
    const midY = Math.min(y1, y2) - 50;
    return `M ${x1} ${y1} C ${x1} ${midY}, ${x2} ${midY}, ${x2} ${y2}`;
  }
</script>

<div class="process-map-wrap" data-testid="process-map">
  <svg
    viewBox={`0 0 ${layout.width} ${layout.height}`}
    role="img"
    aria-label="Directly-follows process map"
  >
    {#each layout.edges as edge (edge.from + "->" + edge.to)}
      {#if edge.isSelfLoop}
        <path
          d={loopPath(edge.fromNode.x, edge.fromNode.y, edge.fromNode.radius)}
          class="edge"
          class:suppressed={edge.suppressed}
          fill="none"
        />
      {:else if edge.isBackEdge}
        <path
          d={backEdgePath(
            edge.fromNode.x,
            edge.fromNode.y,
            edge.toNode.x,
            edge.toNode.y,
          )}
          class="edge"
          class:suppressed={edge.suppressed}
          fill="none"
        />
      {:else}
        <line
          x1={edge.fromNode.x}
          y1={edge.fromNode.y}
          x2={edge.toNode.x}
          y2={edge.toNode.y}
          class="edge"
          class:suppressed={edge.suppressed}
        />
      {/if}
      {#if !edge.suppressed}
        <text
          x={(edge.fromNode.x + edge.toNode.x) / 2}
          y={(edge.fromNode.y + edge.toNode.y) / 2 - 6}
          class="edge-label"
        >
          {edge.medianGapDays.toFixed(1)}d
        </text>
      {/if}
    {/each}
    {#each layout.nodes as node (node.activity)}
      <circle
        cx={node.x}
        cy={node.y}
        r={node.radius}
        class="node"
        class:suppressed={node.suppressed}
      />
      <text x={node.x} y={node.y - node.radius - 8} class="node-label">
        {node.activity}
      </text>
      <text x={node.x} y={node.y + 4} class="node-count">
        {node.suppressed
          ? withheldLabel(node.suppressionNote)
          : node.instanceCount}
      </text>
    {/each}
  </svg>
</div>

<style>
  .process-map-wrap {
    overflow-x: auto;
    margin: 1rem 0;
  }
  svg {
    display: block;
    min-width: 100%;
  }
  .node {
    fill: rgb(60 120 200 / 0.25);
    stroke: rgb(60 120 200);
    stroke-width: 1.5;
  }
  .node.suppressed {
    fill: none;
    stroke: rgb(128 128 128 / 0.6);
    stroke-dasharray: 4 3;
  }
  .node-label {
    font-size: 0.7rem;
    text-anchor: middle;
    fill: currentColor;
  }
  .node-count {
    font-size: 0.75rem;
    text-anchor: middle;
    font-variant-numeric: tabular-nums;
    fill: currentColor;
  }
  .edge {
    stroke: rgb(128 128 128 / 0.6);
    stroke-width: 1.5;
  }
  .edge.suppressed {
    stroke-dasharray: 3 3;
    opacity: 0.5;
  }
  .edge-label {
    font-size: 0.65rem;
    text-anchor: middle;
    opacity: 0.75;
  }
</style>
