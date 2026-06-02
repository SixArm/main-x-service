<script lang="ts">
    import { Grid } from "wx-svelte-grid";
    import type { Worker } from "$lib/api/types.js";

    let {
        workers,
        onselect,
    }: {
        workers: Worker[];
        onselect?: (worker: Worker) => void;
    } = $props();

    const columns = [
        { id: "id", header: "ID", width: 280 },
        { id: "family", header: "Family", width: 160 },
        { id: "given", header: "Given", width: 200 },
        { id: "birth_date", header: "DOB", width: 120 },
        { id: "gender", header: "Gender", width: 90 },
        { id: "active", header: "Active", width: 80 },
    ];

    const data = $derived(
        workers.map((p) => ({
            id: p.id ?? "",
            family: p.name.family,
            given: p.name.given.join(" "),
            birth_date: p.birth_date ?? "",
            gender: p.gender,
            active: (p.active ?? true) ? "yes" : "no",
        })),
    );

    function initGrid(api: { on(action: string, cb: (ev: { id: string | number }) => void): void }) {
        api.on("select-row", (ev) => {
            const found = workers.find((p) => p.id === String(ev.id));
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
