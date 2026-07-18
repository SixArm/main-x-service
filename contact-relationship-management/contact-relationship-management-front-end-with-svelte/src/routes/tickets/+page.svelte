<script lang="ts">
  import { listTickets, ticketStatus } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";
  import type { Ticket } from "$lib/api/crm";

  /** The forward move(s) a ticket offers per status. */
  const NEXT: Record<string, string[]> = {
    open: ["pending", "resolved"],
    pending: ["open", "resolved"],
    resolved: ["closed", "open"],
    closed: [],
  };

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

  async function move(ticket: Ticket, to: string) {
    actionError = null;
    try {
      await ticketStatus(ticket.pid, to);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  function breached(ticket: Ticket): boolean {
    return Boolean(ticket.live_first_response_breached || ticket.live_resolution_breached);
  }
</script>

<h1>{t("nav.tickets")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if tickets === null}
  <p>{t("common.loading")}</p>
{:else}
  {#if actionError}
    <p class="error" data-testid="action-error">{actionError}</p>
  {/if}
  <table data-testid="ticket-queue">
    <thead>
      <tr>
        <th>{t("common.name")}</th>
        <th>{t("ticket.priority")}</th>
        <th>{t("common.status")}</th>
        <th>{t("ticket.due")}</th>
        <th>{t("common.actions")}</th>
      </tr>
    </thead>
    <tbody>
      {#each tickets as ticket (ticket.pid)}
        <tr class={breached(ticket) ? "breached" : ""}>
          <td>{ticket.title}</td>
          <td><span class="chip">{ticket.priority}</span></td>
          <td><span class="chip">{ticket.status}</span></td>
          <td>
            {#if breached(ticket)}
              <strong class="breach" data-testid="breached">{t("ticket.breached")}</strong>
            {:else if ticket.first_response_due_at}
              {new Date(ticket.first_response_due_at).toLocaleString()}
            {/if}
          </td>
          <td>
            {#each NEXT[ticket.status] ?? [] as to (to)}
              <button onclick={() => void move(ticket, to)}>→ {to}</button>
            {/each}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .breach {
    color: var(--state-closed, #8a1d2d);
  }
</style>
