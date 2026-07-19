<script lang="ts">
  import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
  import {
    FilterBar,
    Willow as FilterTheme,
    createArrayFilter,
  } from "@svar-ui/svelte-filter";
  import { getLead, listLeads } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";
  import type { Lead, ScoreBreakdown } from "$lib/api/crm";

  let leads = $state<Lead[] | null>(null);
  let selectedPid = $state<string | null>(null);
  let breakdown = $state<{ pid: string; score: ScoreBreakdown } | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        leads = await listLeads();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  const columns = $derived([
    { id: "score", header: t("lead.score"), width: 90 },
    { id: "display_name", header: t("common.name"), flexgrow: 1 },
    { id: "source", header: t("lead.source"), width: 120 },
    { id: "status", header: t("common.status"), width: 130 },
  ]);

  const rows = $derived(
    (leads ?? []).map((l) => ({
      id: l.pid,
      score: l.score,
      display_name: l.display_name,
      source: l.source,
      status: l.status,
    })),
  );

  const filterFields = $derived([
    { id: "display_name", label: t("common.name"), type: "text" },
    { id: "source", label: t("lead.source"), type: "text" },
    { id: "status", label: t("common.status"), type: "text" },
  ]);
  let filterRules = $state<unknown>(null);
  const filtered = $derived(
    filterRules
      ? createArrayFilter(filterRules as Parameters<typeof createArrayFilter>[0])(rows)
      : rows,
  );

  // Track the selected row so the breakdown button explains it.
  function initGrid(api: {
    on(action: string, cb: (ev: { id: string | number }) => void): void;
  }) {
    api.on("select-row", (ev) => {
      selectedPid = String(ev.id);
    });
  }

  // Explain the selected lead (defaults to the top-scored lead so the
  // button always has a subject).
  async function explain() {
    const pid = selectedPid ?? leads?.[0]?.pid;
    if (!pid) return;
    const detail = await getLead(pid);
    breakdown = { pid, score: detail.score };
  }
</script>

<h1>{t("nav.leads")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if leads === null}
  <p>{t("common.loading")}</p>
{:else}
  <div data-testid="lead-queue">
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
  <p><button onclick={() => void explain()}>{t("lead.breakdown")}</button></p>
  {#if breakdown}
    <div class="panel" data-testid="breakdown">
      <strong>{breakdown.score.score} · {breakdown.score.label}</strong>
      {#each breakdown.score.rules as rule (rule.rule)}
        <span class="chip">{rule.rule}: {rule.points}</span>
      {/each}
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
</style>
