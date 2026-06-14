<!--
  ThingGrid — tabular list of Things backed by the SVAR (wx-svelte-grid)
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
    import { Grid } from "wx-svelte-grid";
    import type { Thing } from "$lib/api/types.js";

    let {
        things,
        onselect,
    }: {
        things: Thing[];
        onselect?: (thing: Thing) => void;
    } = $props();

    // Static grid column definitions (id, header, pixel width).
    const columns = [
        { id: "id", header: "ID", width: 220 },
        { id: "name", header: "Name", width: 240 },
        { id: "additional_type", header: "Type (schema.org)", width: 200 },
        { id: "primary_id", header: "Primary identifier", width: 200 },
        { id: "url", header: "URL", width: 200 },
    ];

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
    function initGrid(api: { on(action: string, cb: (ev: { id: string | number }) => void): void }) {
        api.on("select-row", (ev) => {
            const found = things.find((t) => t.id === String(ev.id));
            if (found) onselect?.(found);
        });
    }
</script>

<div class="grid-wrap">
    <Grid {data} {columns} select init={initGrid} />
</div>

<style>
    .grid-wrap {
        height: 480px;
        border: 1px solid var(--mxi-color-border);
        border-radius: var(--mxi-radius);
        overflow: hidden;
    }
</style>
