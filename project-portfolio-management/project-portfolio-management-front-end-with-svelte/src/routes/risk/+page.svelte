<!--
  Chief Risk Officer area (`/risk`): the probability×impact heatmap,
  top exposures, per-portfolio posture, concentration (server-disclosed
  threshold), review hygiene, and the declared risk appetite — or an
  honest "not configured" when no appetite is declared. All numbers
  server-derived. English-first, like the other PPM views.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import { PpmClient, type RiskHeatmap } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let heatmap = $state<RiskHeatmap | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      heatmap = await ppm.riskHeatmap();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  });

  const SCALE = [1, 2, 3, 4, 5] as const;
  const cell = (p: number, i: number) => heatmap?.cells[`p${p}i${i}`] ?? 0;
</script>

<svelte:head><title>{t("ppm.nav.risk")} — PPM</title></svelte:head>

<h1>{t("ppm.nav.risk")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if heatmap}
  <p data-testid="risk-summary">
    <strong>{heatmap.open_risks}</strong> open risks ·
    estate open exposure <strong>{heatmap.estate_open_exposure}</strong>
  </p>
  <p class="muted" data-testid="risk-appetite">
    {heatmap.appetite_note}
    {#if heatmap.appetite}
      — max estate {heatmap.appetite.max_open_exposure ?? "—"},
      max single risk {heatmap.appetite.max_item_exposure ?? "—"}
    {/if}
  </p>
  {#if heatmap.breaches.length > 0}
    <ul class="banner" data-testid="risk-breaches">
      {#each heatmap.breaches as breach, index (index)}
        <li><code>{breach.rule}</code></li>
      {/each}
    </ul>
  {/if}

  <h2>Heatmap</h2>
  <table class="matrix" data-testid="risk-matrix">
    <thead>
      <tr><th>probability ↓ / impact →</th>{#each SCALE as impact (impact)}<th>{impact}</th>{/each}</tr>
    </thead>
    <tbody>
      {#each [...SCALE].reverse() as probability (probability)}
        <tr>
          <th>{probability}</th>
          {#each SCALE as impact (impact)}
            <td class:hot={probability * impact >= 15} class:warm={probability * impact >= 8 && probability * impact < 15}>
              {cell(probability, impact) || ""}
            </td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>Top risks</h2>
  <table data-testid="risk-top">
    <thead><tr><th>Risk</th><th>Category</th><th>Exposure</th><th>Item</th></tr></thead>
    <tbody>
      {#each heatmap.top_risks as row (row.pid)}
        <tr>
          <td>{row.title}</td>
          <td>{row.category}</td>
          <td>{row.exposure}</td>
          <td>{row.item?.name ?? "—"}</td>
        </tr>
      {:else}
        <tr><td colspan="4" class="muted">No open risks.</td></tr>
      {/each}
    </tbody>
  </table>

  <h2>Posture by portfolio</h2>
  <table data-testid="risk-posture">
    <thead><tr><th>Portfolio</th><th>Open exposure</th><th>Escalated</th><th>Materialised</th></tr></thead>
    <tbody>
      {#each heatmap.posture as row (row.portfolio?.pid ?? "unassigned")}
        <tr>
          <td>{row.portfolio?.name ?? "(unassigned)"}</td>
          <td>{row.open_exposure}</td>
          <td>{row.escalated}</td>
          <td>{row.materialised}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  {#if heatmap.concentration.length > 0}
    <h2>Concentration</h2>
    <ul data-testid="risk-concentration">
      {#each heatmap.concentration as row (row.pid)}
        <li><strong>{row.title}</strong> — exposure {row.exposure} (≥ 25% of estate)</li>
      {/each}
    </ul>
  {/if}

  {#if heatmap.overdue_reviews.length > 0}
    <h2>Overdue reviews</h2>
    <ul data-testid="risk-overdue">
      {#each heatmap.overdue_reviews as row (row.pid)}
        <li>{row.title} <span class="muted">(review due {row.review_date})</span></li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .matrix td { text-align: center; min-width: 2.5rem; }
  .matrix td.warm { background: color-mix(in srgb, #b45309 25%, transparent); }
  .matrix td.hot { background: color-mix(in srgb, #b91c1c 30%, transparent); }
</style>
