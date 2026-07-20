<!--
  Per-item task board (`/{collection}/{pid}/board`) — the spec-§13
  operational Kanban: five status columns, drag = the PATCH board move
  (the service stamps status_changed_at / first done_at, so flow data
  stays true), plus sprint create/select with the honest burndown
  (real completions only — the server's derivation note is shown) and
  the last-24h standup digest. English-first, like the other PPM views.
-->
<script lang="ts">
  import { page } from "$app/state";
  import { Kanban, Willow, getCardShape } from "@svar-ui/svelte-kanban";
  import type { KanbanInstanceApi } from "@svar-ui/svelte-kanban";
  import {
    PpmClient,
    type Burndown,
    type Sprint,
    type Standup,
    type Task,
  } from "$lib/api/ppm";
  import { t } from "$lib/i18n.svelte";

  const collection = page.params.collection ?? "";
  const pid = page.params.pid ?? "";
  const ppm = PpmClient.withFetch();

  let tasks = $state<Task[] | null>(null);
  let sprints = $state<Sprint[]>([]);
  let selectedSprint = $state("");
  let burndown = $state<Burndown | null>(null);
  let standup = $state<Standup | null>(null);
  let error = $state<string | null>(null);
  let title = $state("");
  let sprintName = $state("");
  let sprintStart = $state("");
  let sprintEnd = $state("");

  async function load() {
    try {
      tasks = (await ppm.listTasks(collection, pid)).tasks;
      sprints = await ppm.listSprints(collection, pid);
      standup = await ppm.standup(collection, pid);
      // Default to the latest sprint so the burndown shows up unprompted.
      if (!selectedSprint && sprints.length > 0) {
        selectedSprint = sprints[0]?.pid ?? "";
      }
      if (selectedSprint) {
        burndown = await ppm.burndown(collection, pid, selectedSprint);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  }
  $effect(() => {
    void load();
  });

  async function act(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
      await load();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.actionFailed");
    }
  }

  const COLUMNS = [
    { id: "todo", label: "Todo", addCard: false },
    { id: "in_progress", label: "In progress", addCard: false },
    { id: "in_review", label: "In review", addCard: false },
    { id: "done", label: "Done", addCard: false },
    { id: "blocked", label: "Blocked", addCard: false },
  ];

  const cards = $derived(
    (tasks ?? []).map((task) => ({
      id: task.pid,
      label: task.title,
      description:
        (task.assignee_ref ?? "unassigned") +
        (task.blocked_days !== null ? ` · blocked ${task.blocked_days}d` : ""),
      status: task.status,
    })),
  );

  // Drag = the PATCH board move; the service owns the stamps and the
  // reload restores the stored truth on refusal.
  function init(api: KanbanInstanceApi) {
    api.on("move-card", (raw) => {
      const ev = raw as { id: string | number; column?: string | number };
      if (!ev.column) return;
      void act(() => ppm.moveTask(collection, pid, String(ev.id), String(ev.column)));
    });
  }

  async function selectSprint(value: string) {
    selectedSprint = value;
    burndown = null;
    if (value) {
      try {
        burndown = await ppm.burndown(collection, pid, value);
      } catch (err) {
        error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
      }
    }
  }
</script>

<svelte:head><title>{t("ppm.common.board")} — PPM</title></svelte:head>

<h1>{t("ppm.common.board")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

<form
  class="row"
  onsubmit={(event) => {
    event.preventDefault();
    if (!title.trim()) return;
    const body: Record<string, unknown> = { title: title.trim() };
    if (selectedSprint) body["sprint_pid"] = selectedSprint;
    title = "";
    void act(() => ppm.createTask(collection, pid, body));
  }}
>
  <input bind:value={title} placeholder="New task title" aria-label="New task title" />
  <button type="submit">Add task</button>
  <label>
    Sprint
    <select
      value={selectedSprint}
      onchange={(event) => void selectSprint(event.currentTarget.value)}
    >
      <option value="">(none)</option>
      {#each sprints as sprint (sprint.pid)}
        <option value={sprint.pid}>{sprint.name}</option>
      {/each}
    </select>
  </label>
</form>

{#if tasks !== null}
  <div class="board-wrap" data-testid="task-board">
    <Willow>
      <Kanban
        {cards}
        columns={COLUMNS}
        columnAccessor="status"
        card={{ ...getCardShape(), menu: false }}
        {init}
      />
    </Willow>
  </div>
{/if}

<h2>Sprints</h2>
<form
  class="row"
  onsubmit={(event) => {
    event.preventDefault();
    if (!sprintName.trim() || !sprintStart || !sprintEnd) return;
    const body = { name: sprintName.trim(), starts_on: sprintStart, ends_on: sprintEnd };
    sprintName = "";
    void act(() => ppm.createSprint(collection, pid, body));
  }}
>
  <input bind:value={sprintName} placeholder="Sprint name" aria-label="Sprint name" />
  <input type="date" bind:value={sprintStart} aria-label="Sprint start" />
  <input type="date" bind:value={sprintEnd} aria-label="Sprint end" />
  <button type="submit">Add sprint</button>
</form>

{#if burndown}
  <h2>Burndown — {burndown.sprint.name}</h2>
  <p class="muted">{burndown.derivation}</p>
  <table data-testid="burndown">
    <thead><tr><th>Date</th><th>Remaining of {burndown.total_tasks}</th></tr></thead>
    <tbody>
      {#each burndown.points as point (point.date)}
        <tr><td>{point.date}</td><td>{point.remaining}</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

{#if standup}
  <h2>Standup (last 24h)</h2>
  <div data-testid="standup">
    <p>
      {standup.tasks_created.length} created ·
      {standup.tasks_moved.length} moves ·
      {standup.blocked_now.length} blocked now
    </p>
    {#if standup.blocked_now.length > 0}
      <ul>
        {#each standup.blocked_now as task (task.pid)}
          <li>{task.title} <span class="muted">blocked {task.blocked_days}d</span></li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .row { display: flex; gap: 0.75rem; align-items: end; flex-wrap: wrap; margin-bottom: 1rem; }
  .board-wrap { height: 480px; overflow-x: auto; }
</style>
