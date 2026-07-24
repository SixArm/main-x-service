<script lang="ts">
  import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
  import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
  import { listRequisitions, money, requisitionStatus } from "$lib/api/wpm";
  import { i18n, t } from "$lib/i18n.svelte";
  import type { Requisition } from "$lib/api/types";

  /** Board columns: the live pipeline statuses (cancelled stays off
   * the board; the state machine still owns which drags are legal). */
  const COLUMNS = ["draft", "open", "interviewing", "offer", "filled"] as const;

  let requisitions = $state<Requisition[] | null>(null);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  async function load() {
    try {
      requisitions = await listRequisitions();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  const columns = $derived(
    COLUMNS.map((status) => ({ id: status, label: status, addCard: false })),
  );

  const cards = $derived(
    (requisitions ?? [])
      .filter((r) => r.status !== "cancelled")
      .map((r) => ({
        id: r.pid,
        label: r.job_title,
        description:
          `${r.department} · ${t("req.headcount")} ${r.headcount}` +
          (r.salary_min_minor !== null && r.salary_max_minor !== null
            ? ` · ${money(r.salary_min_minor, r.salary_currency, i18n.locale)}–${money(r.salary_max_minor, r.salary_currency, i18n.locale)}`
            : ""),
        status: r.status,
      })),
  );

  // Drag = a pipeline transition through the API; the service's state
  // machine refuses illegal moves (422), and the reload puts the card
  // back where the truth says it belongs.
  function init(api: KanbanInstanceApi) {
    api.on("move-card", (raw) => {
      const ev = raw as { id: string | number; column?: string | number };
      if (!ev.column) return;
      actionError = null;
      void requisitionStatus(String(ev.id), String(ev.column))
        .catch((cause) => {
          actionError = cause instanceof Error ? cause.message : String(cause);
        })
        .finally(load);
    });
  }
</script>

<h1>{t("nav.requisitions")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if requisitions === null}
  <p>{t("common.loading")}</p>
{:else}
  {#if actionError}
    <p class="error" data-testid="action-error">{actionError}</p>
  {/if}
  <div class="board-wrap" data-testid="requisition-board">
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
    height: 620px;
    overflow-x: auto;
  }
</style>
