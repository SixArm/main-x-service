<!--
  Time-based analysis (`/time`) — a pathway's cohort measured against
  elapsed calendar time, and one journey drawn to scale.

  The page answers Barker's question in two registers. The **cohort**
  half asks it of a whole pathway: what share of the time patients spend
  on it is care, how the lead times distribute, how they score against a
  named NHS access standard, and where the recoverable time sits. The
  **journey** half asks it of one enrolment, on the timeline wall.

  Instances are not listable across pathways, so — like `/board`,
  `/gantt` and `/sequence` — the page is built from a single selected
  pathway seeded from the registry list.

  A cohort of fewer than five instances has its percentile detail
  withheld by the service, because percentiles over a handful of
  patients identify them. The page shows that as a stated suppression
  rather than as missing data.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import JourneyTimeline from "$lib/components/JourneyTimeline.svelte";
  import ProcessMapView from "$lib/components/ProcessMapView.svelte";
  import VariantsSunburst from "$lib/components/VariantsSunburst.svelte";
  import VariantsSankey from "$lib/components/VariantsSankey.svelte";
  import AttritionFlowchart from "$lib/components/AttritionFlowchart.svelte";
  import CompareView from "$lib/components/CompareView.svelte";
  import StalledList from "$lib/components/StalledList.svelte";
  import DottedChart from "$lib/components/DottedChart.svelte";
  import { CarePathwayRepository } from "$lib/api/care-pathways";
  import type { PathwayRef, PathwayInstance } from "$lib/api/types";
  import {
    TbaRepository,
    confidenceNote,
    days,
    interpretationLabel,
    msAsDays,
    percent,
    valueAddingBand,
    type CohortTimeAnalysis,
    type Constraints,
    type EventLogRow,
    type Flow,
    type InstanceTimeAnalysis,
    type ProcessMapResponse,
    type SplitSideFindings,
    type Stalled,
    type Standards,
    type Timeline,
    type VariantsReport,
  } from "$lib/api/tba";
  import { t } from "$lib/i18n.svelte";

  const registry = CarePathwayRepository.withFetch();
  const tba = TbaRepository.withFetch();

  let pathways = $state<PathwayRef[]>([]);
  let selectedPathway = $state("");
  let instances = $state<PathwayInstance[]>([]);
  let selectedInstance = $state("");
  let standard = $state("");

  // T-14f rule-split filter, shared by the cohort and constraints
  // fetches below.
  let containsFilter = $state("");
  let excludesFilter = $state("");
  let compareEnabled = $state(false);

  let standards = $state<Standards | null>(null);
  let cohort = $state<CohortTimeAnalysis | null>(null);
  let constraints = $state<Constraints | null>(null);
  let flow = $state<Flow | null>(null);
  let timeline = $state<Timeline | null>(null);
  let analysis = $state<InstanceTimeAnalysis | null>(null);
  let processMap = $state<ProcessMapResponse | null>(null);
  let variants = $state<VariantsReport | null>(null);
  let stalled = $state<Stalled | null>(null);
  let eventLogRows = $state<EventLogRow[] | null>(null);
  let dottedChartLoading = $state(false);
  let error = $state<string | null>(null);

  function fail(err: unknown) {
    error = err instanceof Error ? err.message : "Could not load.";
  }

  onMount(async () => {
    try {
      [pathways, standards, stalled] = await Promise.all([
        registry.list(),
        tba.standards(),
        tba.stalled(),
      ]);
      if (pathways.length > 0) {
        selectedPathway = pathways[0]?.pid ?? "";
        await loadPathway();
      }
    } catch (err) {
      fail(err);
    }
  });

  async function loadPathway() {
    if (!selectedPathway) return;
    error = null;
    timeline = null;
    analysis = null;
    eventLogRows = null;
    selectedInstance = "";
    const split = {
      contains: containsFilter.trim() || undefined,
      excludes: excludesFilter.trim() || undefined,
      compare: compareEnabled,
    };
    try {
      [cohort, constraints, flow, instances, processMap, variants] =
        await Promise.all([
          tba.cohort(selectedPathway, {
            ...(standard ? { standard } : {}),
            ...split,
          }),
          tba.constraints(selectedPathway, undefined, split),
          tba.flow(90, selectedPathway),
          registry.listInstances(selectedPathway),
          tba.processMap(selectedPathway),
          tba.variants(selectedPathway),
        ]);
      if (instances.length > 0) {
        selectedInstance = instances[0]?.pid ?? "";
        await loadInstance();
      }
    } catch (err) {
      fail(err);
    }
  }

  async function loadDottedChart() {
    if (!selectedPathway) return;
    dottedChartLoading = true;
    error = null;
    try {
      eventLogRows = await tba.eventLog(selectedPathway);
    } catch (err) {
      fail(err);
    } finally {
      dottedChartLoading = false;
    }
  }

  async function loadInstance() {
    if (!selectedInstance) return;
    error = null;
    try {
      [timeline, analysis] = await Promise.all([
        tba.timeline(selectedInstance),
        tba.instanceAnalysis(selectedInstance),
      ]);
    } catch (err) {
      fail(err);
    }
  }

  const ratio = $derived(
    cohort?.cohort.aggregate_value_adding_ratio.value ?? null,
  );

  /**
   * The standard's human label. The compliance response carries only
   * the id (`rtt_18_weeks`), which a tile heading would render as
   * shouty machine text; the catalogue has the real name.
   */
  const standardLabel = $derived(
    standards?.standards.find(
      (entry) => entry.id === cohort?.compliance?.standard,
    )?.label ??
      cohort?.compliance?.standard ??
      "",
  );
  const journey = $derived(analysis?.analysis ?? null);
  const band = $derived(
    valueAddingBand(
      journey?.value_adding_ratio.value ?? null,
      journey?.confidence ?? "unmapped",
    ),
  );
</script>

<svelte:head><title>{t("nav.time")} — {t("brand.name")}</title></svelte:head>

<h1>Time-based analysis</h1>
<p class="muted">
  Of the calendar time a patient spends on this pathway, how much of it is care?
  Tracked NHS journeys measure 8–14% — the rest is waiting, handoffs, and time
  in which nothing happens. A low figure is what the method predicts, not a
  fault to explain away.
</p>

{#if error}<p class="banner" role="alert">{error}</p>{/if}

<form class="row" onsubmit={(event) => event.preventDefault()}>
  <label>
    Pathway
    <select
      bind:value={selectedPathway}
      onchange={() => void loadPathway()}
      aria-label="Pathway"
    >
      {#each pathways as pathway (pathway.pid)}
        <option value={pathway.pid}>{pathway.name}</option>
      {/each}
    </select>
  </label>
  <label>
    Access standard
    <select
      bind:value={standard}
      onchange={() => void loadPathway()}
      aria-label="Access standard"
    >
      <option value="">(none)</option>
      {#each standards?.standards ?? [] as entry (entry.id)}
        <option value={entry.id}>{entry.label}</option>
      {/each}
    </select>
  </label>
</form>

<form
  class="row"
  onsubmit={(event) => {
    event.preventDefault();
    void loadPathway();
  }}
>
  <label>
    {t("time.contains")}
    <input
      type="text"
      bind:value={containsFilter}
      placeholder="stage:triage,urgency:routine"
      aria-label={t("time.contains")}
    />
  </label>
  <label>
    {t("time.excludes")}
    <input
      type="text"
      bind:value={excludesFilter}
      placeholder="outcome:deceased"
      aria-label={t("time.excludes")}
    />
  </label>
  <label>
    <input type="checkbox" bind:checked={compareEnabled} />
    {t("time.compareComplement")}
  </label>
  <button type="submit">{t("time.applyFilter")}</button>
</form>

{#if cohort}
  <h2>The cohort</h2>
  <div class="tiles">
    <div class="tile" data-testid="cohort-ratio">
      <span class="tile-label">Value-adding time</span>
      <span class="tile-value">{percent(ratio, 1)}</span>
      <span class="tile-note">
        of the cohort's elapsed time. Typical journey:
        {percent(cohort.cohort.median_value_adding_ratio, 1)}.
      </span>
    </div>
    <div class="tile">
      <span class="tile-label">Journeys</span>
      <span class="tile-value">{cohort.cohort.instances}</span>
      <span class="tile-note">
        coverage {percent(cohort.cohort.coverage_ratio.value)} — how much of them
        is mapped at all
      </span>
    </div>
    {#if cohort.cohort.lead_time}
      <div class="tile" data-testid="lead-time">
        <span class="tile-label">Lead time (median)</span>
        <span class="tile-value">{days(cohort.cohort.lead_time.p50_days)}</span>
        <span class="tile-note">
          p90 {days(cohort.cohort.lead_time.p90_days)} · nearest-rank, so every percentile
          is a real patient
        </span>
      </div>
    {/if}
    {#if cohort.compliance}
      <div class="tile" data-testid="compliance">
        <span class="tile-label">{standardLabel}</span>
        <span class="tile-value">
          {percent(cohort.compliance.achieved_ratio)}
        </span>
        <span class="tile-note">
          within {days(cohort.compliance.threshold_days, 0)}
          {#if cohort.compliance.target_ratio !== null}
            · target {percent(cohort.compliance.target_ratio)}
            {cohort.compliance.target_met ? "— met" : "— not met"}
          {/if}
          {#if cohort.compliance.as_of}
            <br />threshold checked {cohort.compliance.as_of}
          {/if}
        </span>
      </div>
    {/if}
  </div>

  {#if cohort.suppressed}
    <p class="finding" data-testid="suppressed">
      {cohort.suppression_note}
    </p>
  {/if}

  {#if cohort.cohort.waste_shape === "concentrated"}
    <p class="finding">
      The waste is <strong>concentrated</strong>: the cohort's overall ratio ({percent(
        ratio,
        1,
      )}) is well away from the typical journey's ({percent(
        cohort.cohort.median_value_adding_ratio,
        1,
      )}). Most patients flow; a minority are stuck. Find those journeys rather
      than redesigning the pathway.
    </p>
  {/if}

  {#if cohort.split}
    <h3>{t("time.compare")}</h3>
    <CompareView split={cohort.split} />
  {/if}

  <h3>{t("time.attritionHeading")}</h3>
  <AttritionFlowchart steps={cohort.attrition} />
{/if}

{#snippet findingsSide(label: string, side: SplitSideFindings | null)}
  <div class="compare-column">
    <h4>{label}</h4>
    {#if side === null}
      <p class="muted">Not requested — add <code>compare=true</code>.</p>
    {:else if side.suppressed}
      <p class="finding">{side.suppression_note ?? "withheld (n < 5)"}</p>
      <p class="muted">{side.instances} instances</p>
    {:else}
      <p class="muted">
        {side.instances} instances · {side.findings?.length ?? 0} finding(s)
      </p>
    {/if}
  </div>
{/snippet}

{#if constraints && constraints.findings.length > 0}
  <h2>Where the time goes</h2>
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
      {/each}
    </tbody>
  </table>
  {#if constraints.split}
    <h3>{t("time.compare")}</h3>
    <p class="muted">
      Matching
      {#if constraints.split.rule.contains.length > 0}<code
          >contains: {constraints.split.rule.contains.join(", ")}</code
        >{/if}
      {#if constraints.split.rule.excludes.length > 0}<code
          >excludes: {constraints.split.rule.excludes.join(", ")}</code
        >{/if}
    </p>
    <div class="columns" data-testid="constraints-compare">
      {@render findingsSide(t("time.matched"), constraints.split.matched)}
      {@render findingsSide(t("time.complement"), constraints.split.complement)}
    </div>
  {/if}
{/if}

{#if flow}
  <h2>Flow</h2>
  <p class="finding" data-testid="littles-law">
    <strong>{interpretationLabel(flow.flow.interpretation)}.</strong>
    {flow.flow.detail}
  </p>
  <table data-testid="flow-rates">
    <tbody>
      <tr>
        <th scope="row">Enrolments (λ)</th>
        <td>{flow.flow.arrival_rate_per_day?.toFixed(2) ?? "—"} / day</td>
        <th scope="row">Closures (μ)</th>
        <td>{flow.flow.service_rate_per_day?.toFixed(2) ?? "—"} / day</td>
      </tr>
      <tr>
        <th scope="row">Utilisation (ρ)</th>
        <td>
          {flow.flow.utilisation?.toFixed(2) ?? "—"}
          {#if flow.flow.utilisation_reason}
            <span class="muted">— {flow.flow.utilisation_reason}</span>
          {/if}
        </td>
        <th scope="row">Open journeys (κ)</th>
        <td>{flow.flow.work_in_progress}</td>
      </tr>
    </tbody>
  </table>
{/if}

{#if processMap}
  <h2>{t("time.processMap")}</h2>
  <p class="muted">{processMap.note}</p>
  <ProcessMapView map={processMap} />
{/if}

{#if variants}
  <h2>{t("time.variants")}</h2>
  <p class="muted">{variants.note}</p>
  <div class="variant-views">
    <div>
      <h3>{t("time.sunburst")}</h3>
      <VariantsSunburst report={variants} />
    </div>
    <div>
      <h3>{t("time.sankey")}</h3>
      <VariantsSankey report={variants} />
    </div>
  </div>
  <table data-testid="variant-lines">
    <thead>
      <tr>
        <th scope="col">Position</th>
        <th scope="col">n</th>
        <th scope="col">Median</th>
        <th scope="col">p90</th>
      </tr>
    </thead>
    <tbody>
      {#each variants.lines as line (line.position)}
        <tr>
          <th scope="row">{line.position}</th>
          <td>{line.n}</td>
          <td>{days(line.median_days)}</td>
          <td>{days(line.p90_days)}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h3>{t("time.dottedChart")}</h3>
  {#if eventLogRows === null}
    <p class="muted">
      One row per instance, one dot per activity — this loads the full
      case-level event log, which the service audits as a disclosure, so it is
      not fetched until you ask for it.
    </p>
    <button
      type="button"
      onclick={() => void loadDottedChart()}
      disabled={dottedChartLoading}
    >
      {dottedChartLoading ? t("time.loading") : t("time.loadDottedChart")}
    </button>
  {:else}
    <DottedChart rows={eventLogRows} />
  {/if}
{/if}

{#if stalled}
  <h2>{t("time.stalledJourneys")}</h2>
  <p class="muted">
    Open instances idle past {stalled.idle_days} days, most idle first — complements
    "overdue reviews" (a due date) with an observed-silence test. Idle-since is the
    last activity time, never when the silence was noticed.
  </p>
  <StalledList {stalled} />
{/if}

<h2>One journey</h2>
{#if instances.length === 0}
  <p class="muted">No enrolments on this pathway yet.</p>
{:else}
  <form class="row" onsubmit={(event) => event.preventDefault()}>
    <label>
      Journey
      <select
        bind:value={selectedInstance}
        onchange={() => void loadInstance()}
        aria-label="Journey"
      >
        {#each instances as instance (instance.pid)}
          <option value={instance.pid}>
            {instance.subject_ref} · {instance.status}
          </option>
        {/each}
      </select>
    </label>
  </form>

  {#if journey && timeline}
    <div class="tiles">
      <div class="tile" data-testid="journey-ratio">
        <span class="tile-label">Value-adding time</span>
        <span class="tile-value"
          >{percent(journey.value_adding_ratio.value, 1)}</span
        >
        <span class="tile-note">
          {#if band === "suspicious"}
            Implausibly high — check the journey is fully mapped before reading
            this as efficiency.
          {:else if band === "better"}
            Above the 8–14% tracked norm.
          {:else if band === "typical"}
            In the 8–14% range tracked NHS journeys measure.
          {:else}
            Not measurable yet.
          {/if}
        </span>
      </div>
      <div class="tile" data-testid="confidence">
        <span class="tile-label">Coverage</span>
        <span class="tile-value">{percent(journey.coverage_ratio.value)}</span>
        <span class="tile-note">{confidenceNote(journey.confidence)}</span>
      </div>
      <div class="tile">
        <span class="tile-label">Lead time</span>
        <span class="tile-value">{days(journey.lead_time_days)}</span>
        <span class="tile-note">
          care {msAsDays(journey.value_time_ms)} · waiting
          {msAsDays(journey.wait_time_ms)}
        </span>
      </div>
      <div class="tile">
        <span class="tile-label">Handoffs</span>
        <span class="tile-value">{journey.handoffs.total}</span>
        <span class="tile-note">
          {journey.handoffs.distinct_actors} clinicians ·
          {journey.handoffs.distinct_locations} locations ·
          {msAsDays(journey.handoffs.gap_ms_at_handoffs)} waiting at the boundaries
        </span>
      </div>
    </div>

    <JourneyTimeline
      wall={timeline.wall}
      clock={timeline.clock}
      note={timeline.note}
    />

    {#if journey.gaps.length > 0}
      <h3>The longest queues</h3>
      <table data-testid="gaps">
        <thead>
          <tr>
            <th scope="col">Between</th>
            <th scope="col">Waiting for</th>
            <th scope="col">Days</th>
            <th scope="col">At a handoff</th>
          </tr>
        </thead>
        <tbody>
          {#each journey.gaps.slice(0, 5) as gap (gap.start_ms)}
            <tr>
              <th scope="row">
                {gap.after ?? "clock start"} → {gap.before ?? "clock stop"}
              </th>
              <td>{gap.stage ?? "—"}</td>
              <td>{gap.days.toFixed(1)}</td>
              <td>{gap.at_handoff ? "yes" : "no"}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="muted">
        A gap is clock time no segment covers. Named by what it sits between, it
        is a queue somebody can go and look at.
      </p>
    {/if}
  {/if}
{/if}

<style>
  .row {
    display: flex;
    gap: 1rem;
    align-items: end;
    flex-wrap: wrap;
    margin-bottom: 1rem;
  }
  .row label {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }
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
  .detail {
    font-size: 0.85rem;
    opacity: 0.85;
  }
  .muted {
    opacity: 0.75;
    font-size: 0.9rem;
  }
  .columns {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
    gap: 1rem;
    margin: 1rem 0;
  }
  .compare-column {
    border: 1px solid rgb(128 128 128 / 0.25);
    border-radius: 6px;
    padding: 0.75rem 1rem;
  }
  .compare-column h4 {
    margin: 0 0 0.5rem;
  }
  .variant-views {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(18rem, 1fr));
    gap: 1.5rem;
    align-items: start;
  }
</style>
