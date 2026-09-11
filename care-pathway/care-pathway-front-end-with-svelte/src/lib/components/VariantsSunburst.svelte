<!--
  Journey variants as a sunburst (spec T-14c/T-14l): each ring is a
  position in the pathway string, each arc's angular span is
  proportional to how many instances share that path so far, and every
  path ends at the synthetic Stopped terminal — TreatmentPatterns' own
  diagram convention for "this is where the cohort stopped following
  the pattern", not a value the service sends.

  A variant folded below the minimum cell count (T-14k) never appears
  as its own arc; `suppressed_instances` is disclosed as a count in
  the caption instead, never as which variants they were.
-->
<script lang="ts">
  import type { VariantsReport } from "$lib/api/tba";
  import {
    STOPPED,
    sunburstArcs,
    sunburstFromVariants,
  } from "$lib/analytics-transforms";

  interface Props {
    report: VariantsReport;
  }
  let { report }: Props = $props();

  const arcs = $derived(sunburstArcs(sunburstFromVariants(report)));
  const maxDepth = $derived(Math.max(1, ...arcs.map((a) => a.depth)));
  const RING_WIDTH = 34;
  const INNER_RADIUS = 30;
  const size = $derived(INNER_RADIUS * 2 + maxDepth * RING_WIDTH * 2 + 20);
  const center = $derived(size / 2);

  function point(radius: number, angle: number): [number, number] {
    // 0 rad at the top, clockwise.
    return [
      center + radius * Math.sin(angle),
      center - radius * Math.cos(angle),
    ];
  }

  function arcPath(
    depth: number,
    startAngle: number,
    endAngle: number,
  ): string {
    const inner = INNER_RADIUS + (depth - 1) * RING_WIDTH;
    const outer = inner + RING_WIDTH;
    const full = endAngle - startAngle >= Math.PI * 2 - 1e-6;
    // A full circle needs two arcs (SVG can't draw a 360° arc in one).
    const end = full ? startAngle + Math.PI * 2 - 1e-6 : endAngle;
    const [x1, y1] = point(outer, startAngle);
    const [x2, y2] = point(outer, end);
    const [x3, y3] = point(inner, end);
    const [x4, y4] = point(inner, startAngle);
    const large = end - startAngle > Math.PI ? 1 : 0;
    return [
      `M ${x1} ${y1}`,
      `A ${outer} ${outer} 0 ${large} 1 ${x2} ${y2}`,
      `L ${x3} ${y3}`,
      `A ${inner} ${inner} 0 ${large} 0 ${x4} ${y4}`,
      "Z",
    ].join(" ");
  }

  function labelPoint(
    depth: number,
    startAngle: number,
    endAngle: number,
  ): [number, number] {
    const mid = (startAngle + endAngle) / 2;
    const radius = INNER_RADIUS + (depth - 1) * RING_WIDTH + RING_WIDTH / 2;
    return point(radius, mid);
  }

  /** Wide-enough arcs get their own label; narrow slivers would just
   * overlap illegibly, so they are named in the legend/hover instead. */
  function labelFits(startAngle: number, endAngle: number): boolean {
    return endAngle - startAngle > 0.18;
  }
</script>

<div class="sunburst-wrap" data-testid="variants-sunburst">
  <svg
    viewBox={`0 0 ${size} ${size}`}
    role="img"
    aria-label="Journey variants sunburst"
  >
    {#each arcs as arc (arc.depth + ":" + arc.startAngle)}
      <path
        d={arcPath(arc.depth, arc.startAngle, arc.endAngle)}
        class="arc"
        class:stopped={arc.name === STOPPED}
      >
        <title
          >{arc.name} — {arc.value} instance{arc.value === 1 ? "" : "s"}</title
        >
      </path>
      {#if labelFits(arc.startAngle, arc.endAngle)}
        {@const [lx, ly] = labelPoint(arc.depth, arc.startAngle, arc.endAngle)}
        <text x={lx} y={ly} class="arc-label">{arc.name}</text>
      {/if}
    {/each}
  </svg>
  {#if report.suppressed_instances > 0}
    <p class="muted" data-testid="variants-suppressed">
      {report.suppressed_instances} instance{report.suppressed_instances === 1
        ? ""
        : "s"}
      folded into no variant shown — each one's own variant was below the minimum
      cell count.
    </p>
  {/if}
</div>

<style>
  .sunburst-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.5rem;
    margin: 1rem 0;
  }
  svg {
    max-width: 26rem;
    width: 100%;
  }
  .arc {
    fill: rgb(60 120 200 / 0.35);
    stroke: var(--app-bg, white);
    stroke-width: 1;
  }
  .arc.stopped {
    fill: rgb(128 128 128 / 0.3);
  }
  .arc-label {
    font-size: 0.6rem;
    text-anchor: middle;
    dominant-baseline: middle;
    fill: currentColor;
    pointer-events: none;
  }
  .muted {
    opacity: 0.75;
    font-size: 0.85rem;
    text-align: center;
  }
</style>
