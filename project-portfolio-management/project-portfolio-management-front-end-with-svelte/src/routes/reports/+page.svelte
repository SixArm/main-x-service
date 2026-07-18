<!--
  Saved reports (`/reports`, PPM-9): definitions (collection +
  filters + projected fields), synchronous runs previewed as a
  table, and the CSV download.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { PpmClient, type ReportDefinition, type ReportRun } from "$lib/api/ppm";
  import { COLLECTIONS, type Collection } from "$lib/api/types";

  const ppm = PpmClient.withFetch();
  let reports = $state<ReportDefinition[]>([]);
  let runs = $state<Record<string, ReportRun>>({});
  let error = $state<string | null>(null);

  let name = $state("");
  let collection = $state<Collection>("projects");
  let nameLike = $state("");
  let fields = $state("name,stage,status,target_date");

  async function refresh() {
    try {
      reports = await ppm.listReports();
      error = null;
    } catch (err) {
      error = err instanceof Error ? err.message : "load failed";
    }
  }
  onMount(refresh);

  async function create(event: SubmitEvent) {
    event.preventDefault();
    try {
      await ppm.createReport({
        name,
        collection,
        filters: nameLike.trim() ? { name_like: nameLike.trim() } : {},
        fields: fields.split(",").map((field) => field.trim()).filter(Boolean),
      });
      name = "";
      nameLike = "";
      await refresh();
    } catch (err) {
      error = err instanceof Error ? err.message : "save failed";
    }
  }

  async function run(pid: string) {
    try {
      runs = { ...runs, [pid]: await ppm.runReport(pid) };
    } catch (err) {
      error = err instanceof Error ? err.message : "run failed";
    }
  }
</script>

<svelte:head><title>Reports — PPM</title></svelte:head>

<h1>Reports</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

<form class="row" onsubmit={create}>
  <input placeholder="Report name" bind:value={name} required />
  <select bind:value={collection} aria-label="Collection">
    {#each COLLECTIONS as segment (segment)}<option value={segment}>{segment}</option>{/each}
  </select>
  <input placeholder="name contains…" bind:value={nameLike} />
  <input placeholder="fields (comma separated)" bind:value={fields} size="34" />
  <button class="button primary" type="submit">Save</button>
</form>

{#each reports as report (report.pid)}
  <section class="report">
    <p>
      <strong>{report.name}</strong>
      <span class="chip">{report.collection}</span>
      <button class="button small" onclick={() => run(report.pid)}>Run</button>
      <a class="button small" href={ppm.reportCsvUrl(report.pid)} download>CSV</a>
    </p>
    {#if runs[report.pid]}
      {@const result = runs[report.pid]}
      {#if result}
        <p class="small muted">{result.rows} rows</p>
        {#if result.data.length > 0}
          {@const first = result.data[0] ?? {}}
          <table class="small">
            <thead>
              <tr>{#each Object.keys(first) as column (column)}<th>{column}</th>{/each}</tr>
            </thead>
            <tbody>
              {#each result.data as row, index (index)}
                <tr>{#each Object.values(row) as cell, cellIndex (cellIndex)}<td>{cell}</td>{/each}</tr>
              {/each}
            </tbody>
          </table>
        {/if}
      {/if}
    {/if}
  </section>
{/each}

<style>
  .row { display: flex; gap: 0.5rem; flex-wrap: wrap; align-items: center; margin: 0.8rem 0; }
  .report { border-bottom: 1px solid var(--border, #ddd); padding: 0.5rem 0; }
  .chip {
    display: inline-block;
    border: 1px solid var(--border, #ccc);
    border-radius: 999px;
    padding: 0 0.5rem;
    margin: 0 0.3rem;
    font-size: 0.78rem;
  }
</style>
