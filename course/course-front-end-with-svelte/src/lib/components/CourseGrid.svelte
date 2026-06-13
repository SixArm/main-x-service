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

    const columns = [
        { id: "id", header: "ID", width: 220 },
        { id: "name", header: "Name", width: 240 },
        { id: "course_code", header: "Course code", width: 120 },
        { id: "educational_level", header: "Level", width: 140 },
        { id: "status", header: "Status", width: 100 },
        { id: "primary_id", header: "Primary identifier", width: 200 },
    ];

    function primaryIdentifier(c: Course): string {
        if (!c.identifiers || c.identifiers.length === 0) return "";
        const first = c.identifiers[0];
        if (!first) return "";
        const type = typeof first.property_id === "string" ? first.property_id : `Custom:${first.property_id.Custom}`;
        return `${type} ${first.value}`;
    }

    function levelLabel(c: Course): string {
        const l = c.educational_level;
        if (!l) return "";
        return typeof l === "string" ? l : `Custom: ${l.Custom}`;
    }

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
