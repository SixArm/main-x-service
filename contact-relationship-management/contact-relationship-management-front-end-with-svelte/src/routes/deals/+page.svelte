<script lang="ts">
  import { forecast, listDeals, listPipelines, money, moveDeal } from "$lib/api/crm";
  import { i18n, t } from "$lib/i18n.svelte";
  import type { Deal, Stage } from "$lib/api/crm";

  let stages = $state<Stage[]>([]);
  let deals = $state<Deal[] | null>(null);
  let totals = $state<Record<string, number>>({});
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  async function load() {
    try {
      const pipelines = await listPipelines();
      const first = pipelines[0];
      stages = first?.stages ?? [];
      deals = first ? await listDeals(first.pipeline.pid) : [];
      totals = (await forecast()).totals_minor;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  /** The next open stage after this deal's, when any (board arrow). */
  function nextStage(deal: Deal): Stage | undefined {
    const index = stages.findIndex((stage) => stage.pid === deal.stage_pid);
    return stages[index + 1];
  }

  async function advance(deal: Deal) {
    const target = nextStage(deal);
    if (!target) return;
    actionError = null;
    try {
      await moveDeal(
        deal.pid,
        target.pid,
        target.is_lost ? "moved to lost from the board" : undefined,
      );
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  const columns = $derived(
    stages.map((stage) => ({
      stage,
      cards: (deals ?? []).filter(
        (deal) => deal.stage_pid === stage.pid && deal.closed_at === null,
      ),
    })),
  );
</script>

<h1>{t("deal.board")}</h1>

<p data-testid="forecast">
  {t("dash.forecast")}:
  {#each Object.entries(totals) as [currency, minor] (currency)}
    <strong>{money(minor, currency, i18n.locale)}</strong>&nbsp;
  {:else}
    <span class="muted">{t("dash.noData")}</span>
  {/each}
</p>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if deals === null}
  <p>{t("common.loading")}</p>
{:else}
  {#if actionError}
    <p class="error" data-testid="action-error">{actionError}</p>
  {/if}
  <div class="board" data-testid="deal-board">
    {#each columns as column (column.stage.pid)}
      <section class="column">
        <h2>
          {column.stage.name}
          <span class="muted">{column.stage.probability_percent}%</span>
        </h2>
        {#each column.cards as deal (deal.pid)}
          <article class="card">
            <strong>{deal.name}</strong>
            <div>{money(deal.amount_minor, deal.currency, i18n.locale)}</div>
            {#if nextStage(deal)}
              <button onclick={() => void advance(deal)}>→ {nextStage(deal)?.name}</button>
            {/if}
          </article>
        {/each}
      </section>
    {/each}
  </div>
{/if}

<style>
  .board {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(170px, 1fr);
    gap: 0.75rem;
    overflow-x: auto;
  }
  .column {
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 0.5rem;
  }
  .card {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 0.5rem;
    margin-bottom: 0.5rem;
  }
</style>
