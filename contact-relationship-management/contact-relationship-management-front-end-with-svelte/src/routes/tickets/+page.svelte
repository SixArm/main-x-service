<script lang="ts">
  import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
  import {
    FilterBar,
    Willow as FilterTheme,
    createArrayFilter,
  } from "@svar-ui/svelte-filter";
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
  let selectedPid = $state<string | null>(null);
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

  function breached(ticket: Ticket): boolean {
    return Boolean(ticket.live_first_response_breached || ticket.live_resolution_breached);
  }

  const breachedCount = $derived((tickets ?? []).filter(breached).length);
  const selected = $derived((tickets ?? []).find((x) => x.pid === selectedPid) ?? null);

  const columns = $derived([
    { id: "title", header: t("common.name"), flexgrow: 1 },
    { id: "priority", header: t("ticket.priority"), width: 100 },
    { id: "status", header: t("common.status"), width: 100 },
    { id: "due", header: t("ticket.due"), width: 180 },
    { id: "breach", header: t("ticket.breached"), width: 110 },
  ]);

  const rows = $derived(
    (tickets ?? []).map((x) => ({
      id: x.pid,
      title: x.title,
      priority: x.priority,
      status: x.status,
      due: x.first_response_due_at
        ? new Date(x.first_response_due_at).toLocaleString()
        : "",
      breach: breached(x) ? t("ticket.breached") : "",
    })),
  );

  const filterFields = $derived([
    { id: "title", label: t("common.name"), type: "text" },
    { id: "priority", label: t("ticket.priority"), type: "text" },
    { id: "status", label: t("common.status"), type: "text" },
  ]);
  let filterRules = $state<unknown>(null);
  const filtered = $derived(
    filterRules
      ? createArrayFilter(filterRules as Parameters<typeof createArrayFilter>[0])(rows)
      : rows,
  );

  function initGrid(api: {
    on(action: string, cb: (ev: { id: string | number }) => void): void;
  }) {
    api.on("select-row", (ev) => {
      selectedPid = String(ev.id);
    });
  }

  async function move(pid: string, to: string) {
    actionError = null;
    try {
      await ticketStatus(pid, to);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }
</script>

<h1>{t("nav.tickets")}</h1>
<p><a class="button" href="/tickets/board">{t("common.board")}</a></p>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if tickets === null}
  <p>{t("common.loading")}</p>
{:else}
  {#if breachedCount > 0}
    <p><strong class="breach" data-testid="breached">{t("ticket.breached")}: {breachedCount}</strong></p>
  {/if}
  <div data-testid="ticket-queue">
    <GridTheme>
      <FilterTheme>
        <div class="filter-wrap">
          <FilterBar
            fields={filterFields}
            onchange={({ value }: { value: unknown }) => (filterRules = value)}
          />
        </div>
        <div class="grid-wrap">
          <Grid data={filtered} {columns} select init={initGrid} />
        </div>
      </FilterTheme>
    </GridTheme>
  </div>
  {#if selected}
    <div class="panel">
      <strong>{selected.title}</strong>
      {#each NEXT[selected.status] ?? [] as to (to)}
        <button onclick={() => void move(selected.pid, to)}>→ {to}</button>
      {/each}
      {#if actionError}
        <p class="error" data-testid="action-error">{actionError}</p>
      {/if}
    </div>
  {/if}
{/if}

<style>
  .filter-wrap {
    margin-bottom: 0.5rem;
  }
  .grid-wrap {
    height: 420px;
    overflow: hidden;
  }
  .breach {
    color: var(--state-closed, #8a1d2d);
  }
</style>
