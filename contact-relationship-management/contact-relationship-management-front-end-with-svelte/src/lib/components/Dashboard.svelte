<script lang="ts">
  import { forecast, money, salesDashboard, slaDashboard } from "$lib/api/crm";
  import { i18n, t } from "$lib/i18n.svelte";

  let sales = $state<Awaited<ReturnType<typeof salesDashboard>> | null>(null);
  let sla = $state<Awaited<ReturnType<typeof slaDashboard>> | null>(null);
  let totals = $state<Record<string, number>>({});
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        const [salesData, slaData, forecastData] = await Promise.all([
          salesDashboard(),
          slaDashboard(),
          forecast(),
        ]);
        sales = salesData;
        sla = slaData;
        totals = forecastData.totals_minor;
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  const winRateLabel = $derived(
    sales === null
      ? ""
      : sales.win_rate.value === null
        ? t("dash.noData")
        : `${Math.round(sales.win_rate.value * 100)}% (${sales.win_rate.numerator}/${sales.win_rate.denominator})`,
  );
</script>

<h1>{t("dash.title")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if sales === null || sla === null}
  <p>{t("common.loading")}</p>
{:else}
  <div class="tiles">
    <div class="tile" data-testid="tile-winrate">
      <strong>{winRateLabel}</strong>
      <span>{t("dash.winRate")}</span>
    </div>
    <a class="tile" href="/deals" data-testid="tile-deals">
      <strong>{sales.open_deals}</strong>
      <span>{t("dash.openDeals")}</span>
    </a>
    <a class="tile" href="/tickets" data-testid="tile-tickets">
      <strong>{sla.open_tickets}</strong>
      <span>{t("dash.openTickets")}</span>
    </a>
    <div class="tile" data-testid="tile-forecast">
      <strong>
        {#each Object.entries(totals) as [currency, minor] (currency)}
          <div>{money(minor, currency, i18n.locale)}</div>
        {:else}
          <div>{t("dash.noData")}</div>
        {/each}
      </strong>
      <span>{t("dash.forecast")}</span>
    </div>
  </div>
{/if}

<style>
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 1rem;
  }
  .tile {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    color: inherit;
  }
  .tile strong {
    font-size: 1.5rem;
  }
</style>
