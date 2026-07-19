<!--
  Wards index route (`/wards`) — the ward estate in the SVAR DataGrid
  with a SVAR FilterBar above it (client-side filtering). Row
  selection opens the ward's whiteboard.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
  import {
    FilterBar,
    Willow as FilterTheme,
    createArrayFilter,
  } from "@svar-ui/svelte-filter";
  import { getWards } from "$lib/api/flow";
  import type { Ward } from "$lib/api/types";

  let wards = $state<Ward[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      wards = await getWards();
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to load wards";
    } finally {
      loading = false;
    }
  });

  const columns = [
    { id: "code", header: "Code", width: 90 },
    { id: "name", header: "Ward", flexgrow: 1 },
    { id: "kind", header: "Kind", width: 110 },
    { id: "specialty", header: "Specialty", width: 140 },
    { id: "open", header: "Open", width: 80 },
    { id: "closed_to_admissions", header: "Closed to admissions", width: 160 },
  ];

  const rows = $derived(
    wards.map((w) => ({
      id: w.pid,
      code: w.code,
      name: w.name,
      kind: w.kind,
      specialty: w.specialty ?? "",
      open: w.open ? "yes" : "no",
      closed_to_admissions: w.closed_to_admissions ? "yes" : "no",
    })),
  );

  const filterFields = [
    { id: "code", label: "Code", type: "text" },
    { id: "name", label: "Ward", type: "text" },
    { id: "kind", label: "Kind", type: "text", options: ["inpatient", "assessment", "virtual"] },
    { id: "specialty", label: "Specialty", type: "text" },
  ];
  let filterRules = $state<unknown>(null);
  const filtered = $derived(
    filterRules
      ? createArrayFilter(filterRules as Parameters<typeof createArrayFilter>[0])(rows)
      : rows,
  );

  // Row selection opens the ward whiteboard.
  function initGrid(api: {
    on(action: string, cb: (ev: { id: string | number }) => void): void;
  }) {
    api.on("select-row", (ev) => {
      void goto(`/wards/${ev.id}/whiteboard`);
    });
  }
</script>

<svelte:head><title>Wards — Patient Flow</title></svelte:head>

<h1>Wards</h1>

{#if loading}
  <p>Loading…</p>
{:else if error}
  <p class="error">{error}</p>
{:else}
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
{/if}

<style>
  .filter-wrap {
    margin-bottom: 0.5rem;
  }
  .grid-wrap {
    height: 480px;
    overflow: hidden;
  }
</style>
