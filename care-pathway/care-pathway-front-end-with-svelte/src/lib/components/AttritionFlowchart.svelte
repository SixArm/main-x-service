<!--
  A CONSORT-style attrition flowchart (spec T-14g/T-14l): a box per
  pipeline step, an arrow to each step it narrows from, and the
  instance count that survived it — the denominator explained inside
  the page rather than in a log. A step whose count is unchanged from
  its own parent is a step this crate discloses but does not yet
  enforce (`window`, `degenerate_clock`, `coverage_floor`); that is
  shown as a note under the box, not hidden.
-->
<script lang="ts">
  import type { AttritionStep } from "$lib/api/tba";
  import { layoutAttrition } from "$lib/analytics-transforms";

  interface Props {
    steps: AttritionStep[];
  }
  let { steps }: Props = $props();

  const laidOut = $derived(layoutAttrition(steps));
  const BOX_WIDTH = 190;
  const BOX_HEIGHT = 54;
  const COLUMN_GAP = 40;
  const ROW_GAP = 16;
  const MARGIN = 16;

  const positions = $derived(
    laidOut.map((node) => ({
      node,
      x: MARGIN + node.depth * (BOX_WIDTH + COLUMN_GAP),
      y: MARGIN + node.row * (BOX_HEIGHT + ROW_GAP),
    })),
  );
  const byIndex = $derived(new Map(positions.map((p) => [p.node.index, p])));
  const width = $derived(
    MARGIN * 2 +
      (Math.max(0, ...laidOut.map((n) => n.depth)) + 1) *
        (BOX_WIDTH + COLUMN_GAP) -
      COLUMN_GAP,
  );
  const height = $derived(
    MARGIN * 2 +
      (Math.max(0, ...laidOut.map((n) => n.row)) + 1) * (BOX_HEIGHT + ROW_GAP) -
      ROW_GAP,
  );

  /** Unchanged from its own parent — this crate's own disclosed-but-
   * not-enforced steps (window/degenerate_clock/coverage_floor). */
  function isUnenforced(node: (typeof laidOut)[number]): boolean {
    if (node.parentIndex === null) return false;
    const parent = laidOut[node.parentIndex];
    return parent !== undefined && parent.instances === node.instances;
  }
</script>

<div class="attrition-wrap" data-testid="attrition-flowchart">
  <svg
    viewBox={`0 0 ${width} ${height}`}
    role="img"
    aria-label="Cohort attrition flowchart"
  >
    <defs>
      <marker
        id="attrition-arrowhead"
        markerWidth="8"
        markerHeight="8"
        refX="6"
        refY="4"
        orient="auto"
      >
        <path d="M0,0 L8,4 L0,8 Z" fill="rgb(128 128 128 / 0.7)" />
      </marker>
    </defs>
    {#each positions as { node, x, y } (node.index)}
      {#if node.parentIndex !== null}
        {@const parentPos = byIndex.get(node.parentIndex)}
        {#if parentPos}
          <line
            x1={parentPos.x + BOX_WIDTH}
            y1={parentPos.y + BOX_HEIGHT / 2}
            x2={x}
            y2={y + BOX_HEIGHT / 2}
            class="arrow"
          />
        {/if}
      {/if}
      <rect
        {x}
        {y}
        width={BOX_WIDTH}
        height={BOX_HEIGHT}
        class="box"
        class:unenforced={isUnenforced(node)}
      />
      <text x={x + 8} y={y + 18} class="box-label">{node.label}</text>
      <text x={x + 8} y={y + 36} class="box-count">{node.instances}</text>
      <text x={x + BOX_WIDTH - 8} y={y + 36} class="box-op" text-anchor="end"
        >{node.operation}</text
      >
    {/each}
  </svg>
</div>

<style>
  .attrition-wrap {
    overflow-x: auto;
    margin: 1rem 0;
  }
  svg {
    display: block;
    min-width: 100%;
  }
  .box {
    fill: rgb(60 120 200 / 0.12);
    stroke: rgb(60 120 200 / 0.6);
    rx: 4;
  }
  .box.unenforced {
    fill: none;
    stroke: rgb(128 128 128 / 0.5);
    stroke-dasharray: 4 3;
  }
  .box-label {
    font-size: 0.7rem;
    font-weight: 600;
    fill: currentColor;
  }
  .box-count {
    font-size: 0.9rem;
    font-variant-numeric: tabular-nums;
    fill: currentColor;
  }
  .box-op {
    font-size: 0.6rem;
    opacity: 0.75;
    fill: currentColor;
  }
  .arrow {
    stroke: rgb(128 128 128 / 0.7);
    stroke-width: 1.5;
    marker-end: url(#attrition-arrowhead);
  }
</style>
