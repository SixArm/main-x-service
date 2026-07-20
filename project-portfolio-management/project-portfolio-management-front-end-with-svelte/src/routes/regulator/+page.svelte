<!--
  Regulator area (`/regulator`): the deliberately coarse extract —
  portfolio-level aggregates only (no person references, no item-level
  budgets; the server states this and may withhold names under an ABAC
  `mask` obligation). English-first, like the other PPM views.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import { PpmClient, money, type RegulatorExtract } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let extract = $state<RegulatorExtract | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      extract = await ppm.regulatorExtract();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  });
</script>

<svelte:head><title>{t("ppm.nav.regulator")} — PPM</title></svelte:head>

<h1>{t("ppm.nav.regulator")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if extract}
  <p class="muted">{extract.note}</p>
  {#if extract.masked}
    <p class="banner" data-testid="regulator-masked">
      Names withheld (ABAC mask obligation).
    </p>
  {/if}
  {#each extract.portfolios as portfolio (portfolio.pid)}
    <section data-testid="regulator-portfolio">
      <h2>{portfolio.name ?? portfolio.pid}</h2>
      <p class="muted">stage: {portfolio.stage ?? "pre_gate"}</p>
      <p>
        Members:
        {#each Object.entries(portfolio.members) as [kind, count] (kind)}
          {count} {kind}&nbsp;
        {/each}
        · Gate decisions:
        {#each Object.entries(portfolio.gate_decisions) as [decision, count] (decision)}
          {count} {decision}&nbsp;
        {:else}
          <span class="muted">none</span>
        {/each}
      </p>
      <table>
        <thead><tr><th>Currency</th><th>Planned</th><th>Actual</th></tr></thead>
        <tbody>
          {#each portfolio.spend as row (row.currency)}
            <tr>
              <td>{row.currency}</td>
              <td>{money(row.planned_minor, row.currency)}</td>
              <td>{money(row.actual_minor, row.currency)}</td>
            </tr>
          {:else}
            <tr><td colspan="3" class="muted">No budget lines.</td></tr>
          {/each}
        </tbody>
      </table>
      {#if portfolio.benefits.length > 0}
        <p>
          Benefits:
          {#each portfolio.benefits as row (row.currency)}
            target {money(row.target_minor, row.currency)} /
            realized {money(row.realized_minor, row.currency)}&nbsp;
          {/each}
        </p>
      {/if}
    </section>
  {:else}
    <p class="muted">No portfolios recorded.</p>
  {/each}
{/if}
