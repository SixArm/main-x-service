<!--
  WorkerGrid — tabular list of workers backed by the SVAR (wx-svelte-grid)
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
    import { Grid } from "wx-svelte-grid";
    import type { Worker } from "$lib/api/types.js";

    let {
        workers,
        onselect,
    }: {
        workers: Worker[];
        onselect?: (worker: Worker) => void;
    } = $props();

    // Column definitions for the SVAR grid (id + fixed pixel widths).
    const columns = [
        { id: "id", header: "ID", width: 280 },
        { id: "family", header: "Family", width: 160 },
        { id: "given", header: "Given", width: 200 },
        { id: "birth_date", header: "DOB", width: 120 },
        { id: "gender", header: "Gender", width: 90 },
        { id: "active", header: "Active", width: 80 },
    ];

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
            active: (p.active ?? true) ? "yes" : "no",
        })),
    );

    // Grid init hook: subscribe to row selection and resolve the row id back
    // to the originating Worker before invoking the parent callback.
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
