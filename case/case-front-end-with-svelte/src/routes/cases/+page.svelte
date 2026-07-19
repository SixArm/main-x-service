<!--
  Cases index route — SVAR DataGrid index route with a FilterBar
  (client-side filtering over the loaded refs). Row selection
  navigates to the record's detail route.
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
    import { CaseRepository } from "$lib/api/cases";
    import type { CaseRef } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte";

    const repo = CaseRepository.withFetch();

    let items = $state<CaseRef[]>([]);
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

    // The list endpoint returns lightweight refs, so the grid carries
    // those columns; `pid` stays a technical literal.
    const columns = $derived([
        { id: "name", header: t("form.title"), flexgrow: 1 },
        { id: "pid", header: "pid", width: 300 },
    ]);

    const rows = $derived(
        items.map((r) => ({ id: r.pid, name: r.title, pid: r.pid })),
    );

    // FilterBar over the name column (contains-match, client-side).
    const filterFields = $derived([
        { id: "name", label: t("form.title"), type: "text" },
    ]);
    let filterRules = $state<unknown>(null);
    const filtered = $derived(
        filterRules
            ? createArrayFilter(
                  filterRules as Parameters<typeof createArrayFilter>[0],
              )(rows)
            : rows,
    );

    // Row selection navigates to the record's detail route.
    function initGrid(api: {
        on(action: string, cb: (ev: { id: string | number }) => void): void;
    }) {
        api.on("select-row", (ev) => {
            void goto(`/${ev.id}`);
        });
    }
</script>

<svelte:head><title>{t("list.title")} — Main X</title></svelte:head>

<h1>{t("list.title")}</h1>

{#if loading}
    <p>{t("list.loading")}</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if items.length === 0}
    <p>{t("list.empty")}</p>
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
