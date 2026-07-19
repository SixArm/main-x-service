<!--
  CourseGrid — tabular course list backed by the SVAR (@svar-ui/svelte-grid)
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
    import { Grid, Willow as GridTheme } from "@svar-ui/svelte-grid";
    import {
        FilterBar,
        Willow as FilterTheme,
        createArrayFilter,
    } from "@svar-ui/svelte-filter";
    import type { Course } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    let {
        courses,
        onselect,
    }: {
        courses: Course[];
        onselect?: (course: Course) => void;
    } = $props();

    // Column definitions (id ⇒ row key, localized header label, pixel width).
    // Headers are derived so a locale switch relabels the grid columns.
    const columns = $derived([
        { id: "id", header: t("grid.id"), width: 220 },
        { id: "name", header: t("grid.name"), width: 240 },
        { id: "course_code", header: t("grid.courseCode"), width: 120 },
        { id: "educational_level", header: t("grid.level"), width: 140 },
        { id: "status", header: t("grid.status"), width: 100 },
        { id: "primary_id", header: t("grid.primaryIdentifier"), width: 200 },
    ]);

    // Render the first identifier as "Type value" for the grid cell;
    // handles the { Custom } object variant of property_id.
    function primaryIdentifier(c: Course): string {
        if (!c.identifiers || c.identifiers.length === 0) return "";
        const first = c.identifiers[0];
        if (!first) return "";
        const type = typeof first.property_id === "string" ? first.property_id : `${t("detail.customPrefix")}${first.property_id.Custom}`;
        return `${type} ${first.value}`;
    }

    // Flatten educational_level (string or { Custom }) to a display string.
    function levelLabel(c: Course): string {
        const l = c.educational_level;
        if (!l) return "";
        return typeof l === "string" ? l : `${t("detail.customPrefix")} ${l.Custom}`;
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
            const found = courses.find((c) => c.id === String(ev.id));
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
