<!--
  Board area (`/board`): the period board pack (window decisions,
  benefits realized, milestones, tranche releases + as-of-now health),
  the investment-decisions view, and the stored trend series with an
  explicit "take snapshot" action. All numbers are server-derived;
  trends only ever show captured snapshots. English-first, like the
  other PPM views.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import {
    PpmClient,
    money,
    type BoardPack,
    type Investment,
    type TrendSeries,
  } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let pack = $state<BoardPack | null>(null);
  let investments = $state<Investment[] | null>(null);
  let trends = $state<TrendSeries | null>(null);
  let error = $state<string | null>(null);

  async function load() {
    try {
      pack = await ppm.boardPack();
      investments = (await ppm.boardInvestments()).investments;
      trends = await ppm.boardTrends();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  }
  onMount(load);

  async function snapshot() {
    error = null;
    try {
      await ppm.takeSnapshot();
      trends = await ppm.boardTrends();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.actionFailed");
    }
  }

  const label = (entry: Investment) =>
    entry.name ?? entry.title ?? entry.description ?? entry.kind;
  const amount = (entry: Investment) => {
    const minor =
      entry.planned_minor ?? entry.requested_minor ?? entry.budget_cap_minor;
    return minor !== null && minor !== undefined && entry.currency
      ? money(minor, entry.currency)
      : "—";
  };
</script>

<svelte:head><title>{t("ppm.nav.board")} — PPM</title></svelte:head>

<h1>{t("ppm.nav.board")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if pack}
  <h2>Board pack</h2>
  <p class="muted">window {pack.window.from} → {pack.window.to}</p>
  <section class="tiles" data-testid="board-tiles">
    <div class="tile"><strong>{pack.health_now.portfolios["red"] ?? 0}</strong><span>red portfolios</span></div>
    <div class="tile"><strong>{pack.health_now.portfolios["amber"] ?? 0}</strong><span>amber</span></div>
    <div class="tile"><strong>{pack.health_now.portfolios["green"] ?? 0}</strong><span>green</span></div>
    <div class="tile"><strong>{pack.milestones_completed}</strong><span>milestones completed</span></div>
    <div class="tile"><strong>{pack.tranches_released.count}</strong><span>tranches released</span></div>
    <div class="tile"><strong>{pack.benefits_realized.events}</strong><span>benefit realizations</span></div>
  </section>
  <p class="muted">{pack.health_now.note}</p>

  {#if Object.keys(pack.benefits_realized.per_currency_minor).length > 0}
    <p data-testid="board-realized">
      Realized in window:
      {#each Object.entries(pack.benefits_realized.per_currency_minor) as [currency, minor] (currency)}
        <strong>{money(minor, currency)}</strong>&nbsp;
      {/each}
      {#if pack.benefits_realized.unattributed_events > 0}
        <span class="muted">({pack.benefits_realized.unattributed_events} unattributed events)</span>
      {/if}
    </p>
  {/if}

  <h3>Decisions in window</h3>
  <table data-testid="board-decisions">
    <thead><tr><th>When</th><th>Kind</th><th>Decision</th><th>Subject</th></tr></thead>
    <tbody>
      {#each pack.decisions as entry (entry.kind + entry.at + entry.subject.pid)}
        <tr>
          <td>{entry.at}</td>
          <td>{entry.kind}</td>
          <td>{entry.decision}</td>
          <td>{entry.subject.name ?? entry.subject.pid}</td>
        </tr>
      {:else}
        <tr><td colspan="4" class="muted">No decisions in the window.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

{#if investments}
  <h2>Investment decisions</h2>
  <table data-testid="board-investments">
    <thead><tr><th>When</th><th>Kind</th><th>What</th><th>Amount</th></tr></thead>
    <tbody>
      {#each investments as entry (entry.kind + entry.at + label(entry))}
        <tr>
          <td>{entry.at}</td>
          <td>{entry.kind}{#if entry.gate}&nbsp;· {entry.gate}{/if}</td>
          <td>{label(entry)}</td>
          <td>{amount(entry)}</td>
        </tr>
      {:else}
        <tr><td colspan="4" class="muted">No investment decisions recorded.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

{#if trends}
  <h2>Trends</h2>
  <p class="muted">{trends.note}</p>
  <p><button onclick={() => void snapshot()}>Take snapshot now</button></p>
  <table data-testid="board-trends">
    <thead><tr><th>Taken</th><th>Work items</th><th>Portfolios</th><th>Open exposure</th></tr></thead>
    <tbody>
      {#each trends.series as row (row.taken_at)}
        <tr>
          <td>{row.taken_at}</td>
          <td>{String(row.body["work_items"] ?? "—")}</td>
          <td>{String(row.body["portfolios"] ?? "—")}</td>
          <td>{String(row.body["open_exposure"] ?? "—")}</td>
        </tr>
      {:else}
        <tr><td colspan="4" class="muted">No snapshots captured yet.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}
