<!--
  Ticket board (`/tickets/board`): open / pending / resolved / closed
  as SVAR Kanban columns; drag = the status transition (the lifecycle
  machine owns legality; a refused move reloads to the stored truth).
  Cards badge priority and live SLA-breach flags.
-->
<script lang="ts">
  import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
  import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
  import { listTickets, ticketStatus, type Ticket } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";

  let tickets = $state<Ticket[] | null>(null);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  async function load() {
    try {
      tickets = await listTickets();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }
  $effect(() => {
    void load();
  });

  const columns = [
    { id: "open", label: "Open", addCard: false },
    { id: "pending", label: "Pending", addCard: false },
    { id: "resolved", label: "Resolved", addCard: false },
    { id: "closed", label: "Closed", addCard: false },
  ];

  const cards = $derived(
    (tickets ?? []).map((ticket) => ({
      id: ticket.pid,
      label: ticket.title,
      description:
        ticket.priority +
        (ticket.live_first_response_breached || ticket.live_resolution_breached
          ? " · SLA breached"
          : ""),
      status: ticket.status,
    })),
  );

  function init(api: KanbanInstanceApi) {
    api.on("move-card", (raw) => {
      const ev = raw as { id: string | number; column?: string | number };
      if (!ev.column) return;
      actionError = null;
      void ticketStatus(String(ev.id), String(ev.column))
        .catch((cause) => {
          actionError = cause instanceof Error ? cause.message : String(cause);
        })
        .finally(load);
    });
  }
</script>

<svelte:head><title>{t("nav.tickets")} — CRM</title></svelte:head>

<h1>{t("nav.tickets")} {t("common.board")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if tickets === null}
  <p>{t("common.loading")}</p>
{:else}
  {#if actionError}
    <p class="error" data-testid="action-error">{actionError}</p>
  {/if}
  <div class="board-wrap" data-testid="ticket-board">
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
