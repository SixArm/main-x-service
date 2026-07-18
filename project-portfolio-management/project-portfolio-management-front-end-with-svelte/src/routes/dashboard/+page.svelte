<!--
  PPM dashboard (`/dashboard`, PPM-7): site tiles + per-collection
  RAG / stage rollups from the ETag-conditional `/api/at-a-glance`.
  English-first (locale catalogues extend as a follow-up).
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import { PpmClient, type Dashboard } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let board = $state<Dashboard | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      board = await ppm.dashboard();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  });
</script>

<svelte:head><title>Dashboard — PPM</title></svelte:head>

<h1>{t("ppm.dashboard.title")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if board}
  <section class="tiles" data-testid="site-tiles">
    <div class="tile"><strong>{board.site_tiles.work_items}</strong><span>{t("ppm.dashboard.workItems")}</span></div>
    <div class="tile"><strong>{board.site_tiles.proposals_open}</strong><span>{t("ppm.dashboard.openProposals")}</span></div>
    <div class="tile"><strong>{board.site_tiles.materialised_risks}</strong><span>{t("ppm.dashboard.materialisedRisks")}</span></div>
    <div class="tile"><strong>{board.site_tiles.open_risk_exposure}</strong><span>{t("ppm.dashboard.exposure")}</span></div>
    <div class="tile"><strong>{board.site_tiles.schedule_violations}</strong><span>{t("ppm.dashboard.violations")}</span></div>
    <div class="tile"><strong>{board.site_tiles.over_allocated_people}</strong><span>{t("ppm.dashboard.overAllocated")}</span></div>
  </section>

  <table>
    <thead>
      <tr>
        <th>{t("ppm.dashboard.collection")}</th><th>{t("ppm.dashboard.total")}</th>
        <th>{t("ppm.dashboard.red")}</th><th>{t("ppm.dashboard.amber")}</th><th>{t("ppm.dashboard.green")}</th>
        <th>{t("ppm.dashboard.stages")}</th>
      </tr>
    </thead>
    <tbody>
      {#each board.collections as row (row.collection)}
        <tr>
          <td><strong>{row.collection}</strong></td>
          <td>{row.total}</td>
          <td class="rag red">{row.rag.red}</td>
          <td class="rag amber">{row.rag.amber}</td>
          <td class="rag green">{row.rag.green}</td>
          <td class="small muted">
            {#each Object.entries(row.stages) as [stage, count] (stage)}
              <span class="chip">{stage}: {count}</span>
            {/each}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  <p class="small muted">{t("ppm.common.asOf")} {board.as_of}</p>
{:else if !error}
  <p>{t("ppm.common.loading")}</p>
{/if}

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
    gap: 0.75rem;
    margin: 1rem 0;
  }
  .tile {
    border: 1px solid var(--border, #ccc);
    border-radius: 8px;
    padding: 0.7rem 0.9rem;
    display: flex;
    flex-direction: column;
  }
  .tile strong { font-size: 1.5rem; }
  .tile span { font-size: 0.8rem; opacity: 0.75; }
  .rag { font-weight: 700; text-align: center; }
  .rag.red { color: #a4262c; }
  .rag.amber { color: #a06a00; }
  .rag.green { color: #1d8a4e; }
  .chip {
    display: inline-block;
    border: 1px solid var(--border, #ccc);
    border-radius: 999px;
    padding: 0 0.5rem;
    margin: 0.1rem;
    font-size: 0.75rem;
  }
</style>
