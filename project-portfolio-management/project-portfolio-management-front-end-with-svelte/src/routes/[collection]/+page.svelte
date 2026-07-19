<!--
  Work-item index route (`/[collection]` — /portfolios, /projects,
  /products, /programs).

  Purpose: fetch the collection's work-item refs and render them in the
  SVAR DataGrid with a SVAR FilterBar above it (client-side filtering
  over the loaded rows). Row selection navigates to the detail route.
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
  import { WorkItemRepository } from "$lib/api/work-items";
  import type { WorkItemRef } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();
  const collection = $derived(data.collection);
  const repo = $derived(WorkItemRepository.withFetch(collection));

  let items = $state<WorkItemRef[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      items = await repo.list();
    } catch (err) {
      error = err instanceof Error ? err.message : t("list.loadFailed");
    } finally {
      loading = false;
    }
  });

  // The list endpoint returns lightweight `{pid, name}` refs, so the
  // grid carries those two columns; `pid` stays a technical literal.
  const columns = $derived([
    { id: "name", header: t("form.title"), flexgrow: 1 },
    { id: "pid", header: "pid", width: 300 },
  ]);

  const rows = $derived(items.map((r) => ({ id: r.pid, name: r.name, pid: r.pid })));

  // FilterBar over the name column (contains-match, client-side).
  const filterFields = $derived([
    { id: "name", label: t("form.title"), type: "text" },
  ]);
  let filterRules = $state<unknown>(null);
  const filtered = $derived(
    filterRules
      ? createArrayFilter(filterRules as Parameters<typeof createArrayFilter>[0])(rows)
      : rows,
  );

  // Row selection navigates to the work-item detail route.
  function initGrid(api: {
    on(action: string, cb: (ev: { id: string | number }) => void): void;
  }) {
    api.on("select-row", (ev) => {
      void goto(`/${collection}/${ev.id}`);
    });
  }
</script>

<svelte:head><title>{collection} — Main X</title></svelte:head>

<h1>{t("list.title")} — {collection}</h1>
<p><a class="button" href={`/${collection}/new`}>{t("list.new")}</a></p>

{#if loading}
  <p>{t("list.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if items.length === 0}
  <p class="surface">{t("list.empty")} <a href={`/${collection}/new`}>{t("list.createOne")}</a>.</p>
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
