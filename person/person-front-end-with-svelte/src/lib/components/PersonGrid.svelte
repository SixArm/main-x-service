<!--
  PersonGrid — tabular person list backed by the SVAR (@svar-ui/svelte-grid)
  DataGrid.

  Flattens each Person into the flat row shape the grid expects, then maps
  the grid's row-select event back to the originating Person object so the
  parent receives a domain type, not a row record.

  Props:
    - persons: Person[] — records to display.
    - onselect?: (person) => void — called when a row is selected.

  State:
    - data ($derived) — persons flattened into grid rows (recomputes on change).
-->
<script lang="ts">
    import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
    import {
        FilterBar,
        Willow as FilterTheme,
        createArrayFilter,
    } from "@svar-ui/svelte-filter";
    import { t } from "$lib/i18n.svelte.js";
    import type { Person } from "$lib/api/types.js";

    let {
        persons,
        onselect,
    }: {
        persons: Person[];
        onselect?: (person: Person) => void;
    } = $props();

    // Column definitions for the SVAR grid (id/header/width per column).
    // Headers are translated reactively so a locale switch relabels them.
    const columns = $derived([
        { id: "id", header: t("grid.id"), width: 280 },
        { id: "family", header: t("grid.family"), width: 160 },
        { id: "given", header: t("grid.given"), width: 200 },
        { id: "birth_date", header: t("grid.dob"), width: 120 },
        { id: "gender", header: t("grid.gender"), width: 90 },
        { id: "active", header: t("grid.active"), width: 80 },
    ]);

    // SVAR Grid wants flat row records. The `id` field must match
    // the Person.id so the select-row event can map back to a Person.
    const data = $derived(
        persons.map((p) => ({
            id: p.id ?? "",
            family: p.name.family,
            given: p.name.given.join(" "),
            birth_date: p.birth_date ?? "",
            gender: p.gender,
            active: (p.active ?? true) ? t("grid.yes") : t("grid.no"),
        })),
    );

    // FilterBar fields — the filterable columns (the opaque `id`
    // column is excluded; uuids filter poorly). Text contains-match
    // per field, labels reusing the translated column headers.
    const filterFields = $derived(
        columns
            .filter((c) => c.id !== "id")
            .map((c) => ({ id: c.id, label: c.header, type: "text" })),
    );

    // The FilterBar's current rule tree; null = show everything.
    let filterRules = $state<unknown>(null);

    // Rows surviving the filter: createArrayFilter compiles the rule
    // tree into a transform over the flattened rows.
    const filtered = $derived(
        filterRules
            ? createArrayFilter(
                  filterRules as Parameters<typeof createArrayFilter>[0],
              )(data)
            : data,
    );

    // SVAR exposes events through an IApi reference. Subscribe in
    // `init` and look up the originating Person by id.
    function initGrid(api: {
        on(action: string, cb: (ev: { id: string | number }) => void): void;
    }) {
        api.on("select-row", (ev) => {
            const found = persons.find((p) => p.id === String(ev.id));
            if (found) onselect?.(found);
        });
    }
</script>

<GridTheme>
    <FilterTheme>
        <div class="filter-wrap">
            <FilterBar
                fields={filterFields}
                onchange={({ value }: { value: unknown }) =>
                    (filterRules = value)}
            />
        </div>
        <div class="grid-wrap">
            <Grid data={filtered} {columns} select init={initGrid} />
        </div>
    </FilterTheme>
</GridTheme>

<style>
    .filter-wrap {
        margin-bottom: 0.5rem;
    }
    .grid-wrap {
        height: 480px;
        border: 1px solid var(--mxi-color-border);
        border-radius: var(--mxi-radius);
        overflow: hidden;
    }
</style>
