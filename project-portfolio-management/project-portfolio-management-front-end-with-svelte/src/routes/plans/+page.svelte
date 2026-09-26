<!--
  Plans index route (`/plans`).

  Purpose: fetch the plan refs and render them in the SVAR DataGrid with a
  SVAR FilterBar above it (client-side filtering over the loaded rows). Row
  selection navigates to the detail route.
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
  import { PlanRepository } from "$lib/api/plans";
  import type { PlanRef } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  const repo = PlanRepository.withFetch();

  let items = $state<PlanRef[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  // Rows matching overall, from `X-Total-Count` — not `items.length`,
  // which is only this page. Shown so an operator can tell a short list
  // from a first page of a long one (agents/share/restful.md), matching
  // the organization front-end's `/organizations` list convention.
  let total = $state(0);

  onMount(async () => {
    try {
      const page = await repo.listPage();
      items = page.items;
      total = page.total;
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

  // Row selection navigates to the plan detail route.
  function initGrid(api: {
    on(action: string, cb: (ev: { id: string | number }) => void): void;
  }) {
    api.on("select-row", (ev) => {
      void goto(`/plans/${ev.id}`);
    });
  }
</script>

<svelte:head><title>{t("list.title")} — Main X</title></svelte:head>

<h1>{t("list.title")}</h1>
<p><a class="button" href="/plans/new">{t("list.new")}</a></p>

{#if loading}
  <p>{t("list.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if items.length === 0}
  <p class="surface">{t("list.empty")} <a href="/plans/new">{t("list.createOne")}</a>.</p>
{:else}
  <p class="count">
    {items.length === total ? `${total}` : `${items.length} / ${total}`}
  </p>
  <GridTheme>
    <FilterTheme>
      <div class="filter-wrap">
        <FilterBar
          fields={filterFields}
          onchange={({ value }: { value: unknown }) => (filterRules = value)}
        />
      </div>
      <!-- T-28k (repo tasks.md EV-1): the SVAR grid is a poor fit under
           a phone viewport (fixed-height, horizontal-scrolling columns).
           Both this and `.mobile-list` below render unconditionally;
           `@media (max-width: 600px)` is the sole switch, so there is
           no JS viewport detection and nothing to get out of sync with
           SSR (this app has none — ssr = false, +layout.ts). -->
      <div class="grid-wrap">
        <Grid data={filtered} {columns} select init={initGrid} />
      </div>
    </FilterTheme>
  </GridTheme>
  <ul class="mobile-list">
    {#each filtered as row (row.id)}
      <li><a href="/plans/{row.id}">{row.name}</a></li>
    {/each}
  </ul>
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
     cannot be mistaken for a short list (same convention as the
     organization service's front-end /organizations list route). */
  .count {
    margin: 0 0 0.5rem;
    opacity: 0.75;
    font-variant-numeric: tabular-nums;
  }

  /* T-28k: a plain, read-only list stands in for the SVAR grid under a
     phone viewport — hidden above the breakpoint, shown below it. */
  .mobile-list {
    display: none;
  }

  @media (max-width: 600px) {
    .grid-wrap {
      display: none;
    }
    /* T-28k: the SVAR FilterBar (`.filter-wrap`) sits OUTSIDE
       `.grid-wrap`, so hiding the grid alone left it as the actual
       overflow source (measured: its `.wx-filter-bar` rendered at a
       fixed ~610px regardless of viewport). Filtering a plain list of
       names has little value anyway, so it is hidden alongside the
       grid rather than reflowed. */
    .filter-wrap {
      display: none;
    }
    .mobile-list {
      display: block;
      list-style: none;
      margin: 0;
      padding: 0;
    }
    .mobile-list li {
      border-bottom: 1px solid var(--mxi-color-border);
    }
    .mobile-list a {
      display: block;
      padding: 0.75rem 0.25rem;
      color: inherit;
      text-decoration: none;
    }
  }
</style>
