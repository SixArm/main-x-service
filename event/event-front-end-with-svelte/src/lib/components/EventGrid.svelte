<!--
  EventGrid — tabular event list backed by the SVAR (wx-svelte-grid)
  DataGrid. Maps each Event to a flat display row, and emits the original
  Event when a row is selected.

  Props:
    - events (Event[]): events to display.
    - onselect ((event) => void, optional): fired with the full Event when a
      row is selected.

  State:
    - data ($derived): display rows recomputed whenever `events` changes.
-->
<script lang="ts">
    import { Grid } from "wx-svelte-grid";
    import { t } from "$lib/i18n.svelte.js";
    import type { Event } from "$lib/api/types.js";

    let {
        events,
        onselect,
    }: {
        events: Event[];
        onselect?: (event: Event) => void;
    } = $props();

    // Column definitions for the SVAR grid (id matches the row keys below).
    // Headers are localized; `$derived` so a locale switch re-renders them.
    const columns = $derived([
        { id: "id", header: t("grid.id"), width: 220 },
        { id: "name", header: t("grid.name"), width: 240 },
        { id: "start_date", header: t("grid.start"), width: 160 },
        { id: "event_type", header: t("grid.type"), width: 140 },
        { id: "event_status", header: t("grid.status"), width: 120 },
        { id: "attendance_mode", header: t("grid.mode"), width: 100 },
    ]);

    // Flatten/format each Event into a primitive row for the grid; nullable
    // fields collapse to "" and the start date is localized for display.
    const data = $derived(
        events.map((e) => ({
            id: e.id ?? "",
            name: e.name,
            start_date: e.start_date ? new Date(e.start_date).toLocaleString() : "",
            event_type: e.event_type ?? "",
            event_status: e.event_status ?? "",
            attendance_mode: e.event_attendance_mode ?? "",
        })),
    );

    // Grid init hook: subscribe to row selection and map the selected row id
    // back to the original Event before invoking the parent callback.
    function initGrid(api: { on(action: string, cb: (ev: { id: string | number }) => void): void }) {
        api.on("select-row", (ev) => {
            const found = events.find((e) => e.id === String(ev.id));
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
