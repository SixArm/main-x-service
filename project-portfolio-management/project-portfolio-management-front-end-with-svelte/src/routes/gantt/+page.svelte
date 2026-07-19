<!--
  Gantt route (`/gantt`) — the selected portfolio's schedule (PPM-6)
  in the SVAR Gantt: dated work items as task bars, dependency edges
  as links, the critical path highlighted via task CSS. Read-only in
  v1: schedule edits stay with the schedule endpoints; the Gantt is a
  derived visualization, not a second editor.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { Gantt, Willow } from "@svar-ui/svelte-gantt";
  import { PpmClient } from "$lib/api/ppm";
  import { WorkItemRepository } from "$lib/api/work-items";
  import type { WorkItemRef } from "$lib/api/types";
  import type { ScheduleView } from "$lib/api/ppm";
  import { t } from "$lib/i18n.svelte";

  const client = PpmClient.withFetch();

  let portfolios = $state<WorkItemRef[]>([]);
  let selected = $state<string>("");
  let schedule = $state<ScheduleView | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function loadSchedule(pid: string) {
    if (!pid) return;
    loading = true;
    error = null;
    try {
      schedule = await client.schedule(pid);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      schedule = null;
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    try {
      portfolios = await WorkItemRepository.withFetch("portfolios").list();
      selected = portfolios[0]?.pid ?? "";
      if (selected) await loadSchedule(selected);
      else loading = false;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      loading = false;
    }
  });

  // Dated schedule items become task bars; the critical path gets a
  // CSS marker class. Undated items are listed below the chart rather
  // than invented onto the timeline.
  const tasks = $derived(
    (schedule?.items ?? [])
      .filter((i) => i.start && i.end)
      .map((i) => ({
        id: i.pid,
        text: i.name,
        start: new Date(i.start ?? ""),
        end: new Date(i.end ?? ""),
        type: "task",
        css: i.on_critical_path ? "ppm-critical" : "",
      })),
  );

  const links = $derived(
    (schedule?.edges ?? []).map((e) => ({
      id: e.pid,
      source: e.predecessor_pid,
      target: e.successor_pid,
      type: "e2s",
    })),
  );

  const undated = $derived(
    (schedule?.items ?? []).filter((i) => !i.start || !i.end),
  );
</script>

<svelte:head><title>{t("ppm.nav.gantt")} — Main X</title></svelte:head>

<h1>{t("ppm.nav.gantt")}</h1>

<p>
  <label>
    {t("detail.opened")}
    <select
      bind:value={selected}
      onchange={() => void loadSchedule(selected)}
    >
      {#each portfolios as p (p.pid)}
        <option value={p.pid}>{p.name}</option>
      {/each}
    </select>
  </label>
</p>

{#if loading}
  <p>{t("list.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if !schedule || tasks.length === 0}
  <p class="surface">{t("list.empty")}</p>
{:else}
  <div class="gantt-wrap" data-testid="ppm-gantt">
    <Willow>
      <Gantt {tasks} {links} readonly />
    </Willow>
  </div>
  {#if undated.length > 0}
    <p class="muted small">
      {t("list.empty")}:
      {#each undated as item (item.pid)}
        <span class="chip">{item.name}</span>
      {/each}
    </p>
  {/if}
{/if}

<style>
  .gantt-wrap {
    height: 560px;
  }
  .gantt-wrap :global(.ppm-critical .wx-bar) {
    --wx-gantt-task-color: #c0392b;
  }
</style>
