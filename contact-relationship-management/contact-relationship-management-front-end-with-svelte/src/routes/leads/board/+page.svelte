<!--
  Lead triage board (`/leads/board`): the lead lifecycle as SVAR
  Kanban columns; drag = the status transition (the service's
  lifecycle machine refuses illegal moves with 422 and the reload
  restores the stored truth). Cards carry the rule-derived score.
-->
<script lang="ts">
  import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
  import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
  import { leadStatus, listLeads, type Lead } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";

  let leads = $state<Lead[] | null>(null);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  async function load() {
    try {
      leads = await listLeads();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }
  $effect(() => {
    void load();
  });

  const columns = [
    { id: "new", label: "New", addCard: false },
    { id: "contacted", label: "Contacted", addCard: false },
    { id: "qualified", label: "Qualified", addCard: false },
    { id: "converted", label: "Converted", addCard: false },
    { id: "disqualified", label: "Disqualified", addCard: false },
  ];

  const cards = $derived(
    (leads ?? []).map((lead) => ({
      id: lead.pid,
      label: lead.display_name,
      description: `${lead.source} · score ${lead.score}`,
      status: lead.status,
    })),
  );

  function init(api: KanbanInstanceApi) {
    api.on("move-card", (raw) => {
      const ev = raw as { id: string | number; column?: string | number };
      if (!ev.column) return;
      actionError = null;
      void leadStatus(String(ev.id), String(ev.column))
        .catch((cause) => {
          actionError = cause instanceof Error ? cause.message : String(cause);
        })
        .finally(load);
    });
  }
</script>

<svelte:head><title>{t("nav.leads")} — CRM</title></svelte:head>

<h1>{t("nav.leads")} {t("common.board")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if leads === null}
  <p>{t("common.loading")}</p>
{:else}
  {#if actionError}
    <p class="error" data-testid="action-error">{actionError}</p>
  {/if}
  <div class="board-wrap" data-testid="lead-board">
    <Willow>
      <Kanban
        {cards}
        {columns}
        columnAccessor="status"
        card={{ ...getCardShape(), menu: false }}
        {init}
      />
    </Willow>
  </div>
{/if}

<style>
  .board-wrap {
    height: 560px;
    overflow-x: auto;
  }
</style>
