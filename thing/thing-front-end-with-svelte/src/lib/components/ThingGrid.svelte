<!--
  ThingGrid — tabular list of Things backed by the SVAR (@svar-ui/svelte-grid)
  DataGrid.

  Purpose: shows id / name / schema.org type / primary identifier / URL and
  emits the selected Thing when a row is chosen, used by the Things list page.

  $props:
    - things (Thing[]): the records to display.
    - onselect ((thing: Thing) => void, optional): row-select callback.

  Reactive notes: `data` is $derived — it projects each Thing into the flat
  row shape the grid expects, recomputing when `things` changes. Row identity
  is the Thing's id, used to map a grid selection back to the source object.
-->
<script lang="ts">
    import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
    import {
        FilterBar,
        Willow as FilterTheme,
        createArrayFilter,
    } from "@svar-ui/svelte-filter";
    import type { Thing } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    let {
        things,
        onselect,
    }: {
        things: Thing[];
        onselect?: (thing: Thing) => void;
    } = $props();

    // Grid column definitions (id, localized header, pixel width). $derived
    // so headers re-render when the UI locale changes.
    const columns = $derived([
        { id: "id", header: t("grid.id"), width: 220 },
        { id: "name", header: t("grid.name"), width: 240 },
        { id: "additional_type", header: t("grid.type"), width: 200 },
        { id: "primary_id", header: t("grid.primaryId"), width: 200 },
        { id: "url", header: t("grid.url"), width: 200 },
    ]);

    // Format the first identifier as "<scheme> <value>" for the grid cell,
    // handling both bare-string schemes and the Custom tagged variant.
    function primaryIdentifier(t: Thing): string {
        if (!t.identifiers || t.identifiers.length === 0) return "";
        const first = t.identifiers[0];
        if (!first) return "";
        const type = typeof first.property_id === "string" ? first.property_id : `Custom:${first.property_id.Custom}`;
        return `${type} ${first.value}`;
    }

    // Flatten Things into the grid's row records; nullish fields become "".
    const data = $derived(
        things.map((t) => ({
            id: t.id ?? "",
            name: t.name,
            additional_type: t.additional_type ?? "",
            primary_id: primaryIdentifier(t),
            url: t.url ?? "",
        })),
    );

    // Wire the grid's "select-row" event back to the original Thing object
    // (the grid only knows the row id) and forward it to the parent.

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

    function initGrid(api: { on(action: string, cb: (ev: { id: string | number }) => void): void }) {
        api.on("select-row", (ev) => {
            const found = things.find((t) => t.id === String(ev.id));
            if (found) onselect?.(found);
        });
    }
</script>

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
