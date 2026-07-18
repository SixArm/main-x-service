<script lang="ts">
  import { listRequisitions, money, requisitionStatus } from "$lib/api/hcm";
  import { i18n, t } from "$lib/i18n.svelte";
  import type { Requisition } from "$lib/api/types";

  const COLUMNS = ["draft", "open", "interviewing", "offer", "filled"] as const;
  /** The forward move a card offers per column (cancel is implicit). */
  const NEXT: Record<string, string> = {
    draft: "open",
    open: "interviewing",
    interviewing: "offer",
    offer: "filled",
  };

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

  async function advance(requisition: Requisition) {
    actionError = null;
    const next = NEXT[requisition.status];
    if (!next) return;
    try {
      await requisitionStatus(requisition.pid, next);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  const byStatus = $derived(
    COLUMNS.map((status) => ({
      status,
      cards: (requisitions ?? []).filter((r) => r.status === status),
    })),
  );
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
  <div class="board" data-testid="requisition-board">
    {#each byStatus as column (column.status)}
      <section class="column">
        <h2>{column.status} <span class="muted">({column.cards.length})</span></h2>
        {#each column.cards as requisition (requisition.pid)}
          <article class="card">
            <a href={`/requisitions/${requisition.pid}`}><strong>{requisition.job_title}</strong></a>
            <div class="muted">{requisition.department} · {t("req.headcount")} {requisition.headcount}</div>
            {#if requisition.salary_min_minor !== null && requisition.salary_max_minor !== null}
              <div class="muted">
                {money(requisition.salary_min_minor, requisition.salary_currency, i18n.locale)}
                – {money(requisition.salary_max_minor, requisition.salary_currency, i18n.locale)}
              </div>
            {/if}
            {#if NEXT[requisition.status]}
              <button onclick={() => void advance(requisition)}>→ {NEXT[requisition.status]}</button>
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
    grid-template-columns: repeat(5, minmax(160px, 1fr));
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
