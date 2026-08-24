<!--
  Cumulative flow diagram — the board's composition over time, as a
  stacked area chart.

  Bands stack bottom-to-top in board order reversed: `done` at the
  bottom (it only ever grows), then in_review, in_progress, blocked, and
  `todo` on top. Read that way the chart answers two questions at a
  glance that no table does: the **vertical gap** between the top line
  and the done band is work in progress, and the **horizontal distance**
  between the two is approximately the cycle time — Little's Law read
  straight off the picture.

  Colours are the validated categorical palette, assigned to bands in a
  fixed order (never cycled, and never re-assigned when a band goes
  empty — colour follows the status, not its rank). Adjacent stacked
  pairs are what the palette's CVD gate is measured on, and this
  assignment puts the bands on adjacent slots deliberately. Three light
  slots sit below 3:1 against the surface, so the table view below is
  not optional decoration — it is the relief the palette requires.
-->
<script lang="ts">
  import type { FlowSample } from "$lib/api/tba";

  interface Props {
    /** Daily samples, oldest first. */
    samples: FlowSample[];
    /** The service's derivation note, shown verbatim. */
    note?: string;
  }
  let { samples, note = "" }: Props = $props();

  /**
   * Stack order, bottom to top, with each band's fixed palette slot.
   * The order is the board's, reversed, so completed work accumulates
   * along the bottom.
   */
  const BANDS = [
    { status: "done", label: "Done", slot: 1 },
    { status: "in_review", label: "In review", slot: 2 },
    { status: "in_progress", label: "In progress", slot: 3 },
    { status: "blocked", label: "Blocked", slot: 4 },
    { status: "todo", label: "Todo", slot: 5 },
  ] as const;

  const W = 720;
  const H = 260;
  const PAD = { top: 12, right: 16, bottom: 30, left: 40 };

  let showTable = $state(false);
  let hover = $state<number | null>(null);

  const peak = $derived(
    Math.max(1, ...samples.map((s) => s.total)),
  );
  const plotW = $derived(W - PAD.left - PAD.right);
  const plotH = $derived(H - PAD.top - PAD.bottom);

  function x(index: number): number {
    const n = samples.length;
    if (n <= 1) return PAD.left + plotW / 2;
    return PAD.left + (index / (n - 1)) * plotW;
  }
  function y(value: number): number {
    return PAD.top + plotH - (value / peak) * plotH;
  }

  /**
   * Cumulative tops per band per sample. Index `b` holds the running
   * total through band `b`, so band `b`'s polygon runs between
   * `tops[b - 1]` and `tops[b]`.
   */
  const tops = $derived(
    BANDS.map((_, b) =>
      samples.map((sample) =>
        BANDS.slice(0, b + 1).reduce(
          (sum, band) => sum + (sample.counts[band.status] ?? 0),
          0,
        ),
      ),
    ),
  );

  /** The filled polygon for one band: its top edge, then back along the one below. */
  function bandPath(b: number): string {
    if (samples.length === 0) return "";
    const upper = tops[b] ?? [];
    const lower = b === 0 ? samples.map(() => 0) : (tops[b - 1] ?? []);
    const forward = upper.map((v, i) => `${x(i)},${y(v)}`).join(" ");
    const back = lower
      .map((v, i) => `${x(i)},${y(v)}`)
      .reverse()
      .join(" ");
    return `${forward} ${back}`;
  }

  /** Whether a band is present anywhere in the window (an all-zero band is dropped from the legend, not recoloured). */
  function bandUsed(status: string): boolean {
    return samples.some((s) => (s.counts[status] ?? 0) > 0);
  }

  const ticks = $derived(
    samples.length === 0
      ? []
      : [0, Math.floor((samples.length - 1) / 2), samples.length - 1]
          .filter((i, at, all) => all.indexOf(i) === at)
          .map((i) => ({ i, label: dayLabel(samples[i]?.at_ms ?? 0) })),
  );

  function dayLabel(ms: number): string {
    const d = new Date(ms);
    return `${d.getUTCDate()} ${d.toLocaleString("en", { month: "short", timeZone: "UTC" })}`;
  }

  /** The sample under the pointer, narrowed for the template. */
  const hoveredSample = $derived(
    hover === null ? undefined : samples[hover],
  );

  /** Nearest sample to a pointer position — the hit area is the whole column, not the line. */
  function onMove(event: PointerEvent) {
    const svg = event.currentTarget as SVGSVGElement;
    const box = svg.getBoundingClientRect();
    const px = ((event.clientX - box.left) / box.width) * W;
    if (samples.length === 0) return;
    const ratio = (px - PAD.left) / plotW;
    hover = Math.max(
      0,
      Math.min(samples.length - 1, Math.round(ratio * (samples.length - 1))),
    );
  }
</script>

<figure class="cfd viz-root">
  <figcaption>
    <strong>Cumulative flow</strong>
    <span class="muted">
      the vertical gap above “Done” is work in progress; the horizontal gap
      to it is roughly the cycle time
    </span>
  </figcaption>

  {#if samples.length === 0}
    <p class="muted">No samples in this window.</p>
  {:else}
    <ul class="legend">
      {#each BANDS as band (band.status)}
        {#if bandUsed(band.status)}
          <li>
            <span class="swatch" style={`background:var(--series-${band.slot})`}
            ></span>
            {band.label}
          </li>
        {/if}
      {/each}
    </ul>

    <svg
      viewBox={`0 0 ${W} ${H}`}
      role="img"
      aria-label={`Cumulative flow over ${samples.length} days, peaking at ${peak} tasks`}
      onpointermove={onMove}
      onpointerleave={() => (hover = null)}
    >
      <!-- Recessive hairline grid: solid, one shade off the surface. -->
      {#each [0, 0.5, 1] as fraction (fraction)}
        <line
          x1={PAD.left}
          x2={W - PAD.right}
          y1={y(peak * fraction)}
          y2={y(peak * fraction)}
          class="grid"
        />
        <text x={PAD.left - 6} y={y(peak * fraction) + 4} class="tick end">
          {Math.round(peak * fraction)}
        </text>
      {/each}

      {#each BANDS as band, b (band.status)}
        {#if bandUsed(band.status)}
          <!-- The 2px surface gap separates touching bands; it is drawn
               as a stroke in the surface colour, never a border. -->
          <polygon
            points={bandPath(b)}
            fill={`var(--series-${band.slot})`}
            stroke="var(--surface-1)"
            stroke-width="2"
            stroke-linejoin="round"
          />
        {/if}
      {/each}

      {#each ticks as tick (tick.i)}
        <!-- The end ticks anchor inward: centring them would push the
             first and last labels outside the plot, where the container
             clips them. -->
        <text
          x={x(tick.i)}
          y={H - 10}
          class="tick {tick.i === 0
            ? 'start'
            : tick.i === samples.length - 1
              ? 'end'
              : 'mid'}">{tick.label}</text
        >
      {/each}

      {#if hover !== null && samples[hover]}
        <line
          x1={x(hover)}
          x2={x(hover)}
          y1={PAD.top}
          y2={PAD.top + plotH}
          class="crosshair"
        />
      {/if}
    </svg>

    {#if hoveredSample}
      {@const sample = hoveredSample}
      <div class="tooltip" role="status">
        <strong>{dayLabel(sample.at_ms)}</strong>
        <span>{sample.total} total · {sample.work_in_progress} in progress · {sample.done} done</span>
        <ul>
          {#each BANDS as band (band.status)}
            {#if (sample.counts[band.status] ?? 0) > 0}
              <li>
                <span
                  class="swatch"
                  style={`background:var(--series-${band.slot})`}
                ></span>
                {band.label}: {sample.counts[band.status]}
              </li>
            {/if}
          {/each}
        </ul>
      </div>
    {/if}

    <button
      type="button"
      class="link"
      onclick={() => (showTable = !showTable)}
      aria-expanded={showTable}
    >
      {showTable ? "Hide" : "Show"} the numbers
    </button>

    {#if showTable}
      <table data-testid="cfd-table">
        <caption class="muted">
          Every value in the chart. {note}
        </caption>
        <thead>
          <tr>
            <th scope="col">Day</th>
            {#each BANDS as band (band.status)}
              <th scope="col">{band.label}</th>
            {/each}
            <th scope="col">Total</th>
          </tr>
        </thead>
        <tbody>
          {#each samples as sample (sample.at_ms)}
            <tr>
              <th scope="row">{dayLabel(sample.at_ms)}</th>
              {#each BANDS as band (band.status)}
                <td>{sample.counts[band.status] ?? 0}</td>
              {/each}
              <td>{sample.total}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</figure>

<style>
  /* The validated categorical palette. Light values on :root; the dark
     steps are declared under BOTH the OS media query and the explicit
     theme stamp, so the app's theme picker wins in either direction. */
  .viz-root {
    color-scheme: light;
    --surface-1: #fcfcfb;
    --text-primary: #0b0b0b;
    --text-secondary: #52514e;
    --grid: #e6e5e1;
    --series-1: #2a78d6;
    --series-2: #eb6834;
    --series-3: #1baf7a;
    --series-4: #eda100;
    --series-5: #e87ba4;
  }
  @media (prefers-color-scheme: dark) {
    :root:where(:not([data-theme="light"])) .viz-root {
      color-scheme: dark;
      --surface-1: #1a1a19;
      --text-primary: #ffffff;
      --text-secondary: #c3c2b7;
      --grid: #383835;
      --series-1: #3987e5;
      --series-2: #d95926;
      --series-3: #199e70;
      --series-4: #c98500;
      --series-5: #d55181;
    }
  }
  :root[data-theme="dark"] .viz-root {
    color-scheme: dark;
    --surface-1: #1a1a19;
    --text-primary: #ffffff;
    --text-secondary: #c3c2b7;
    --grid: #383835;
    --series-1: #3987e5;
    --series-2: #d95926;
    --series-3: #199e70;
    --series-4: #c98500;
    --series-5: #d55181;
  }

  .cfd {
    margin: 0 0 1.5rem;
    background: var(--surface-1);
    color: var(--text-primary);
    padding: 0.75rem;
    border-radius: 6px;
    position: relative;
  }
  figcaption {
    display: flex;
    gap: 0.75rem;
    align-items: baseline;
    flex-wrap: wrap;
    margin-bottom: 0.5rem;
  }
  .muted {
    color: var(--text-secondary);
    font-size: 0.85rem;
  }
  svg {
    width: 100%;
    height: auto;
    display: block;
    touch-action: none;
  }
  .grid {
    stroke: var(--grid);
    stroke-width: 1;
  }
  .crosshair {
    stroke: var(--text-secondary);
    stroke-width: 1;
  }
  .tick {
    fill: var(--text-secondary);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .tick.end {
    text-anchor: end;
  }
  .tick.mid {
    text-anchor: middle;
  }
  .tick.start {
    text-anchor: start;
  }
  .legend {
    list-style: none;
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
    padding: 0;
    margin: 0 0 0.5rem;
    font-size: 0.85rem;
  }
  .legend li,
  .tooltip li {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .swatch {
    width: 12px;
    height: 12px;
    border-radius: 3px;
    display: inline-block;
    flex: none;
  }
  .tooltip {
    margin-top: 0.5rem;
    font-size: 0.85rem;
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .tooltip ul {
    list-style: none;
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
    padding: 0;
    margin: 0;
  }
  button.link {
    background: none;
    border: 0;
    padding: 0.25rem 0;
    color: inherit;
    text-decoration: underline;
    cursor: pointer;
    font-size: 0.85rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
    margin-top: 0.5rem;
  }
  th,
  td {
    text-align: right;
    padding: 0.2rem 0.4rem;
  }
  th[scope="row"] {
    text-align: left;
    font-weight: normal;
  }
  caption {
    text-align: left;
    padding-bottom: 0.4rem;
  }
</style>
