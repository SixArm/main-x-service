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

    const columns = [
        { id: "id", header: "ID", width: 220 },
        { id: "name", header: "Name", width: 240 },
        { id: "additional_type", header: "Type (schema.org)", width: 200 },
        { id: "primary_id", header: "Primary identifier", width: 200 },
        { id: "url", header: "URL", width: 200 },
    ];

    function primaryIdentifier(t: Thing): string {
        if (!t.identifiers || t.identifiers.length === 0) return "";
        const first = t.identifiers[0];
        if (!first) return "";
        const type = typeof first.property_id === "string" ? first.property_id : `Custom:${first.property_id.Custom}`;
        return `${type} ${first.value}`;
    }

    const data = $derived(
        things.map((t) => ({
            id: t.id ?? "",
            name: t.name,
            additional_type: t.additional_type ?? "",
            primary_id: primaryIdentifier(t),
            url: t.url ?? "",
        })),
    );

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
