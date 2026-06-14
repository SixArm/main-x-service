<!--
  CourseGrid — tabular course list backed by the SVAR (wx-svelte-grid)
  DataGrid. Flattens each Course into a row of primitive cells and
  surfaces row selection back to the parent as the full Course object.

  $props:
    - courses: Course[] — rows to display.
    - onselect?: (course) => void — invoked when a row is selected.

  Reactive state:
    - data ($derived) — courses projected to flat grid rows; recomputes
      whenever `courses` changes.
-->
<script lang="ts">
    import { Grid } from "wx-svelte-grid";
    import type { Course } from "$lib/api/types.js";

    let {
        courses,
        onselect,
    }: {
        courses: Course[];
        onselect?: (course: Course) => void;
    } = $props();

    // Static column definitions (id ⇒ row key, header label, pixel width).
    const columns = [
        { id: "id", header: "ID", width: 220 },
        { id: "name", header: "Name", width: 240 },
        { id: "course_code", header: "Course code", width: 120 },
        { id: "educational_level", header: "Level", width: 140 },
        { id: "status", header: "Status", width: 100 },
        { id: "primary_id", header: "Primary identifier", width: 200 },
    ];

    // Render the first identifier as "Type value" for the grid cell;
    // handles the { Custom } object variant of property_id.
    function primaryIdentifier(c: Course): string {
        if (!c.identifiers || c.identifiers.length === 0) return "";
        const first = c.identifiers[0];
        if (!first) return "";
        const type = typeof first.property_id === "string" ? first.property_id : `Custom:${first.property_id.Custom}`;
        return `${type} ${first.value}`;
    }

    // Flatten educational_level (string or { Custom }) to a display string.
    function levelLabel(c: Course): string {
        const l = c.educational_level;
        if (!l) return "";
        return typeof l === "string" ? l : `Custom: ${l.Custom}`;
    }

    // Project courses into the flat row shape the grid columns expect.
    const data = $derived(
        courses.map((c) => ({
            id: c.id ?? "",
            name: c.name,
            course_code: c.course_code ?? "",
            educational_level: levelLabel(c),
            status: c.status ?? "",
            primary_id: primaryIdentifier(c),
        })),
    );

    // SVAR grid init hook: subscribe to row selection and map the row
    // id back to its source Course before notifying the parent.
    function initGrid(api: { on(action: string, cb: (ev: { id: string | number }) => void): void }) {
        api.on("select-row", (ev) => {
            const found = courses.find((c) => c.id === String(ev.id));
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
