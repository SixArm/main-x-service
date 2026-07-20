<!--
  Sales executive area (`/executive`): the period pack (deals won /
  lost with per-currency won value, lost reasons verbatim, leads,
  tickets, activities, consent withdrawals), stale-deal aging,
  pipeline-hygiene findings, and the stored forecast-trend series.
  All numbers server-derived; currencies never merged.
-->
<script lang="ts">
  import {
    executivePack,
    forecastTrends,
    money,
    pipelineHygiene,
    staleDeals,
    type Finding,
    type StaleDeal,
  } from "$lib/api/crm";
  import { i18n, t } from "$lib/i18n.svelte";

  type Pack = Awaited<ReturnType<typeof executivePack>>;
  type Trends = Awaited<ReturnType<typeof forecastTrends>>;

  let pack = $state<Pack | null>(null);
  let stale = $state<StaleDeal[]>([]);
  let staleDerivation = $state("");
  let findings = $state<Finding[]>([]);
  let trends = $state<Trends | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        pack = await executivePack();
        const aging = await staleDeals();
        stale = aging.deals.filter((deal) => deal.stale);
        staleDerivation = aging.derivation;
        findings = (await pipelineHygiene()).findings;
        trends = await forecastTrends();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
</script>

<svelte:head><title>{t("nav.executive")} — CRM</title></svelte:head>

<h1>{t("nav.executive")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if pack}
  <p class="muted">window {pack.window.from} → {pack.window.to} · {pack.note}</p>
  <section class="tiles" data-testid="exec-tiles">
    <div class="tile"><strong>{pack.deals_won}</strong><span>won</span></div>
    <div class="tile"><strong>{pack.deals_lost}</strong><span>lost</span></div>
    <div class="tile"><strong>{pack.new_leads}</strong><span>new leads</span></div>
    <div class="tile"><strong>{pack.tickets_opened}</strong><span>tickets opened</span></div>
    <div class="tile"><strong>{pack.tickets_resolved}</strong><span>resolved</span></div>
    <div class="tile"><strong>{pack.consent_withdrawals}</strong><span>consent withdrawals</span></div>
  </section>
  <p data-testid="exec-won-value">
    Won value:
    {#each Object.entries(pack.won_value_by_currency_minor) as [currency, minor] (currency)}
      <strong>{money(minor, currency, i18n.locale)}</strong>&nbsp;
    {:else}
      <span class="muted">none in window</span>
    {/each}
  </p>
  {#if Object.keys(pack.lost_reasons).length > 0}
    <p data-testid="exec-lost-reasons">
      Lost reasons:
      {#each Object.entries(pack.lost_reasons) as [reason, count] (reason)}
        {reason} ({count})&nbsp;
      {/each}
    </p>
  {/if}
{/if}

{#if stale.length > 0}
  <h2>Stale deals</h2>
  <p class="muted">{staleDerivation}</p>
  <table data-testid="exec-stale">
    <thead>
      <tr><th>Deal</th><th>Stage</th><th>Days in stage</th><th>Amount</th></tr>
    </thead>
    <tbody>
      {#each stale as deal (deal.pid)}
        <tr>
          <td>{deal.name}</td>
          <td>{deal.stage ?? "—"}</td>
          <td>{deal.days_in_stage}</td>
          <td>{money(deal.amount_minor, deal.currency, i18n.locale)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<h2>Pipeline hygiene</h2>
<table data-testid="exec-hygiene">
  <thead><tr><th>Rule</th><th>Detail</th></tr></thead>
  <tbody>
    {#each findings as finding, index (index)}
      <tr><td><code>{finding.rule}</code></td><td>{finding.detail}</td></tr>
    {:else}
      <tr><td colspan="2" class="muted">No findings.</td></tr>
    {/each}
  </tbody>
</table>

{#if trends}
  <h2>Forecast trend</h2>
  <p class="muted">{trends.note}</p>
  <table data-testid="exec-trends">
    <thead><tr><th>Taken</th><th>Totals</th></tr></thead>
    <tbody>
      {#each trends.series as snapshot (snapshot.taken_on)}
        <tr>
          <td>{snapshot.taken_on}</td>
          <td>
            {#each Object.entries(snapshot.totals) as [currency, minor] (currency)}
              {money(minor, currency, i18n.locale)}&nbsp;
            {/each}
          </td>
        </tr>
      {:else}
        <tr><td colspan="2" class="muted">No snapshots captured yet.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}
