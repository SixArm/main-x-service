<!--
  Time-based analysis (`/plans/{pid}/flow`) — the delivery board read
  through the lens of time rather than of activity.

  The page is ordered by what an operator can act on, not by what is
  easiest to compute: the service level expectation and today's aging
  work in progress come first, the cumulative flow diagram and the
  constraint ranking next, and the distributions last. Cycle time and
  lead time are always shown together — quoting the first as "our
  delivery time" is the commonest misreport in flow measurement, and it
  always flatters.

  Nothing here is per-person. Assignees appear on an aging item so the
  reader knows who to ask, never as a ranked comparison; see the entity
  spec's `time-based-analysis.md` §12.4 for why that is a decision
  rather than a missing feature.

  English-first, like the other PPM views.
-->
<script lang="ts">
  import { page } from "$app/state";
  import CumulativeFlow from "$lib/components/CumulativeFlow.svelte";
  import {
    TbaClient,
    days,
    flowEfficiencyBand,
    interpretationLabel,
    msAsDays,
    percent,
    type AgingWip,
    type Constraints,
    type CumulativeFlow as CumulativeFlowData,
    type PlanFlow,
    type PlanTimeAnalysis,
  } from "$lib/api/tba";

  const pid = page.params.pid ?? "";
  const tba = TbaClient.withFetch();

  let analysis = $state<PlanTimeAnalysis | null>(null);
  let aging = $state<AgingWip | null>(null);
  let constraints = $state<Constraints | null>(null);
  let flow = $state<PlanFlow | null>(null);
  let cfd = $state<CumulativeFlowData | null>(null);
  let windowDays = $state(60);
  let error = $state<string | null>(null);

  async function load() {
    error = null;
    try {
      [analysis, aging, constraints, flow, cfd] = await Promise.all([
        tba.planTimeAnalysis(pid),
        tba.agingWip(pid),
        tba.constraints(pid),
        tba.flow(pid, windowDays),
        tba.cumulativeFlow(pid, windowDays),
      ]);
    } catch (err) {
      error = err instanceof Error ? err.message : "Could not load flow data.";
    }
  }
  $effect(() => {
    void load();
  });

  const sle = $derived(analysis?.service_level_expectation ?? null);
  const summary = $derived(analysis?.plan_analysis ?? null);
  const efficiency = $derived(summary?.aggregate_flow_efficiency.value ?? null);
  const band = $derived(flowEfficiencyBand(efficiency));

  /** Items already past the expectation — the ones worth a conversation today. */
  const overdue = $derived((aging?.aging ?? []).filter((row) => row.aging.past_sle));
</script>

<svelte:head><title>Flow — PPM</title></svelte:head>

<h1>Time-based analysis</h1>
<p class="muted">
  Of the calendar time these items took, how much of it was somebody
  working on them? Flow efficiency in knowledge work typically measures
  5–15%, so a low figure is normal — the queue is the finding, not the
  effort.
</p>

{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if analysis && summary}
  <!-- The headline row: an expectation you can quote, and the ratio
       behind it. Stat tiles rather than charts — each is one number. -->
  <div class="tiles">
    <div class="tile" data-testid="sle-badge">
      <span class="tile-label">Service level expectation</span>
      {#if sle?.within_days !== null && sle?.within_days !== undefined}
        <span class="tile-value">{days(sle.within_days)}</span>
        <span class="tile-note">
          {percent(sle.percentile)} of items finish within this, from
          {sle.sample} finished item{sle.sample === 1 ? "" : "s"}
        </span>
      {:else}
        <span class="tile-value muted">—</span>
        <span class="tile-note">{sle?.reason ?? "Not enough finished items yet."}</span>
      {/if}
    </div>

    <div class="tile" data-testid="flow-efficiency">
      <span class="tile-label">Flow efficiency</span>
      <span class="tile-value">{percent(efficiency, 1)}</span>
      <span class="tile-note">
        {#if band === "suspicious"}
          Unusually high — this usually means the board is not being kept
          up to date rather than that the queue has gone.
        {:else if band === "strong"}
          Strong against the 5–15% norm.
        {:else if band === "typical"}
          Typical. Median item: {percent(summary.median_flow_efficiency, 1)}.
        {:else}
          No finished work to measure yet.
        {/if}
      </span>
    </div>

    <div class="tile">
      <span class="tile-label">First pass yield</span>
      <span class="tile-value">{percent(summary.rolled_first_pass_yield)}</span>
      <span class="tile-note">
        finished without ever moving backwards ({summary.rework_count}
        backwards move{summary.rework_count === 1 ? "" : "s"}). Read this
        beside throughput: rising throughput with falling yield is work
        being shipped back to itself.
      </span>
    </div>

    <div class="tile">
      <span class="tile-label">Work in progress</span>
      <span class="tile-value">{summary.work_in_progress}</span>
      <span class="tile-note">
        {summary.not_started} not started · {summary.finished} finished of
        {summary.tasks}
      </span>
    </div>
  </div>

  {#if summary.waste_shape === "concentrated"}
    <p class="finding">
      The waste is <strong>concentrated</strong>: the plan's overall ratio
      ({percent(efficiency, 1)}) is well away from the typical item's
      ({percent(summary.median_flow_efficiency, 1)}). Most items flow;
      a minority are stuck. That is a different fix from uniformly slow
      delivery — find those items rather than reworking the process.
    </p>
  {/if}

  {#if summary.backfilled_ratio !== null && summary.backfilled_ratio > 0}
    <p class="finding muted">
      {percent(summary.backfilled_ratio)} of the transitions behind these
      figures were synthesised when the log was introduced, not observed.
      They firm up as real board moves accumulate.
    </p>
  {/if}
{/if}

<!-- Aging WIP: the only view here about work that can still be helped. -->
<h2>Aging work in progress</h2>
{#if aging}
  <p class="muted">{aging.note}</p>
  {#if aging.aging.length === 0}
    <p class="muted">Nothing in progress.</p>
  {:else}
    <table data-testid="aging-wip">
      <thead>
        <tr>
          <th scope="col">Item</th>
          <th scope="col">Status</th>
          <th scope="col">Age</th>
          <th scope="col">vs expectation</th>
          <th scope="col">Blocked</th>
          <th scope="col">Rework</th>
          <th scope="col">Assignee</th>
        </tr>
      </thead>
      <tbody>
        {#each aging.aging as row (row.task.pid)}
          <tr class:past={row.aging.past_sle}>
            <th scope="row">
              <a href={`/plans/${pid}/board`}>{row.task.title}</a>
            </th>
            <td>{row.status}</td>
            <td>{days(row.aging.age_days)}</td>
            <td>
              {#if row.aging.sle_ratio === null}
                <span class="muted">no expectation yet</span>
              {:else}
                {row.aging.past_sle ? "⚠ " : ""}{percent(row.aging.sle_ratio)}
              {/if}
            </td>
            <td>{msAsDays(row.blocked_time_ms)}</td>
            <td>{row.rework_count}</td>
            <td class="muted">{row.task.assignee_ref ?? "unassigned"}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if overdue.length > 0}
      <p class="finding">
        {overdue.length} item{overdue.length === 1 ? " is" : "s are"} already
        past the expectation. An aging item is the one thing on this page
        you can still change the outcome of.
      </p>
    {/if}
  {/if}
{/if}

<!-- The picture. -->
<h2>Cumulative flow</h2>
<label class="window">
  Window
  <select
    value={String(windowDays)}
    onchange={(event) => {
      windowDays = Number.parseInt(event.currentTarget.value, 10);
      void load();
    }}
  >
    <option value="30">30 days</option>
    <option value="60">60 days</option>
    <option value="90">90 days</option>
    <option value="180">180 days</option>
  </select>
</label>
{#if cfd}
  <CumulativeFlow samples={cfd.samples} note={cfd.note} />
{/if}

<!-- Where the time goes. -->
<h2>Constraints</h2>
{#if constraints}
  <p class="muted">{constraints.note}</p>
  <table data-testid="constraints">
    <thead>
      <tr>
        <th scope="col">Rule</th>
        <th scope="col">Subject</th>
        <th scope="col">Recoverable</th>
        <th scope="col">Detail</th>
      </tr>
    </thead>
    <tbody>
      {#each constraints.findings as finding (finding.rule + finding.subject)}
        <tr>
          <th scope="row"><code>{finding.rule}</code></th>
          <td>{finding.subject}</td>
          <td>{days(finding.recoverable_days)}</td>
          <td class="detail">{finding.detail}</td>
        </tr>
      {:else}
        <tr><td colspan="4" class="muted">No findings yet.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

<!-- Little's Law, and the columns it points at. -->
<h2>Flow</h2>
{#if flow}
  <p class="finding" data-testid="littles-law">
    <strong>{interpretationLabel(flow.flow.interpretation)}.</strong>
    {flow.flow.detail}
  </p>
  <table data-testid="flow-rates">
    <tbody>
      <tr>
        <th scope="row">Arrival rate (λ)</th>
        <td>{flow.flow.arrival_rate_per_day?.toFixed(2) ?? "—"} / day</td>
        <th scope="row">Throughput (μ)</th>
        <td>{flow.flow.throughput_per_day?.toFixed(2) ?? "—"} / day</td>
      </tr>
      <tr>
        <th scope="row">Utilisation (ρ)</th>
        <td>
          {flow.flow.utilisation?.toFixed(2) ?? "—"}
          {#if flow.flow.utilisation_reason}
            <span class="muted">— {flow.flow.utilisation_reason}</span>
          {/if}
        </td>
        <th scope="row">Work in progress (κ)</th>
        <td>{flow.flow.work_in_progress}</td>
      </tr>
      <tr>
        <th scope="row">Implied cycle time</th>
        <td>{days(flow.flow.implied_cycle_time_days)}</td>
        <th scope="row">Observed median</th>
        <td>{days(flow.flow.observed_p50_cycle_time_days)}</td>
      </tr>
    </tbody>
  </table>

  <h3>Columns</h3>
  <p class="muted">
    Cycle time is work in progress divided by throughput, so lowering a
    cap shortens it without anyone working faster.
  </p>
  <table data-testid="columns">
    <thead>
      <tr><th scope="col">Column</th><th scope="col">Count</th><th scope="col">Limit</th></tr>
    </thead>
    <tbody>
      {#each flow.columns as column (column.status)}
        <tr class:past={column.over_limit}>
          <th scope="row">{column.status}</th>
          <td>{column.count}</td>
          <td>
            {#if column.limit === null}
              <span class="muted">no cap</span>
            {:else}
              {column.over_limit ? "⚠ " : ""}{column.limit}
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<!-- The distributions last: history, not something to act on today. -->
{#if summary?.cycle_time && summary?.lead_time}
  <h2>Distributions</h2>
  <p class="muted">
    Percentiles are nearest-rank, so every one is a real item. Cycle time
    is what the team controls; lead time is what the requester waits, and
    the difference between them is how long work sat in the backlog.
  </p>
  <table data-testid="distributions">
    <thead>
      <tr>
        <th scope="col">Measure</th><th scope="col">n</th><th scope="col">min</th>
        <th scope="col">p50</th><th scope="col">p85</th><th scope="col">p95</th>
        <th scope="col">max</th><th scope="col">mean</th>
      </tr>
    </thead>
    <tbody>
      {#each [{ label: "Cycle time", d: summary.cycle_time }, { label: "Lead time", d: summary.lead_time }] as row (row.label)}
        <tr>
          <th scope="row">{row.label}</th>
          <td>{row.d.n}</td>
          <td>{msAsDays(row.d.min_ms)}</td>
          <td>{msAsDays(row.d.p50_ms)}</td>
          <td>{msAsDays(row.d.p85_ms)}</td>
          <td>{msAsDays(row.d.p95_ms)}</td>
          <td>{msAsDays(row.d.max_ms)}</td>
          <td class="muted">{msAsDays(row.d.mean_ms)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  <p class="muted">
    The mean is shown for completeness and describes no actual item —
    quote the p85.
  </p>
{/if}

{#if analysis}
  <p class="muted classes">
    Classified with {analysis.classification.overridden
      ? "a deployment override"
      : "the default map"}:
    {#each Object.entries(analysis.classification.classes) as [status, category], i (status)}{i >
      0
        ? " · "
        : ""}{status} → {category}{/each}
  </p>
{/if}

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
    gap: 1rem;
    margin: 1rem 0;
  }
  .tile {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    border: 1px solid rgb(128 128 128 / 0.25);
  }
  .tile-label {
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.75;
  }
  .tile-value {
    font-size: 2rem;
    line-height: 1.1;
  }
  .tile-note {
    font-size: 0.8rem;
    opacity: 0.8;
  }
  .finding {
    margin: 0.75rem 0;
    padding: 0.6rem 0.8rem;
    border-left: 3px solid rgb(128 128 128 / 0.4);
    font-size: 0.9rem;
  }
  .window {
    display: inline-flex;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.5rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9rem;
    margin-bottom: 1rem;
  }
  th,
  td {
    text-align: left;
    padding: 0.3rem 0.5rem;
    vertical-align: top;
  }
  td {
    font-variant-numeric: tabular-nums;
  }
  tr.past th[scope="row"] {
    font-weight: 700;
  }
  .detail {
    font-size: 0.85rem;
    opacity: 0.85;
  }
  .muted {
    opacity: 0.75;
    font-size: 0.9rem;
  }
  .classes {
    font-size: 0.8rem;
  }
</style>
