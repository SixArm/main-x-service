<!--
  A dotted chart (spec T-14l): one row per instance, one dot per
  recorded activity, positioned by calendar time and coloured by
  stage — the classic process-mining view of a whole cohort's timing
  side by side. A `step:`/`event:` row (no stage) plots in a neutral
  colour rather than guessing a stage for it.

  Built from the `event_log` bulk export, not fetched on page load —
  see `TbaRepository.eventLog`'s own doc comment for why: it is a
  `Destructive`-gated, audited bulk pull, so this chart only appears
  once the caller has explicitly asked for it.
-->
<script lang="ts">
  import type { EventLogRow } from "$lib/api/tba";
  import { dottedChartPoints } from "$lib/analytics-transforms";

  interface Props {
    rows: EventLogRow[];
  }
  let { rows }: Props = $props();

  const chart = $derived(dottedChartPoints(rows));
  const ROW_HEIGHT = 10;
  const MARGIN = 40;
  const PLOT_WIDTH = 640;
  const span = $derived(Math.max(1, chart.maxMs - chart.minMs));
  const width = $derived(MARGIN * 2 + PLOT_WIDTH);
  const height = $derived(
    MARGIN * 2 + Math.max(1, chart.caseOrder.length) * ROW_HEIGHT,
  );

  function xOf(timestampMs: number): number {
    return MARGIN + ((timestampMs - chart.minMs) / span) * PLOT_WIDTH;
  }
  function yOf(caseRow: number): number {
    return MARGIN + caseRow * ROW_HEIGHT + ROW_HEIGHT / 2;
  }

  const STAGE_COLOR: Record<string, string> = {
    referral: "#3a78c8",
    triage: "#c8783a",
    diagnostics: "#8a3ac8",
    treatment: "#3ac878",
    follow_up: "#c8b03a",
    discharge: "#c83a5a",
    other: "#888888",
  };
  function colorOf(stage: string | null): string {
    return (stage && STAGE_COLOR[stage]) || "#a0a0a0";
  }
</script>

<div class="dotted-chart-wrap" data-testid="dotted-chart">
  {#if chart.points.length === 0}
    <p class="muted">No events to plot.</p>
  {:else}
    <svg
      viewBox={`0 0 ${width} ${height}`}
      role="img"
      aria-label="Dotted chart of the cohort's recorded activity"
    >
      {#each chart.points as point, index (point.caseId + ":" + point.timestampMs + ":" + index)}
        <circle
          cx={xOf(point.timestampMs)}
          cy={yOf(point.caseRow)}
          r={2.2}
          fill={colorOf(point.stage)}
        >
          <title>{point.caseId} — {point.stage ?? "other"}</title>
        </circle>
      {/each}
    </svg>
    <ul class="legend">
      {#each Object.entries(STAGE_COLOR) as [stage, color] (stage)}
        <li>
          <span class="swatch" style={`background:${color}`}></span>{stage}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .dotted-chart-wrap {
    overflow-x: auto;
    margin: 1rem 0;
  }
  svg {
    display: block;
    min-width: 100%;
  }
  .legend {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
    padding: 0;
    margin: 0.5rem 0;
    font-size: 0.75rem;
  }
  .legend li {
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  .swatch {
    width: 0.7rem;
    height: 0.7rem;
    border-radius: 50%;
    display: inline-block;
  }
  .muted {
    opacity: 0.75;
    font-size: 0.9rem;
  }
</style>
