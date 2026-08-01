<!--
  Organizations index route — SVAR DataGrid index route with a FilterBar
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
    import { OrganizationRepository } from "$lib/api/organizations";
    import type { OrgRef } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte";

    const repo = OrganizationRepository.withFetch();

    let items = $state<OrgRef[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);
    // Rows matching overall, from `X-Total-Count` — not `items.length`,
    // which is only this page. Shown so an operator can tell a short list
    // from a first page of a long one.
    let total = $state(0);

    onMount(async () => {
        try {
            const page = await repo.listPage();
            items = page.items;
            total = page.total;
        } catch (err) {
            error = err instanceof Error ? err.message : t("list.title");
        } finally {
            loading = false;
        }
    });

    // The list endpoint returns lightweight refs, so the grid carries
    // those columns; `pid` stays a technical literal.
    const columns = $derived([
        { id: "name", header: t("form.name"), flexgrow: 1 },
        { id: "pid", header: "pid", width: 300 },
    ]);

    const rows = $derived(
        items.map((r) => ({ id: r.pid, name: r.name, pid: r.pid })),
    );

    // FilterBar over the name column (contains-match, client-side).
    const filterFields = $derived([
        { id: "name", label: t("form.name"), type: "text" },
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
    <p class="count">
        {items.length === total
            ? `${total}`
            : `${items.length} / ${total}`}
    </p>
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

    /* The row count: just the total when the page holds everything, and
       "shown / total" when it does not, so a first page of a long list
       cannot be mistaken for a short list. */
    .count {
        margin: 0 0 0.5rem;
        opacity: 0.75;
        font-variant-numeric: tabular-nums;
    }
</style>
