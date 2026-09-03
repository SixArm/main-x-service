<!--
  WorkerGrid — tabular list of workers backed by the SVAR (@svar-ui/svelte-grid)
  DataGrid. Flattens each Worker into a row of primitive cell values and
  surfaces row selection back to the parent.

  Why SVAR uses browser-only APIs: this is why the whole app is CSR-only
  (see routes/+layout.ts).

  $props:
    - workers: Worker[] — the records to display.
    - onselect?: (worker) => void — called with the full Worker when a row
      is selected (used to navigate to the detail page).

  $derived:
    - data — `workers` flattened to the grid's row shape (id, family,
      given, birth_date, gender, active).
-->
<script lang="ts">
    import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
    import {
        FilterBar,
        Willow as FilterTheme,
        createArrayFilter,
    } from "@svar-ui/svelte-filter";
    import type { Worker } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    let {
        workers,
        onselect,
    }: {
        workers: Worker[];
        onselect?: (worker: Worker) => void;
    } = $props();

    // Column definitions for the SVAR grid (id + fixed pixel widths).
    // Headers are translated reactively so a locale switch relabels them.
    const columns = $derived([
        { id: "id", header: t("grid.id"), width: 280 },
        { id: "family", header: t("grid.family"), width: 160 },
        { id: "given", header: t("grid.given"), width: 200 },
        { id: "birth_date", header: t("grid.dob"), width: 120 },
        { id: "gender", header: t("grid.gender"), width: 90 },
        { id: "active", header: t("grid.active"), width: 80 },
    ]);

    // Flatten nested Worker fields into primitive grid cells; `id` must be a
    // non-empty string so initGrid can map a selected row back to a Worker.
    const data = $derived(
        workers.map((p) => ({
            id: p.id ?? "",
            family: p.name.family,
            given: p.name.given.join(" "),
            birth_date: p.birth_date ?? "",
            gender: p.gender,
            // active defaults to true when unset; render as yes/no for display.
            active: (p.active ?? true) ? t("grid.yes") : t("grid.no"),
        })),
    );

    // Grid init hook: subscribe to row selection and resolve the row id back
    // to the originating Worker before invoking the parent callback.

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

    function initGrid(api: {
        on(action: string, cb: (ev: { id: string | number }) => void): void;
    }) {
        api.on("select-row", (ev) => {
            const found = workers.find((p) => p.id === String(ev.id));
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
