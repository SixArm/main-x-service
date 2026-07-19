<!--
  Status board route (`/board`) — cases as SVAR Kanban cards, one
  column per unit lifecycle status. Dragging a card to another column
  writes the status change through the normal update endpoint (a full
  Case PUT), then reloads the truth.

  The list endpoint returns `{pid, title}` refs, so the board loads
  the full records behind them (capped) — fine at demo scale; a
  status-bearing list endpoint is the optimisation seam.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
  import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
  import { CaseRepository } from "$lib/api/cases";
  import type { Case } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  const repo = CaseRepository.withFetch();

  /** Board columns: the unit statuses (Custom-status cases are shown
   * in an extra column and are not draggable targets). */
  const STATUSES = [
    "Open",
    "InProgress",
    "Pending",
    "OnHold",
    "Resolved",
    "Closed",
  ] as const;

  let cases = $state<Map<string, Case>>(new Map());
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load() {
    try {
      const refs = await repo.list();
      const capped = refs.slice(0, 100);
      const full = await Promise.all(capped.map((r) => repo.get(r.pid)));
      const next = new Map<string, Case>();
      capped.forEach((r, i) => {
        const record = full[i];
        if (record) next.set(r.pid, record);
      });
      cases = next;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  const columns = $derived(
    STATUSES.map((s) => ({ id: s, label: s, addCard: false })),
  );

  const cards = $derived(
    [...cases.entries()]
      .filter(([, c]) => typeof c.status === "string")
      .map(([pid, c]) => ({
        id: pid,
        label: c.title,
        description: c.case_number ?? "",
        status: c.status as string,
      })),
  );

  // Drag = a status transition: PUT the full record with the new
  // status, then reload the truth (a rejected transition reloads too,
  // so the board never lies).
  function init(api: KanbanInstanceApi) {
    api.on("move-card", (raw) => {
      const ev = raw as { id: string | number; column?: string | number };
      const record = cases.get(String(ev.id));
      if (!record || !ev.column) return;
      const updated: Case = { ...record, status: String(ev.column) as Case["status"] };
      void repo
        .update(String(ev.id), updated)
        .catch((err) => {
          error = err instanceof Error ? err.message : String(err);
        })
        .finally(load);
    });
  }
</script>

<svelte:head><title>{t("nav.board")} — Main X</title></svelte:head>

<h1>{t("nav.board")}</h1>

{#if loading}
  <p>{t("list.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else}
  <div class="board-wrap" data-testid="case-board">
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
