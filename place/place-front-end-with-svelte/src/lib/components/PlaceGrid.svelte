<!--
  PlaceGrid — tabular list of places rendered with the SVAR (@svar-ui/svelte-grid)
  DataGrid. Flattens each Place into the grid's row shape and re-emits a
  row selection as an `onselect(place)` callback with the full Place.

  $props:
    - places (Place[]) — records to display.
    - onselect ((place) => void) — fired when a row is selected.

  Derived:
    - data — `places` projected into flat grid rows (reactive).
-->
<script lang="ts">
    import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
    import {
        FilterBar,
        Willow as FilterTheme,
        createArrayFilter,
    } from "@svar-ui/svelte-filter";
    import type { Place } from "$lib/api/types.js";
    import { t, translate } from "$lib/i18n.svelte.js";

    let {
        places,
        onselect,
    }: {
        places: Place[];
        onselect?: (place: Place) => void;
    } = $props();

    // Column definitions (id keys must match the row shape below). Headers
    // are reactive so a locale switch relabels the grid columns.
    const columns = $derived([
        { id: "id", header: t("grid.id"), width: 220 },
        { id: "name", header: t("grid.name"), width: 220 },
        { id: "place_type", header: t("grid.type"), width: 140 },
        { id: "locality", header: t("grid.city"), width: 160 },
        { id: "country", header: t("grid.country"), width: 80 },
        { id: "geo", header: t("grid.latLon"), width: 160 },
    ]);

    // Render a PlaceType for the grid cell; `{ Other }` → "Other: <value>".
    function placeTypeLabel(p: Place): string {
        if (!p.place_type) return "";
        if (typeof p.place_type === "string") return p.place_type;
        return translate("grid.typeOther").replace("{value}", p.place_type.Other);
    }

    // Flatten nested Place fields (address, geo) into the flat columns the
    // grid expects; recomputes whenever `places` changes.
    const data = $derived(
        places.map((p) => ({
            id: p.id ?? "",
            name: p.name,
            place_type: placeTypeLabel(p),
            locality: p.address?.address_locality ?? "",
            country: p.address?.address_country ?? "",
            geo: p.geo ? `${p.geo.latitude.toFixed(4)}, ${p.geo.longitude.toFixed(4)}` : "",
        })),
    );

    // Wire the grid's "select-row" event back to the original Place. The
    // grid yields the row id (string|number); match it against places.

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
            const found = places.find((p) => p.id === String(ev.id));
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
