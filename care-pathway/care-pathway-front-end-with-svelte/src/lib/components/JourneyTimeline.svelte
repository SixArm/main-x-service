<!--
  The timeline wall — one patient journey drawn to scale, from clock
  start to clock stop.

  This is the picture time-based analysis exists to produce. Every
  millisecond of the journey is one band, sized in proportion to its
  duration and coloured by whether it added value. Read left to right it
  answers the question a table cannot: *where did the time actually go?*

  Four bands, and the fourth is the point. Value-adding, necessary
  non-value-adding and unnecessary non-value-adding are the
  value-stream-mapping categories; **unrecorded** is clock time no
  segment covers, and on a real journey it is usually the widest band on
  the wall. It is drawn in a neutral rather than a hue because it means
  "nobody recorded this", not "a fourth kind of activity" — and it is
  never dropped, because dropping it is exactly how a journey that was
  never mapped comes to look efficient.

  The three hues are the validated categorical palette's first three
  slots, which clear the colour-vision gates on every pair, not merely
  on adjacent ones. Aqua sits below 3:1 on the light surface, so the
  table view below is the relief that requires — not optional decoration.
-->
<script lang="ts">
  import type { WallEntry, Clock } from "$lib/api/tba";
  import { percent } from "$lib/api/tba";

  interface Props {
    /** Segments and gaps interleaved in time order. */
    wall: WallEntry[];
    /** The clock the journey was measured against. */
    clock: Clock;
    /** The service's derivation note, shown verbatim. */
    note?: string;
  }
  let { wall, clock, note = "" }: Props = $props();

  /** Category → palette slot. `unrecorded` takes the neutral, not a hue. */
  const BAND: Record<string, { slot: string; label: string }> = {
    value_adding: { slot: "1", label: "Care" },
    necessary_non_value_adding: { slot: "2", label: "Necessary, not care" },
    unnecessary_non_value_adding: { slot: "3", label: "Waste" },
    unrecorded: { slot: "0", label: "Unrecorded" },
  };

  /** A gap is unrecorded time; a segment carries its own category. */
  function categoryOf(entry: WallEntry): string {
    return entry.kind === "gap" ? "unrecorded" : (entry.category ?? "unrecorded");
  }

  const total = $derived(
    Math.max(
      1,
      wall.reduce((sum, entry) => sum + Math.max(0, entry.duration_ms), 0),
    ),
  );

  /** Bands that actually occur, in the order the legend lists them. */
  const present = $derived(
    ["value_adding", "necessary_non_value_adding", "unnecessary_non_value_adding", "unrecorded"].filter(
      (category) => wall.some((entry) => categoryOf(entry) === category && entry.duration_ms > 0),
    ),
  );

  /** Per-category totals, for the legend and the table. */
  const totals = $derived(
    Object.fromEntries(
      present.map((category) => [
        category,
        wall
          .filter((entry) => categoryOf(entry) === category)
          .reduce((sum, entry) => sum + Math.max(0, entry.duration_ms), 0),
      ]),
    ) as Record<string, number>,
  );

  let showTable = $state(false);
  let focused = $state<number | null>(null);

  const drawn = $derived(wall.filter((entry) => entry.duration_ms > 0));

  /** The band under the pointer or keyboard focus, narrowed for the template. */
  const focusedEntry = $derived(focused === null ? undefined : drawn[focused]);

  /**
   * The single longest band. Direct-labelling every band would be
   * unreadable, so the wall names the one that matters and lets hover,
   * focus and the table carry the rest.
   */
  const longest = $derived(
    drawn.reduce<(typeof drawn)[number] | undefined>(
      (best, entry) =>
        best === undefined || entry.duration_ms > best.duration_ms ? entry : best,
      undefined,
    ),
  );

  function widthOf(entry: WallEntry): string {
    return `${(entry.duration_ms / total) * 100}%`;
  }

  function dayLabel(iso: string | undefined, ms?: number): string {
    const value = iso ? new Date(iso) : new Date(ms ?? 0);
    return value.toISOString().slice(0, 10);
  }
</script>

<figure class="wall viz-root">
  <figcaption>
    <strong>The journey, to scale</strong>
    <span class="muted">
      {dayLabel(undefined, clock.start_ms)} → {dayLabel(undefined, clock.stop_ms)}
      {#if clock.running}(still running){/if}
      · clock from <code>{clock.start_source}</code> to
      <code>{clock.stop_source}</code>
    </span>
  </figcaption>

  {#if drawn.length === 0}
    <p class="muted">Nothing to draw — this journey has no measurable clock.</p>
  {:else}
    <ul class="legend">
      {#each present as category (category)}
        <li>
          <span class="swatch slot-{BAND[category]?.slot}"></span>
          {BAND[category]?.label ?? category}
          <span class="muted">
            {percent((totals[category] ?? 0) / total, 1)}
          </span>
        </li>
      {/each}
    </ul>

    <!-- One row, sized by duration. Each band is a button so a keyboard
         reaches exactly what a pointer does. -->
    <div class="bar" role="group" aria-label="Journey timeline">
      {#each drawn as entry, i (i)}
        <button
          type="button"
          class="band slot-{BAND[categoryOf(entry)]?.slot} {entry.kind}"
          style={`width:${widthOf(entry)}`}
          title={`${entry.label} — ${entry.duration_days.toFixed(1)}d`}
          aria-label={`${entry.label}, ${entry.duration_days.toFixed(1)} days, ${BAND[categoryOf(entry)]?.label}`}
          onmouseenter={() => (focused = i)}
          onmouseleave={() => (focused = null)}
          onfocus={() => (focused = i)}
          onblur={() => (focused = null)}
        ></button>
      {/each}
    </div>

    <!-- Selective labels only: the longest band is named on the wall,
         and everything else is reachable by hover, focus, or the table. -->
    <p class="readout" role="status">
      {#if focusedEntry}
        <strong>{focusedEntry.label}</strong>
        · {focusedEntry.duration_days.toFixed(1)}d
        · {BAND[categoryOf(focusedEntry)]?.label}
        {#if focusedEntry.stage}· {focusedEntry.stage}{/if}
        {#if focusedEntry.at_handoff}· at a handoff{/if}
      {:else if longest}
        <span class="muted">
          Longest single stretch: <strong>{longest.label}</strong>,
          {longest.duration_days.toFixed(1)}d
          ({percent(longest.duration_ms / total, 1)} of the journey).
          Hover or tab a band for its detail.
        </span>
      {/if}
    </p>

    <button
      type="button"
      class="link"
      onclick={() => (showTable = !showTable)}
      aria-expanded={showTable}
    >
      {showTable ? "Hide" : "Show"} the numbers
    </button>

    {#if showTable}
      <table data-testid="wall-table">
        <caption class="muted">Every band on the wall. {note}</caption>
        <thead>
          <tr>
            <th scope="col">What</th>
            <th scope="col">Kind</th>
            <th scope="col">Category</th>
            <th scope="col">Stage</th>
            <th scope="col">Days</th>
            <th scope="col">Share</th>
          </tr>
        </thead>
        <tbody>
          {#each drawn as entry, i (i)}
            <tr>
              <th scope="row">{entry.label}</th>
              <td>{entry.kind}</td>
              <td>{BAND[categoryOf(entry)]?.label ?? categoryOf(entry)}</td>
              <td>{entry.stage ?? "—"}</td>
              <td>{entry.duration_days.toFixed(1)}</td>
              <td>{percent(entry.duration_ms / total, 1)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</figure>

<style>
  /* The validated categorical palette. Light on :root; the dark steps
     under BOTH the OS media query and the explicit theme stamp, so the
     app's theme picker wins either way. The figure paints its own ink
     as well as its own ground — a container that sets a background and
     inherits the host's text colour is legible only while the two
     themes happen to agree. */
  .viz-root {
    color-scheme: light;
    --surface-1: #fcfcfb;
    --text-primary: #0b0b0b;
    --text-secondary: #52514e;
    --neutral: #d7d5cf;
    --series-1: #2a78d6;
    --series-2: #eb6834;
    --series-3: #1baf7a;
  }
  @media (prefers-color-scheme: dark) {
    :root:where(:not([data-theme="light"])) .viz-root {
      color-scheme: dark;
      --surface-1: #1a1a19;
      --text-primary: #ffffff;
      --text-secondary: #c3c2b7;
      --neutral: #4a4a46;
      --series-1: #3987e5;
      --series-2: #d95926;
      --series-3: #199e70;
    }
  }
  :root[data-theme="dark"] .viz-root {
    color-scheme: dark;
    --surface-1: #1a1a19;
    --text-primary: #ffffff;
    --text-secondary: #c3c2b7;
    --neutral: #4a4a46;
    --series-1: #3987e5;
    --series-2: #d95926;
    --series-3: #199e70;
  }

  .wall {
    margin: 0 0 1.5rem;
    padding: 0.75rem;
    border-radius: 6px;
    background: var(--surface-1);
    color: var(--text-primary);
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
  .legend {
    list-style: none;
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
    padding: 0;
    margin: 0 0 0.5rem;
    font-size: 0.85rem;
  }
  .legend li {
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
  .slot-1 {
    background: var(--series-1);
  }
  .slot-2 {
    background: var(--series-2);
  }
  .slot-3 {
    background: var(--series-3);
  }
  .slot-0 {
    background: var(--neutral);
  }

  .bar {
    display: flex;
    /* The 2px surface gap separates touching bands — a gap, never a
       border drawn around them. */
    gap: 2px;
    height: 44px;
    /* The hit target is the full band height, well past the 24px floor,
       so a one-day band in a 300-day journey is still reachable. */
    align-items: stretch;
  }
  .band {
    border: 0;
    padding: 0;
    min-width: 2px;
    border-radius: 3px;
    cursor: pointer;
  }
  /* A gap is unrecorded time: same neutral, drawn shorter so the wall
     reads as "something happened here" versus "nothing did", without
     relying on colour alone. */
  .band.gap {
    align-self: center;
    height: 26px;
  }
  .band:focus-visible {
    outline: 2px solid var(--text-primary);
    outline-offset: 2px;
  }
  .readout {
    font-size: 0.85rem;
    margin: 0.5rem 0 0;
    min-height: 1.4em;
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
    text-align: left;
    padding: 0.2rem 0.4rem;
  }
  th[scope="row"] {
    font-weight: normal;
  }
  caption {
    text-align: left;
    padding-bottom: 0.4rem;
  }
</style>
