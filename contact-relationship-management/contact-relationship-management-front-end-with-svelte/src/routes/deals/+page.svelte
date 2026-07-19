<script lang="ts">
  import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
  import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
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

  // Columns are the pipeline's stage rows — data, not a token table
  // (CRM-D3); terminal stages are drop targets that close the deal.
  const columns = $derived(
    stages.map((stage) => ({
      id: stage.pid,
      label: `${stage.name} ${stage.probability_percent}%`,
      addCard: false,
    })),
  );

  const cards = $derived(
    (deals ?? [])
      .filter((deal) => deal.closed_at === null)
      .map((deal) => ({
        id: deal.pid,
        label: deal.name,
        description: money(deal.amount_minor, deal.currency, i18n.locale),
        stage_pid: deal.stage_pid,
      })),
  );

  // Drag between stage columns = the stage-move API (a lost target
  // carries a reason; the service enforces pipeline membership and
  // terminal immutability). The forecast strip re-reads the derived
  // number after every move — never client math.
  function init(api: KanbanInstanceApi) {
    api.on("move-card", (raw) => {
      const ev = raw as { id: string | number; column?: string | number };
      if (!ev.column) return;
      const target = stages.find((stage) => stage.pid === String(ev.column));
      actionError = null;
      void moveDeal(
        String(ev.id),
        String(ev.column),
        target?.is_lost ? "moved to lost on the board" : undefined,
      )
        .catch((cause) => {
          actionError = cause instanceof Error ? cause.message : String(cause);
        })
        .finally(load);
    });
  }
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
  <div class="board-wrap" data-testid="deal-board">
    <Willow>
      <Kanban
        {cards}
        {columns}
        columnAccessor="stage_pid"
        card={{ ...getCardShape(), menu: false }}
        {init}
      />
    </Willow>
  </div>
{/if}

<style>
  .board-wrap {
    height: 620px;
    overflow-x: auto;
  }
</style>
