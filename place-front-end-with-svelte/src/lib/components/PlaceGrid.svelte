<script lang="ts">
    import { Grid } from "wx-svelte-grid";
    import type { Place } from "$lib/api/types.js";

    let {
        places,
        onselect,
    }: {
        places: Place[];
        onselect?: (place: Place) => void;
    } = $props();

    const columns = [
        { id: "id", header: "ID", width: 220 },
        { id: "name", header: "Name", width: 220 },
        { id: "place_type", header: "Type", width: 140 },
        { id: "locality", header: "City", width: 160 },
        { id: "country", header: "Country", width: 80 },
        { id: "geo", header: "Lat / Lon", width: 160 },
    ];

    function placeTypeLabel(p: Place): string {
        if (!p.place_type) return "";
        if (typeof p.place_type === "string") return p.place_type;
        return `Other: ${p.place_type.Other}`;
    }

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

    function initGrid(api: { on(action: string, cb: (ev: { id: string | number }) => void): void }) {
        api.on("select-row", (ev) => {
            const found = places.find((p) => p.id === String(ev.id));
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
