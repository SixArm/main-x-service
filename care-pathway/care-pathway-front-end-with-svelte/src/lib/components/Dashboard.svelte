<!--
  Dashboard — the signed-in home (`/`): the care-pathway registry as a SVAR DataGrid
  with a FilterBar (client-side name filter over the loaded refs). Row
  selection navigates to the record's detail route. Mirrors the
  organization front-end's index-grid pattern exactly.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
    import {
        FilterBar,
        Willow as FilterTheme,
        createArrayFilter,
    } from "@svar-ui/svelte-filter";
    import { CarePathwayRepository } from "#lib/api/care-pathways.js";
    import type { PathwayRef } from "#lib/api/types.js";
    import { t } from "#lib/i18n.svelte.js";

    const repo = CarePathwayRepository.withFetch();

    let items = $state<PathwayRef[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);
    // The next page's URL, from `Link: rel="next"`, or `null` on the last.
    // There is no total, so the count says "+" while more follow, and an
    // operator can tell a short list from a first page of a long one.
    let next = $state<string | null>(null);
    let loadingMore = $state(false);

    onMount(async () => {
        try {
            const page = await repo.listPage();
            items = page.items;
            next = page.next;
        } catch (err) {
            error = err instanceof Error ? err.message : t("list.title");
        } finally {
            loading = false;
        }
    });

    async function loadMore() {
        if (!next || loadingMore) return;
        loadingMore = true;
        try {
            const page = await repo.listPage({}, next);
            items = [...items, ...page.items];
            next = page.next;
        } catch (err) {
            error = err instanceof Error ? err.message : t("list.title");
        } finally {
            loadingMore = false;
        }
    }

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
    // Site-wide search (PickerBar) navigates here as `?<terms>`; keep rows
    // whose name contains every term (case-insensitive).
    const terms = $derived(
        decodeURIComponent(page.url.search.slice(1))
            .toLowerCase()
            .split(/\s+/)
            .filter(Boolean),
    );
    const searched = $derived(
        terms.length
            ? rows.filter((r) =>
                  terms.every((w) => String(r.name ?? "").toLowerCase().includes(w)),
              )
            : rows,
    );
    const filtered = $derived(
        filterRules
            ? createArrayFilter(
                  filterRules as Parameters<typeof createArrayFilter>[0],
              )(searched)
            : searched,
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
<p><a class="button" href="/new">{t("list.new")}</a></p>

{#if loading}
    <p>{t("list.loading")}</p>
{:else if error}
    <p class="banner error" role="alert">{error}</p>
{:else if items.length === 0}
    <p class="surface">{t("list.empty")} <a href="/new">{t("list.createOne")}</a>.</p>
{:else}
    <p class="count">{next ? `${items.length}+` : `${items.length}`}</p>
    <div data-testid="pathway-grid">
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
    {#if next}
        <button type="button" onclick={loadMore} disabled={loadingMore}>
            {t("list.more")}
        </button>
    {/if}
{/if}

<style>
  /* The row count, with "+" while more pages follow, so a first page of
     a long list cannot be mistaken for a short list. */
  .count {
    margin: 0 0 0.5rem;
    opacity: 0.75;
  }
  .filter-wrap {
    margin-bottom: 0.5rem;
  }
  .grid-wrap {
    height: 480px;
    overflow: hidden;
  }
</style>
