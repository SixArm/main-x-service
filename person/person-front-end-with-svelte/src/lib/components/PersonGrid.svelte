<!--
  PersonGrid — tabular person list backed by the SVAR (wx-svelte-grid)
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
    import { Grid } from "wx-svelte-grid";
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

    // SVAR exposes events through an IApi reference. Subscribe in
    // `init` and look up the originating Person by id.
    function initGrid(api: { on(action: string, cb: (ev: { id: string | number }) => void): void }) {
        api.on("select-row", (ev) => {
            const found = persons.find((p) => p.id === String(ev.id));
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
